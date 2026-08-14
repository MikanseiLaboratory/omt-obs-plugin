//! Dedicated OMT filter sender.

use log::warn;
use obs_wrapper::data::DataObj;
use obs_wrapper::obs_string;
use obs_wrapper::obs_sys::{
    obs_audio_info, obs_filter_get_parent, obs_filter_get_target, obs_get_audio_info,
    obs_get_video_frame_time, obs_get_video_info, obs_source_get_height, obs_source_get_width,
    obs_source_showing, obs_source_skip_video_filter, obs_source_t, obs_video_info,
};
use obs_wrapper::prelude::*;
use obs_wrapper::properties::{Properties, TextProp, TextType};
use obs_wrapper::source::*;
use obs_wrapper::wrapper::PtrWrapper;
use openmediatransport::{Codec, ColorSpace, FrameType, MediaFrame, VideoFlags};

use crate::clock::obs_ns_to_omt_ticks;
use crate::graphics_capture::BgraCapture;
use crate::ids::{DEFAULT_FILTER_NAME, PROP_FILTER_NAME};
use crate::send_session::SendSession;

pub struct OmtSendFilter {
    source: SourceRef,
    name_template: String,
    session: Option<SendSession>,
    capture: Option<BgraCapture>,
    rendered: bool,
    fps_num: u32,
    fps_den: u32,
    sample_rate: u32,
}

impl OmtSendFilter {
    fn resolved_name(&self) -> String {
        let mut name = self.name_template.clone();
        if name.contains("${source}") {
            if let Some(parent) = self.source.filter_parent() {
                if let Ok(n) = parent.name() {
                    name = name.replace("${source}", n.as_str());
                }
            }
        }
        if name.contains("${filter}") {
            if let Ok(n) = self.source.name() {
                name = name.replace("${filter}", n.as_str());
            }
        }
        let name = name.trim();
        if name.is_empty() {
            DEFAULT_FILTER_NAME.to_string()
        } else {
            name.to_string()
        }
    }

    fn restart_sender(&mut self) {
        self.session = None;
        match SendSession::start(self.resolved_name()) {
            Ok(s) => self.session = Some(s),
            Err(e) => warn!("OMT filter sender failed: {e}"),
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
        let mut this = Self {
            source,
            name_template: DEFAULT_FILTER_NAME.to_string(),
            session: None,
            capture: Some(BgraCapture::new()),
            rendered: false,
            fps_num: ovi.fps_num.max(1),
            fps_den: ovi.fps_den.max(1),
            sample_rate: oai.samples_per_sec.max(1),
        };
        if let Some(n) = create
            .settings
            .get::<obs_wrapper::string::ObsString>(ObsString::from(PROP_FILTER_NAME))
        {
            let s = n.as_str().trim();
            if !s.is_empty() {
                this.name_template = s.to_string();
            }
        }
        this
    }
}

impl GetNameSource for OmtSendFilter {
    fn get_name() -> ObsString {
        obs_string!("OMT Dedicated Output")
    }
}

impl GetDefaultsSource for OmtSendFilter {
    fn get_defaults(settings: &mut DataObj) {
        settings.set_default::<obs_wrapper::string::ObsString>(
            ObsString::from(PROP_FILTER_NAME),
            ObsString::from(DEFAULT_FILTER_NAME),
        );
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
        props
    }
}

impl UpdateSource for OmtSendFilter {
    fn update(&mut self, settings: &mut DataObj, _context: &mut GlobalContext) {
        if let Some(n) =
            settings.get::<obs_wrapper::string::ObsString>(ObsString::from(PROP_FILTER_NAME))
        {
            let s = n.as_str().trim();
            if !s.is_empty() {
                self.name_template = s.to_string();
            }
        }
        if self.source.filter_parent().is_some() {
            self.restart_sender();
        }
    }
}

impl FilterAddSource for OmtSendFilter {
    fn filter_add(&mut self, _source: SourceRef) {
        self.restart_sender();
    }
}

impl FilterRemoveSource for OmtSendFilter {
    fn filter_remove(&mut self, _source: SourceRef) {
        self.session = None;
    }
}

impl VideoTickSource for OmtSendFilter {
    fn video_tick(&mut self, _seconds: f32) {
        self.rendered = false;
    }
}

impl VideoRenderSource for OmtSendFilter {
    fn video_render(&mut self, _context: &mut GlobalContext, _render: &mut VideoRenderContext) {
        unsafe {
            obs_source_skip_video_filter(self.source.as_ptr() as *mut obs_source_t);
        }
        if self.rendered {
            return;
        }
        self.rendered = true;
        let parent = unsafe { obs_filter_get_parent(self.source.as_ptr()) };
        let target = unsafe { obs_filter_get_target(self.source.as_ptr()) };
        if parent.is_null() || target.is_null() {
            return;
        }
        if !unsafe { obs_source_showing(parent) } {
            return;
        }
        let width = unsafe { obs_source_get_width(self.source.as_ptr() as *mut obs_source_t) };
        let height = unsafe { obs_source_get_height(self.source.as_ptr() as *mut obs_source_t) };
        if width == 0 || height == 0 {
            return;
        }
        let Some(capture) = self.capture.as_mut() else {
            return;
        };
        let pixels = capture.capture_with(width, height, || unsafe {
            if target == parent {
                obs_source_skip_video_filter(self.source.as_ptr() as *mut obs_source_t);
            } else {
                obs_wrapper::obs_sys::obs_source_video_render(target);
            }
        });
        let Some(pixels) = pixels else {
            return;
        };
        let Some(session) = self.session.as_ref() else {
            return;
        };
        let ts = crate::clock::obs_ns_to_omt_ticks(unsafe { obs_get_video_frame_time() });
        session.push_video(MediaFrame {
            frame_type: FrameType::VIDEO,
            timestamp: ts,
            codec: Codec::Bgra as i32,
            width: width as i32,
            height: height as i32,
            stride: (width * 4) as i32,
            flags: VideoFlags::ALPHA | VideoFlags::PREMULTIPLIED,
            frame_rate_n: self.fps_num as i32,
            frame_rate_d: self.fps_den as i32,
            aspect_ratio: width as f32 / height.max(1) as f32,
            color_space: ColorSpace::Undefined,
            data: pixels,
            ..MediaFrame::default()
        });
    }
}

impl FilterAudioSource for OmtSendFilter {
    fn filter_audio(&mut self, audio: &mut obs_wrapper::media::AudioDataContext) {
        let Some(session) = self.session.as_ref() else {
            return;
        };
        let frames = audio.frames();
        if frames == 0 {
            return;
        }
        let channels = 2usize.min(audio.channels());
        let mut data = vec![0u8; channels * frames * 4];
        for c in 0..channels {
            if let Some(slice) = audio.get_channel_as_mut_slice(c) {
                let bytes = bytemuck_f32(slice);
                let dst = c * frames * 4;
                if dst + bytes.len() <= data.len() {
                    data[dst..dst + bytes.len()].copy_from_slice(bytes);
                }
            }
        }
        session.push_audio(MediaFrame {
            frame_type: FrameType::AUDIO,
            timestamp: obs_ns_to_omt_ticks(audio.timestamp()),
            codec: Codec::Fpa1 as i32,
            sample_rate: self.sample_rate as i32,
            channels: channels as i32,
            samples_per_channel: frames as i32,
            data,
            ..MediaFrame::default()
        });
    }
}

fn bytemuck_f32(slice: &[f32]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(slice.as_ptr() as *const u8, slice.len() * 4) }
}

impl Drop for OmtSendFilter {
    fn drop(&mut self) {
        self.session = None;
        if let Some(mut cap) = self.capture.take() {
            cap.destroy();
        }
    }
}
