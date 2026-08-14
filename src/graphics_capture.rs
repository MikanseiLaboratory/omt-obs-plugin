//! Offscreen BGRA capture.

use std::ptr;

use obs_wrapper::obs_sys::{
    gs_blend_function, gs_blend_state_pop, gs_blend_state_push, gs_blend_type_GS_BLEND_ONE,
    gs_blend_type_GS_BLEND_ZERO, gs_clear, gs_color_format_GS_BGRA, gs_ortho, gs_stage_texture,
    gs_stagesurf_t, gs_stagesurface_create, gs_stagesurface_destroy, gs_stagesurface_map,
    gs_stagesurface_unmap, gs_texrender_begin, gs_texrender_create, gs_texrender_destroy,
    gs_texrender_end, gs_texrender_get_texture, gs_texrender_reset, gs_texrender_t,
    gs_zstencil_format_GS_ZS_NONE, obs_enter_graphics, obs_leave_graphics, obs_source_t,
    obs_source_video_render, vec4, GS_CLEAR_COLOR,
};

pub struct BgraCapture {
    texrender: *mut gs_texrender_t,
    stagesurface: *mut gs_stagesurf_t,
    width: u32,
    height: u32,
}

impl BgraCapture {
    pub fn new() -> Self {
        unsafe {
            obs_enter_graphics();
            let texrender =
                gs_texrender_create(gs_color_format_GS_BGRA, gs_zstencil_format_GS_ZS_NONE);
            obs_leave_graphics();
            Self {
                texrender,
                stagesurface: ptr::null_mut(),
                width: 0,
                height: 0,
            }
        }
    }

    fn ensure_size(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        if self.width == width && self.height == height && !self.stagesurface.is_null() {
            return;
        }
        unsafe {
            if !self.stagesurface.is_null() {
                gs_stagesurface_destroy(self.stagesurface);
            }
            self.stagesurface = gs_stagesurface_create(width, height, gs_color_format_GS_BGRA);
        }
        self.width = width;
        self.height = height;
    }

    /// Capture by running `draw` inside an offscreen texrender. Must run on the
    /// OBS graphics thread.
    pub fn capture_with<F: FnOnce()>(
        &mut self,
        width: u32,
        height: u32,
        draw: F,
    ) -> Option<Vec<u8>> {
        if width == 0 || height == 0 || self.texrender.is_null() {
            return None;
        }
        self.ensure_size(width, height);
        if self.stagesurface.is_null() {
            return None;
        }
        unsafe {
            gs_texrender_reset(self.texrender);
            if !gs_texrender_begin(self.texrender, width, height) {
                return None;
            }
            let background: vec4 = std::mem::zeroed();
            gs_clear(GS_CLEAR_COLOR, &background, 0.0, 0);
            gs_ortho(0.0, width as f32, 0.0, height as f32, -100.0, 100.0);
            gs_blend_state_push();
            gs_blend_function(gs_blend_type_GS_BLEND_ONE, gs_blend_type_GS_BLEND_ZERO);
            draw();
            gs_blend_state_pop();
            gs_texrender_end(self.texrender);

            let tex = gs_texrender_get_texture(self.texrender);
            gs_stage_texture(self.stagesurface, tex);

            let mut src: *mut u8 = ptr::null_mut();
            let mut linesize: u32 = 0;
            if !gs_stagesurface_map(self.stagesurface, &mut src, &mut linesize) || src.is_null() {
                return None;
            }
            let mut out = vec![0u8; (width * height * 4) as usize];
            for y in 0..height as usize {
                let src_off = y * linesize as usize;
                let dst_off = y * (width as usize * 4);
                ptr::copy_nonoverlapping(
                    src.add(src_off),
                    out.as_mut_ptr().add(dst_off),
                    width as usize * 4,
                );
            }
            gs_stagesurface_unmap(self.stagesurface);
            Some(out)
        }
    }

    /// Capture `source` into a tightly packed BGRA buffer. Must run on the OBS
    /// graphics thread.
    pub fn capture(
        &mut self,
        source: *mut obs_source_t,
        width: u32,
        height: u32,
    ) -> Option<Vec<u8>> {
        if source.is_null() {
            return None;
        }
        self.capture_with(width, height, || unsafe {
            obs_source_video_render(source);
        })
    }

    pub fn destroy(&mut self) {
        unsafe {
            obs_enter_graphics();
            if !self.stagesurface.is_null() {
                gs_stagesurface_destroy(self.stagesurface);
                self.stagesurface = ptr::null_mut();
            }
            if !self.texrender.is_null() {
                gs_texrender_destroy(self.texrender);
                self.texrender = ptr::null_mut();
            }
            obs_leave_graphics();
        }
        self.width = 0;
        self.height = 0;
    }
}

impl Drop for BgraCapture {
    fn drop(&mut self) {
        self.destroy();
    }
}
