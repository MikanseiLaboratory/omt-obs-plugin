//! Studio-mode Preview OMT sender (DistroAV-style render callback).

use std::ptr;
use std::sync::Mutex;

use log::{info, warn};
use obs_wrapper::frontend::FrontendEvent;
use obs_wrapper::obs_sys::{
    obs_add_main_render_callback, obs_get_video_frame_time, obs_get_video_info,
    obs_remove_main_render_callback, obs_source_get_base_height, obs_source_get_base_width,
    obs_source_get_ref, obs_source_release, obs_source_t, obs_video_info,
};
use openmediatransport::{Codec, ColorSpace, FrameType, MediaFrame, VideoFlags};

use crate::clock::obs_ns_to_omt_ticks;
use crate::graphics_capture::BgraCapture;
use crate::send_session::SendSession;
use obs_wrapper::wrapper::PtrWrapper;

struct PreviewState {
    session: Option<SendSession>,
    capture: Option<BgraCapture>,
    source: *mut obs_source_t,
    fps_num: u32,
    fps_den: u32,
    callback_installed: bool,
}

unsafe impl Send for PreviewState {}

static STATE: Mutex<PreviewState> = Mutex::new(PreviewState {
    session: None,
    capture: None,
    source: ptr::null_mut(),
    fps_num: 60,
    fps_den: 1,
    callback_installed: false,
});

fn set_source(state: &mut PreviewState, src: *mut obs_source_t) {
    unsafe {
        if !state.source.is_null() {
            obs_source_release(state.source);
        }
        state.source = if src.is_null() {
            ptr::null_mut()
        } else {
            obs_source_get_ref(src)
        };
    }
}

pub fn apply(enabled: bool, name: &str) {
    stop();
    if !enabled {
        return;
    }
    let session = match SendSession::start(name) {
        Ok(s) => s,
        Err(e) => {
            warn!("OMT preview sender failed: {e}");
            return;
        }
    };
    let mut ovi = obs_video_info {
        graphics_module: ptr::null(),
        fps_num: 60,
        fps_den: 1,
        base_width: 0,
        base_height: 0,
        output_width: 0,
        output_height: 0,
        output_format: 0,
        adapter: 0,
        gpu_conversion: false,
        colorspace: 0,
        range: 0,
        scale_type: 0,
    };
    unsafe {
        obs_get_video_info(&mut ovi);
    }
    if let Ok(mut state) = STATE.lock() {
        state.session = Some(session);
        state.capture = Some(BgraCapture::new());
        state.fps_num = ovi.fps_num.max(1);
        state.fps_den = ovi.fps_den.max(1);
        refresh_source(&mut state);
        if !state.callback_installed {
            unsafe {
                obs_add_main_render_callback(Some(render_preview), ptr::null_mut());
            }
            state.callback_installed = true;
        }
        info!("OMT preview output started as '{name}'");
    }
}

pub fn stop() {
    if let Ok(mut state) = STATE.lock() {
        set_source(&mut state, ptr::null_mut());
        state.session = None;
        if let Some(mut cap) = state.capture.take() {
            cap.destroy();
        }
        if state.callback_installed {
            unsafe {
                obs_remove_main_render_callback(Some(render_preview), ptr::null_mut());
            }
            state.callback_installed = false;
        }
    }
}

pub fn on_frontend_event(event: FrontendEvent) {
    if let Ok(mut state) = STATE.lock() {
        if state.session.is_none() {
            return;
        }
        match event {
            FrontendEvent::StudioModeEnabled
            | FrontendEvent::PreviewSceneChanged
            | FrontendEvent::StudioModeDisabled
            | FrontendEvent::SceneChanged => {
                refresh_source(&mut state);
            }
            FrontendEvent::SceneCollectionCleanup | FrontendEvent::Exit => {
                set_source(&mut state, ptr::null_mut());
            }
            _ => {}
        }
    }
}

fn refresh_source(state: &mut PreviewState) {
    let src = if obs_wrapper::frontend::preview_program_mode_active() {
        obs_wrapper::frontend::current_preview_scene()
    } else {
        obs_wrapper::frontend::current_scene()
    };
    let ptr = src
        .as_ref()
        .map(|s| unsafe { s.as_ptr() as *mut obs_source_t })
        .unwrap_or(ptr::null_mut());
    set_source(state, ptr);
}

unsafe extern "C" fn render_preview(_param: *mut std::ffi::c_void, _cx: u32, _cy: u32) {
    let Ok(mut state) = STATE.try_lock() else {
        return;
    };
    let source = state.source;
    let width = obs_source_get_base_width(source);
    let height = obs_source_get_base_height(source);
    let pixels = {
        let Some(capture) = state.capture.as_mut() else {
            return;
        };
        let Some(pixels) = capture.capture(source, width, height) else {
            return;
        };
        pixels
    };
    let Some(session) = state.session.as_ref() else {
        return;
    };
    let ts = obs_ns_to_omt_ticks(obs_get_video_frame_time());
    session.push_video(MediaFrame {
        frame_type: FrameType::VIDEO,
        timestamp: ts,
        codec: Codec::Bgra as i32,
        width: width as i32,
        height: height as i32,
        stride: (width * 4) as i32,
        flags: VideoFlags::ALPHA | VideoFlags::PREMULTIPLIED,
        frame_rate_n: state.fps_num as i32,
        frame_rate_d: state.fps_den as i32,
        aspect_ratio: if height == 0 {
            16.0 / 9.0
        } else {
            width as f32 / height as f32
        },
        color_space: ColorSpace::Undefined,
        data: pixels,
        ..MediaFrame::default()
    });
}
