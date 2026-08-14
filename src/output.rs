//! Program OMT output (`omtoutput`) plus Tools-menu settings.

use std::sync::Mutex;

use log::{info, warn};
use obs_wrapper::data::DataObj;
use obs_wrapper::media::VideoFormat;
use obs_wrapper::obs_string;
use obs_wrapper::obs_sys::{audio_data, video_data, OBS_SOURCE_CAP_DISABLED};
use obs_wrapper::output::*;
use obs_wrapper::prelude::*;
use obs_wrapper::properties::{BoolProp, Properties, TextProp, TextType};
use obs_wrapper::source::*;
use obs_wrapper::wrapper::PtrWrapper;
use openmediatransport::ColorSpace;

use crate::config::OutputConfig;
use crate::format::{copy_audio_data, copy_video_data, obs_format_to_omt_codec};
use crate::ids::{PROP_ENABLED, PROP_NAME, PROP_PREVIEW_ENABLED, PROP_PREVIEW_NAME};
use crate::send_session::SendSession;

pub struct OmtOutput {
    output: OutputRef,
    session: Option<SendSession>,
    width: u32,
    height: u32,
    fps_num: u32,
    fps_den: u32,
    format: Option<VideoFormat>,
    sample_rate: u32,
    channels: u32,
}

impl Outputable for OmtOutput {
    fn get_id() -> ObsString {
        crate::ids::output_id()
    }

    fn create(_context: &mut CreatableOutputContext<'_, Self>, output: OutputRef) -> Self {
        Self {
            output,
            session: None,
            width: 0,
            height: 0,
            fps_num: 60,
            fps_den: 1,
            format: None,
            sample_rate: 48000,
            channels: 2,
        }
    }

    fn start(&mut self) -> bool {
        let video = self.output.video();
        let audio = self.output.audio();
        let info = video.info();
        let Some(format) = info.format else {
            warn!("OMT output: unknown video format");
            return false;
        };
        if let Err(e) = obs_format_to_omt_codec(format) {
            warn!("OMT output refuses format {format:?}: {e:?}");
            return false;
        }
        let name = current_config().name;
        match SendSession::start(name) {
            Ok(session) => {
                self.session = Some(session);
                self.width = info.width;
                self.height = info.height;
                self.fps_num = info.frame_rate.round().max(1.0) as u32;
                self.fps_den = 1;
                if info.frame_rate > 0.0 {
                    // Prefer exact fraction from OBS video info when available.
                    self.fps_num = info.frame_rate as u32;
                }
                self.format = Some(format);
                self.sample_rate = audio.sample_rate() as u32;
                self.channels = audio.channels() as u32;
                if !self.output.start_capture(0) {
                    warn!("OMT output: begin_data_capture failed");
                    self.session = None;
                    return false;
                }
                info!(
                    "OMT program output started {}x{} {:?}",
                    self.width, self.height, format
                );
                true
            }
            Err(e) => {
                warn!("OMT output start failed: {e}");
                false
            }
        }
    }

    fn stop(&mut self, _ts: u64) {
        self.output.stop_capture();
        self.session = None;
        info!("OMT program output stopped");
    }
}

impl GetNameOutput for OmtOutput {
    fn get_name() -> ObsString {
        obs_string!("OMT Output")
    }
}

impl RawVideoOutput for OmtOutput {
    fn raw_video(&mut self, frame: &mut video_data) {
        let (Some(session), Some(format)) = (self.session.as_ref(), self.format) else {
            return;
        };
        match copy_video_data(
            frame,
            self.width,
            self.height,
            format,
            self.fps_num,
            self.fps_den,
            ColorSpace::Undefined,
        ) {
            Ok(media) => session.push_video(media),
            Err(e) => warn!("OMT raw_video copy failed: {e:?}"),
        }
    }
}

impl RawAudioOutput for OmtOutput {
    fn raw_audio(&mut self, frame: &mut audio_data) {
        let Some(session) = self.session.as_ref() else {
            return;
        };
        match copy_audio_data(frame, self.channels, self.sample_rate) {
            Ok(media) => session.push_audio(media),
            Err(e) => warn!("OMT raw_audio copy failed: {e:?}"),
        }
    }
}

// --- settings source + lifecycle -------------------------------------------

struct Controller {
    config_path: Option<String>,
    config: OutputConfig,
    settings_source: Option<SourceRef>,
    main: Option<OutputRef>,
}

unsafe impl Send for Controller {}

static CONTROLLER: Mutex<Controller> = Mutex::new(Controller {
    config_path: None,
    config: OutputConfig {
        enabled: false,
        name: String::new(),
        preview_enabled: false,
        preview_name: String::new(),
    },
    settings_source: None,
    main: None,
});

fn current_config() -> OutputConfig {
    CONTROLLER
        .lock()
        .map(|c| c.config.clone())
        .unwrap_or_default()
}

pub fn init_controller(path: String) {
    if let Ok(mut c) = CONTROLLER.lock() {
        c.config = OutputConfig::load_file(&path);
        c.config_path = Some(path);
    }
}

pub fn on_frontend_event(event: obs_wrapper::frontend::FrontendEvent) {
    use obs_wrapper::frontend::FrontendEvent::*;
    match event {
        FinishedLoading | ProfileChanged => apply_outputs(),
        ProfileChanging | Exit | SceneCollectionCleanup => destroy_outputs(),
        StudioModeEnabled | StudioModeDisabled | PreviewSceneChanged | SceneChanged => {
            crate::preview::on_frontend_event(event);
        }
        _ => {}
    }
}

pub fn show_settings() {
    ensure_settings_source();
    if let Ok(c) = CONTROLLER.lock() {
        if let Some(src) = c.settings_source.as_ref() {
            obs_wrapper::frontend::open_source_properties(src);
        }
    }
}

fn ensure_settings_source() {
    let mut c = match CONTROLLER.lock() {
        Ok(g) => g,
        Err(_) => return,
    };
    if c.settings_source.is_some() {
        return;
    }
    let mut data = DataObj::new();
    c.config.write_to(&mut data);
    let src = unsafe {
        obs_wrapper::obs_sys::obs_source_create(
            crate::ids::settings_id().as_ptr(),
            obs_string!("OMT Output Settings").as_ptr(),
            data.as_ptr_mut(),
            std::ptr::null_mut(),
        )
    };
    c.settings_source = unsafe { SourceRef::from_raw_unchecked(src) };
}

pub fn apply_outputs() {
    destroy_outputs();
    let cfg = current_config();
    if cfg.enabled {
        match OutputRef::new(crate::ids::output_id(), obs_string!("OMT Output"), None) {
            Ok(mut output) => {
                if output.start() {
                    if let Ok(mut c) = CONTROLLER.lock() {
                        c.main = Some(output);
                    }
                } else {
                    warn!("OMT program output failed to start");
                }
            }
            Err(e) => warn!("OMT program output create failed: {e}"),
        }
    }
    crate::preview::apply(cfg.preview_enabled, &cfg.preview_name);
}

pub fn destroy_outputs() {
    if let Ok(mut c) = CONTROLLER.lock() {
        if let Some(mut out) = c.main.take() {
            out.stop();
        }
    }
    crate::preview::stop();
}

pub fn persist_and_apply(settings: &mut DataObj) {
    let cfg = OutputConfig::from_data(settings);
    if let Ok(mut c) = CONTROLLER.lock() {
        c.config = cfg.clone();
        if let Some(path) = c.config_path.clone() {
            cfg.save_file(&path);
        }
    }
    apply_outputs();
}

pub struct OmtOutputSettings;

impl Sourceable for OmtOutputSettings {
    fn get_id() -> ObsString {
        crate::ids::settings_id()
    }
    fn get_type() -> SourceType {
        SourceType::Filter
    }
    fn create(_create: &mut CreatableSourceContext<Self>, _source: SourceRef) -> Self {
        Self
    }
}

impl GetNameSource for OmtOutputSettings {
    fn get_name() -> ObsString {
        obs_string!("OMT Output Settings")
    }
}

impl GetDefaultsSource for OmtOutputSettings {
    fn get_defaults(settings: &mut DataObj) {
        OutputConfig::apply_defaults(settings);
    }
}

impl GetPropertiesSource for OmtOutputSettings {
    fn get_properties(&mut self) -> Properties {
        let mut props = Properties::new();
        props.add(
            ObsString::from(PROP_ENABLED),
            obs_string!("Enable Program Output"),
            BoolProp,
        );
        props.add(
            ObsString::from(PROP_NAME),
            obs_string!("Program Source Name"),
            TextProp::new(TextType::Default),
        );
        props.add(
            ObsString::from(PROP_PREVIEW_ENABLED),
            obs_string!("Enable Preview Output"),
            BoolProp,
        );
        props.add(
            ObsString::from(PROP_PREVIEW_NAME),
            obs_string!("Preview Source Name"),
            TextProp::new(TextType::Default),
        );
        props
    }
}

impl UpdateSource for OmtOutputSettings {
    fn update(&mut self, settings: &mut DataObj, _context: &mut GlobalContext) {
        persist_and_apply(settings);
    }
}

impl SaveSource for OmtOutputSettings {
    fn save(&mut self, settings: &mut DataObj) {
        persist_and_apply(settings);
    }
}

pub fn settings_flags() -> u32 {
    OBS_SOURCE_CAP_DISABLED
}
