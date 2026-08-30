//! Dedicated OMT filter sender.

use std::borrow::Cow;
use std::ffi::CStr;

use log::{info, warn};
use obs_wrapper::data::DataObj;
use obs_wrapper::obs_string;
use obs_wrapper::obs_sys::{
    obs_audio_info, obs_filter_get_parent, obs_filter_get_target, obs_get_audio_info,
    obs_get_video_frame_time, obs_get_video_info, obs_source_enabled, obs_source_get_height,
    obs_source_get_name, obs_source_get_width, obs_source_showing, obs_source_skip_video_filter,
    obs_source_t, obs_source_video_render, obs_video_info, speaker_layout,
    speaker_layout_SPEAKERS_2POINT1, speaker_layout_SPEAKERS_4POINT0,
    speaker_layout_SPEAKERS_4POINT1, speaker_layout_SPEAKERS_5POINT1,
    speaker_layout_SPEAKERS_7POINT1, speaker_layout_SPEAKERS_MONO,
};
use obs_wrapper::prelude::*;
use obs_wrapper::properties::{Properties, TextProp, TextType};
use obs_wrapper::source::*;
use obs_wrapper::wrapper::PtrWrapper;
use openmediatransport::{Codec, ColorSpace, FrameType, MediaFrame, VideoFlags};

use crate::clock::obs_ns_to_omt_ticks;
use crate::graphics_capture::BgraCapture;
use crate::ids::{DEFAULT_FILTER_NAME, PROP_FILTER_MODE, PROP_FILTER_NAME};
use crate::media_mode::MediaMode;
use crate::send_session::SendSession;

pub struct OmtSendFilter {
    source: SourceRef,
    name_template: String,
    mode: MediaMode,
    session: Option<SendSession>,
    capture: Option<BgraCapture>,
    rendered: bool,
    fps_num: u32,
    fps_den: u32,
    sample_rate: u32,
    channels: u32,
}

impl OmtSendFilter {
    fn parent_ptr(&self) -> *mut obs_source_t {
        unsafe { obs_filter_get_parent(self.source.as_ptr()) }
    }

    fn source_name(ptr: *mut obs_source_t) -> Option<String> {
        if ptr.is_null() {
            return None;
        }
        let c = unsafe { obs_source_get_name(ptr) };
        if c.is_null() {
            return None;
        }
        let s = unsafe { CStr::from_ptr(c) }.to_string_lossy();
        let s = s.trim();
        if s.is_empty() {
            None
        } else {
            Some(s.to_string())
        }
    }

    fn resolved_name(&self) -> String {
        let mut name = self.name_template.clone();
        if name.contains("${source}") {
            if let Some(n) = Self::source_name(self.parent_ptr()) {
                name = name.replace("${source}", &n);
            }
        }
        if name.contains("${filter}") {
            if let Some(n) = Self::source_name(unsafe { self.source.as_ptr() } as *mut obs_source_t)
            {
                name = name.replace("${filter}", &n);
            }
        }
        let name = name.trim();
        if name.is_empty() || name.contains("${source}") || name.contains("${filter}") {
            DEFAULT_FILTER_NAME
                .replace("${source}", "OMT Filter")
                .replace("${filter}", "OMT Filter")
        } else {
            name.to_string()
        }
    }

    fn ensure_session(&mut self) {
        if self.parent_ptr().is_null() {
            if self.session.is_some() {
                info!("OMT Dedicated Output detached; stopping sender");
            }
            self.session = None;
            return;
        }
        let name = self.resolved_name();
        let types = self.mode.frame_types();
        let matches = self
            .session
            .as_ref()
            .is_some_and(|s| s.name() == name && s.frame_types() == types);
        if matches {
            return;
        }
        self.session = None;
        match SendSession::start(name, types) {
            Ok(s) => {
                info!("OMT Dedicated Output sender active as '{}'", s.name());
                self.session = Some(s);
            }
            Err(e) => warn!("OMT filter sender failed: {e}"),
        }
    }

    fn read_settings(&mut self, settings: &mut DataObj) {
        if let Some(n) = settings.get::<Cow<str>>(ObsString::from(PROP_FILTER_NAME)) {
            let s = n.trim();
            if !s.is_empty() {
                self.name_template = s.to_string();
            }
        }
        if let Some(v) = settings.get::<i64>(ObsString::from(PROP_FILTER_MODE)) {
            self.mode = MediaMode::from_i64(v);
        }
    }
}

impl Sourceable for OmtSendFilter {
    fn get_id() -> ObsString {
        crate::ids::filter_id()
    }
    fn get_type() -> SourceType {
        SourceType::Filter
    }
    fn create(create: &mut CreatableSourceContext<Self>, source: SourceRef) -> Self {
        let mut ovi = unsafe { std::mem::zeroed::<obs_video_info>() };
        let mut oai = unsafe { std::mem::zeroed::<obs_audio_info>() };
        unsafe {
            obs_get_video_info(&mut ovi);
            obs_get_audio_info(&mut oai);
        }
        let channels = speaker_channels(oai.speakers);
        let mut this = Self {
            source,
            name_template: DEFAULT_FILTER_NAME.to_string(),
            mode: MediaMode::Embedded,
            session: None,
            capture: Some(BgraCapture::new()),
            rendered: false,
            fps_num: ovi.fps_num.max(1),
            fps_den: ovi.fps_den.max(1),
            sample_rate: oai.samples_per_sec.max(1),
            channels,
        };
        this.read_settings(&mut create.settings);
        this
    }
}

impl GetNameSource for OmtSendFilter {
    fn get_name() -> ObsString {
        obs_string!("OMT Dedicated Output (Rust)")
    }
}

impl GetDefaultsSource for OmtSendFilter {
    fn get_defaults(settings: &mut DataObj) {
        settings.set_default::<obs_wrapper::string::ObsString>(
            ObsString::from(PROP_FILTER_NAME),
            ObsString::from(DEFAULT_FILTER_NAME),
        );
        settings.set_default::<i64>(ObsString::from(PROP_FILTER_MODE), 0);
        crate::ids::apply_official_plugin_note_default(settings);
    }
}

impl GetPropertiesSource for OmtSendFilter {
    fn get_properties(&mut self) -> Properties {
        let mut props = Properties::new();
        props.add(
            ObsString::from(PROP_FILTER_NAME),
            obs_string!("OMT Source Name (${source} / ${filter})"),
            TextProp::new(TextType::Default),
        );
        crate::media_mode::add_media_mode_list(&mut props, PROP_FILTER_MODE, obs_string!("Media"));
        crate::ids::add_official_plugin_note(&mut props);
        props
    }
}

impl UpdateSource for OmtSendFilter {
    fn update(&mut self, settings: &mut DataObj, _context: &mut GlobalContext) {
        self.read_settings(settings);
        self.ensure_session();
    }
}

impl FilterAddSource for OmtSendFilter {
    fn filter_add(&mut self, _source: SourceRef) {
        info!("OMT Dedicated Output attached");
        self.ensure_session();
    }
}

impl FilterRemoveSource for OmtSendFilter {
    fn filter_remove(&mut self, _source: SourceRef) {
        info!("OMT Dedicated Output removed");
        self.session = None;
    }
}

impl VideoTickSource for OmtSendFilter {
    fn video_tick(&mut self, _seconds: f32) {
        self.rendered = false;
        self.ensure_session();
    }
}

impl VideoRenderSource for OmtSendFilter {
    fn video_render(&mut self, _context: &mut GlobalContext, _render: &mut VideoRenderContext) {
        let parent = unsafe { obs_filter_get_parent(self.source.as_ptr()) };
        let target = unsafe { obs_filter_get_target(self.source.as_ptr()) };
        if parent.is_null() || target.is_null() {
            unsafe {
                obs_source_skip_video_filter(self.source.as_ptr() as *mut obs_source_t);
            }
            return;
        }
        unsafe {
            obs_source_skip_video_filter(self.source.as_ptr() as *mut obs_source_t);
        }
        if !self.mode.video() || self.rendered {
            return;
        }
        self.rendered = true;
        let filter_ptr = unsafe { self.source.as_ptr() } as *mut obs_source_t;
        if !unsafe { obs_source_enabled(filter_ptr) && obs_source_showing(parent) } {
            return;
        }
        let (width, height) = unsafe {
            (
                obs_source_get_width(filter_ptr),
                obs_source_get_height(filter_ptr),
            )
        };
        if width == 0 || height == 0 {
            return;
        }
        let pixels = self.capture.as_mut().and_then(|capture| {
            capture.capture_with(width, height, || unsafe {
                if target == parent {
                    obs_source_skip_video_filter(filter_ptr);
                } else {
                    obs_source_video_render(target);
                }
            })
        });
        if let (Some(pixels), Some(session)) = (pixels, self.session.as_ref()) {
            let ts = obs_ns_to_omt_ticks(unsafe { obs_get_video_frame_time() });
            session.push_video(MediaFrame {
                frame_type: FrameType::VIDEO,
                timestamp: ts,
                codec: Codec::Bgra as i32,
                width: width as i32,
                height: height as i32,
                stride: (width * 4) as i32,
                // TODO: Preserve alpha once the OMT VMX receiver supports
                // alpha/high-bit-depth video. OBS has already composited this
                // SDR surface, so it is currently encoded as opaque BGRX.
                flags: VideoFlags::NONE,
                frame_rate_n: self.fps_num as i32,
                frame_rate_d: self.fps_den as i32,
                aspect_ratio: width as f32 / height.max(1) as f32,
                color_space: ColorSpace::Undefined,
                data: pixels,
                ..MediaFrame::default()
            });
        }
    }
}

impl FilterAudioSource for OmtSendFilter {
    fn filter_audio(&mut self, audio: &mut obs_wrapper::media::AudioDataContext) {
        if !self.mode.audio() {
            return;
        }
        let Some(session) = self.session.as_ref() else {
            return;
        };
        let frames = audio.frames();
        if frames == 0 {
            return;
        }
        let channels = (self.channels as usize).clamp(1, 8);
        let mut data = vec![0u8; channels * frames * 4];
        let mut used = 0usize;
        for c in 0..channels {
            if let Some(slice) = audio.get_channel_as_mut_slice(c) {
                if slice.is_empty() {
                    continue;
                }
                let bytes = bytemuck_f32(slice);
                let dst = c * frames * 4;
                if dst + bytes.len() <= data.len() {
                    data[dst..dst + bytes.len()].copy_from_slice(bytes);
                    used = c + 1;
                }
            }
        }
        if used == 0 {
            return;
        }
        session.push_audio(MediaFrame {
            frame_type: FrameType::AUDIO,
            timestamp: obs_ns_to_omt_ticks(audio.timestamp()),
            codec: Codec::Fpa1 as i32,
            sample_rate: self.sample_rate as i32,
            channels: used as i32,
            samples_per_channel: frames as i32,
            data,
            ..MediaFrame::default()
        });
    }
}

fn speaker_channels(layout: speaker_layout) -> u32 {
    if layout == speaker_layout_SPEAKERS_MONO {
        1
    } else if layout == speaker_layout_SPEAKERS_2POINT1 {
        3
    } else if layout == speaker_layout_SPEAKERS_4POINT0 {
        4
    } else if layout == speaker_layout_SPEAKERS_4POINT1 {
        5
    } else if layout == speaker_layout_SPEAKERS_5POINT1 {
        6
    } else if layout == speaker_layout_SPEAKERS_7POINT1 {
        8
    } else {
        2
    }
}

fn bytemuck_f32(slice: &[f32]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(slice.as_ptr() as *const u8, slice.len() * 4) }
}

impl Drop for OmtSendFilter {
    fn drop(&mut self) {
        info!("OMT Dedicated Output destroyed");
        self.session = None;
        if let Some(mut cap) = self.capture.take() {
            cap.destroy();
        }
    }
}
