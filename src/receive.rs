//! OMT receive source (`omtsource`).

use std::borrow::Cow;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use log::{info, warn};
use obs_wrapper::data::DataObj;
use obs_wrapper::obs_string;
use obs_wrapper::obs_sys::{
    audio_format_AUDIO_FORMAT_FLOAT_PLANAR, obs_source_audio, obs_source_frame,
    obs_source_output_audio, obs_source_output_video, obs_source_t, speaker_layout_SPEAKERS_MONO,
    speaker_layout_SPEAKERS_STEREO, video_format_VIDEO_FORMAT_BGRA,
};
use obs_wrapper::prelude::*;
use obs_wrapper::properties::{ListProp, Properties};
use obs_wrapper::source::*;
use obs_wrapper::wrapper::PtrWrapper;
use openmediatransport::{
    ColorSpace, Discovery, FrameType, Quality, ReceiverConfig, ReceiverSession, Tally,
};

use crate::bandwidth::{add_bandwidth_policy_list, BandwidthPolicy};
use crate::clock::omt_ticks_to_obs_ns;
use crate::ids::{
    PROP_BANDWIDTH_POLICY, PROP_COLOR_SPACE, PROP_PREVIEW, PROP_QUALITY, PROP_SOURCE,
};

static DISCOVERY: Mutex<Option<DiscoveryCache>> = Mutex::new(None);

struct DiscoveryCache {
    discovery: Discovery,
    last: Instant,
}

fn listed_sources() -> Vec<String> {
    let mut guard = DISCOVERY.lock().unwrap_or_else(|e| e.into_inner());
    let cache = guard.get_or_insert_with(|| DiscoveryCache {
        discovery: Discovery::new().unwrap_or_else(|_| Discovery::default()),
        last: Instant::now() - Duration::from_secs(10),
    });
    if cache.last.elapsed() > Duration::from_millis(500) {
        if let Err(e) = cache.discovery.refresh_for(Duration::from_millis(400)) {
            warn!("OMT discovery refresh failed: {e}");
        }
        cache.last = Instant::now();
    }
    cache
        .discovery
        .sources()
        .iter()
        .map(|s| s.to_url())
        .collect()
}

fn quality_from_i64(v: i64) -> Quality {
    match v {
        1 => Quality::Low,
        50 => Quality::Medium,
        100 => Quality::High,
        _ => Quality::Default,
    }
}

fn color_space_from_i64(v: i64) -> ColorSpace {
    match v {
        601 => ColorSpace::Bt601,
        709 => ColorSpace::Bt709,
        _ => ColorSpace::Undefined,
    }
}

fn policy_from_settings(settings: &DataObj<'_>) -> BandwidthPolicy {
    BandwidthPolicy::from_settings(
        settings.get::<i64>(ObsString::from(PROP_BANDWIDTH_POLICY)),
        settings
            .get::<bool>(ObsString::from(PROP_PREVIEW))
            .unwrap_or(false),
    )
}

pub struct OmtReceiveSource {
    source: SourceRef,
    address: String,
    quality: Quality,
    policy: BandwidthPolicy,
    color_space: ColorSpace,
    desired_preview: Arc<AtomicBool>,
    tally_preview: Arc<AtomicBool>,
    tally_program: Arc<AtomicBool>,
    stop: Option<Arc<AtomicBool>>,
    join: Option<JoinHandle<()>>,
}

impl OmtReceiveSource {
    fn stop_worker(&mut self) {
        if let Some(stop) = self.stop.take() {
            stop.store(true, Ordering::Release);
        }
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }

    fn persist_policy(&self, settings: &mut DataObj<'_>) {
        settings.set_int(ObsString::from(PROP_BANDWIDTH_POLICY), self.policy.as_i64());
        settings.set_bool(
            ObsString::from(PROP_PREVIEW),
            self.policy == BandwidthPolicy::Always,
        );
    }

    fn refresh_visibility(&mut self) {
        let active = self.source.active();
        let showing = self.source.showing();
        self.desired_preview
            .store(self.policy.use_preview(active, showing), Ordering::Release);
        self.tally_preview
            .store(showing && !active, Ordering::Release);
        self.tally_program.store(active, Ordering::Release);
    }

    fn apply_settings(&mut self, settings: &mut DataObj<'_>) {
        let address = settings
            .get::<Cow<str>>(ObsString::from(PROP_SOURCE))
            .unwrap_or(Cow::Borrowed(""))
            .trim()
            .to_string();
        let quality = quality_from_i64(
            settings
                .get::<i64>(ObsString::from(PROP_QUALITY))
                .unwrap_or(0),
        );
        let policy = policy_from_settings(settings);
        let color_space = color_space_from_i64(
            settings
                .get::<i64>(ObsString::from(PROP_COLOR_SPACE))
                .unwrap_or(0),
        );

        let reconnect =
            address != self.address || quality != self.quality || color_space != self.color_space;
        self.address = address;
        self.quality = quality;
        self.policy = policy;
        self.color_space = color_space;
        self.persist_policy(settings);
        self.refresh_visibility();
        if reconnect {
            self.restart();
        }
    }

    fn restart(&mut self) {
        self.stop_worker();
        if self.address.is_empty() {
            return;
        }
        let stop = Arc::new(AtomicBool::new(false));
        let address = self.address.clone();
        let quality = self.quality;
        let preview = self.desired_preview.load(Ordering::Acquire);
        let ctrl = ReceiveCtrl {
            desired_preview: Arc::clone(&self.desired_preview),
            tally_preview: Arc::clone(&self.tally_preview),
            tally_program: Arc::clone(&self.tally_program),
            stop: Arc::clone(&stop),
        };
        let source_ptr = unsafe { self.source.as_ptr() as usize };
        match thread::Builder::new()
            .name("omt-rx".into())
            .spawn(move || receive_loop(address, quality, preview, ctrl, source_ptr))
        {
            Ok(join) => {
                self.stop = Some(stop);
                self.join = Some(join);
            }
            Err(e) => warn!("failed to start OMT receive thread: {e}"),
        }
    }
}

impl Drop for OmtReceiveSource {
    fn drop(&mut self) {
        self.stop_worker();
    }
}

struct ReceiveCtrl {
    desired_preview: Arc<AtomicBool>,
    tally_preview: Arc<AtomicBool>,
    tally_program: Arc<AtomicBool>,
    stop: Arc<AtomicBool>,
}

fn receive_loop(
    address: String,
    quality: Quality,
    preview: bool,
    ctrl: ReceiveCtrl,
    source_ptr: usize,
) {
    let config = ReceiverConfig {
        frame_types: FrameType::VIDEO | FrameType::AUDIO,
        quality,
        preview,
        connect_timeout: Duration::from_secs(5),
        auto_reconnect: true,
    };
    info!("OMT receive connecting to {address} preview={preview}");
    let session = match ReceiverSession::connect(&address, config) {
        Ok(s) => s,
        Err(e) => {
            warn!("OMT receive connect failed: {e}");
            return;
        }
    };
    let source = source_ptr as *mut obs_source_t;
    if source.is_null() {
        return;
    }
    let mut preview = preview;
    let mut last_tally_preview = ctrl.tally_preview.load(Ordering::Acquire);
    let mut last_tally_program = ctrl.tally_program.load(Ordering::Acquire);
    let send_tally = |preview: bool, program: bool| {
        if let Err(e) = session.set_tally(Tally::new(i32::from(preview), i32::from(program))) {
            warn!("OMT tally send failed: {e}");
        }
    };
    send_tally(last_tally_preview, last_tally_program);
    while !ctrl.stop.load(Ordering::Acquire) {
        let want_preview = ctrl.desired_preview.load(Ordering::Acquire);
        if want_preview != preview {
            match session.set_preview(want_preview) {
                Ok(()) => {
                    info!("OMT receive preview={want_preview}");
                    preview = want_preview;
                }
                Err(e) => warn!("OMT set_preview failed: {e}"),
            }
        }
        let tp = ctrl.tally_preview.load(Ordering::Acquire);
        let tg = ctrl.tally_program.load(Ordering::Acquire);
        if tp != last_tally_preview || tg != last_tally_program {
            send_tally(tp, tg);
            last_tally_preview = tp;
            last_tally_program = tg;
        }
        if let Some(video) = session.recv_video_timeout(Duration::from_millis(50)) {
            let mut frame = obs_source_frame {
                width: video.width,
                height: video.height,
                timestamp: omt_ticks_to_obs_ns(video.timestamp),
                format: video_format_VIDEO_FORMAT_BGRA,
                full_range: true,
                ..Default::default()
            };
            frame.linesize[0] = video.stride;
            frame.data[0] = video.pixels.as_ptr() as *mut u8;
            unsafe { obs_source_output_video(source, &frame) };
        }
        while let Some(audio) = session.try_recv_audio() {
            let ch = audio.channels.max(1) as usize;
            let samples = audio.samples_per_channel.max(0) as usize;
            let mut out = obs_source_audio {
                frames: audio.samples_per_channel as u32,
                format: audio_format_AUDIO_FORMAT_FLOAT_PLANAR,
                speakers: if ch >= 2 {
                    speaker_layout_SPEAKERS_STEREO
                } else {
                    speaker_layout_SPEAKERS_MONO
                },
                samples_per_sec: audio.sample_rate.max(1) as u32,
                timestamp: omt_ticks_to_obs_ns(audio.timestamp),
                ..Default::default()
            };
            let pcm = audio.pcm_planar_f32.as_ref();
            for c in 0..ch.min(8) {
                let off = c * samples * 4;
                if off < pcm.len() {
                    out.data[c] = pcm[off..].as_ptr();
                }
            }
            unsafe { obs_source_output_audio(source, &out) };
        }
    }
    session.disconnect();
}

impl Sourceable for OmtReceiveSource {
    fn get_id() -> ObsString {
        crate::ids::source_id()
    }

    fn get_type() -> SourceType {
        SourceType::Input
    }

    fn create(create: &mut CreatableSourceContext<Self>, source: SourceRef) -> Self {
        let mut this = Self {
            source,
            address: String::new(),
            quality: Quality::Default,
            policy: BandwidthPolicy::None,
            color_space: ColorSpace::Undefined,
            desired_preview: Arc::new(AtomicBool::new(false)),
            tally_preview: Arc::new(AtomicBool::new(false)),
            tally_program: Arc::new(AtomicBool::new(false)),
            stop: None,
            join: None,
        };
        this.apply_settings(&mut create.settings);
        this
    }
}

impl GetNameSource for OmtReceiveSource {
    fn get_name() -> ObsString {
        obs_string!("OMT Source")
    }
}

impl GetDefaultsSource for OmtReceiveSource {
    fn get_defaults(settings: &mut DataObj) {
        settings.set_default::<obs_wrapper::string::ObsString>(
            ObsString::from(PROP_SOURCE),
            obs_string!(""),
        );
        settings.set_default::<i64>(ObsString::from(PROP_QUALITY), 0);
        settings.set_default::<i64>(ObsString::from(PROP_COLOR_SPACE), 0);
        settings.set_default::<bool>(ObsString::from(PROP_PREVIEW), false);
        settings.set_default::<i64>(ObsString::from(PROP_BANDWIDTH_POLICY), 0);
    }
}

impl UpdateSource for OmtReceiveSource {
    fn update(&mut self, settings: &mut DataObj, _context: &mut GlobalContext) {
        self.apply_settings(settings);
    }
}

impl SaveSource for OmtReceiveSource {
    fn save(&mut self, settings: &mut DataObj) {
        self.persist_policy(settings);
    }
}

impl ActivateSource for OmtReceiveSource {
    fn activate(&mut self) {
        self.refresh_visibility();
    }
}

impl DeactivateSource for OmtReceiveSource {
    fn deactivate(&mut self) {
        self.refresh_visibility();
    }
}

impl ShowSource for OmtReceiveSource {
    fn show(&mut self) {
        self.refresh_visibility();
    }
}

impl HideSource for OmtReceiveSource {
    fn hide(&mut self) {
        self.refresh_visibility();
    }
}

impl GetPropertiesSource for OmtReceiveSource {
    fn get_properties(&mut self) -> Properties {
        let mut props = Properties::new();
        {
            let mut list: ListProp<ObsString> =
                props.add_list(ObsString::from(PROP_SOURCE), obs_string!("Source"), true);
            list.push(obs_string!(""), obs_string!(""));
            for src in listed_sources() {
                let label = ObsString::from(src.as_str());
                list.push(label.clone(), label);
            }
        }
        {
            let mut q: ListProp<i64> = props.add_list(
                ObsString::from(PROP_QUALITY),
                obs_string!("Suggested Quality"),
                false,
            );
            q.push(obs_string!("Default"), 0);
            q.push(obs_string!("Low"), 1);
            q.push(obs_string!("Medium"), 50);
            q.push(obs_string!("High"), 100);
        }
        {
            let mut cs: ListProp<i64> = props.add_list(
                ObsString::from(PROP_COLOR_SPACE),
                obs_string!("Color Space"),
                false,
            );
            cs.push(obs_string!("Default"), 0);
            cs.push(obs_string!("BT601"), 601);
            cs.push(obs_string!("BT709"), 709);
        }
        add_bandwidth_policy_list(&mut props, PROP_BANDWIDTH_POLICY);
        props
    }
}
