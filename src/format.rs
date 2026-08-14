//! OBS ↔ OMT pixel format mapping.

use obs_wrapper::media::VideoFormat;
use obs_wrapper::obs_sys::{audio_data, video_data, video_format};
use openmediatransport::{Codec, ColorSpace, FrameType, MediaFrame, VideoFlags};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SdrMappingError {
    UnsupportedHdr,
    UnsupportedFormat,
    NullPlane,
}

pub fn obs_format_to_omt_codec(format: VideoFormat) -> Result<Codec, SdrMappingError> {
    match format {
        VideoFormat::NV12 => Ok(Codec::Nv12),
        VideoFormat::UYVY => Ok(Codec::Uyvy),
        VideoFormat::YUY2 => Ok(Codec::Yuy2),
        VideoFormat::BGRA | VideoFormat::BGRX => Ok(Codec::Bgra),
        VideoFormat::P010 | VideoFormat::P216 => Err(SdrMappingError::UnsupportedHdr),
        _ => Err(SdrMappingError::UnsupportedFormat),
    }
}

#[allow(dead_code)]
pub fn is_hdr_format(format: video_format) -> bool {
    matches!(
        VideoFormat::from_raw(format),
        Ok(VideoFormat::P010) | Ok(VideoFormat::P216)
    )
}

pub fn video_flags_for(format: VideoFormat) -> VideoFlags {
    match format {
        VideoFormat::BGRA => VideoFlags::ALPHA | VideoFlags::PREMULTIPLIED,
        _ => VideoFlags::NONE,
    }
}

pub fn packed_frame_len(format: VideoFormat, _width: u32, height: u32, stride0: u32) -> usize {
    let h = height as usize;
    let stride = stride0 as usize;
    match format {
        VideoFormat::NV12 => stride * h + stride * h.div_ceil(2),
        VideoFormat::P010 | VideoFormat::P216 => stride * h * 2,
        _ => stride * h,
    }
}

pub fn copy_video_data(
    frame: &video_data,
    width: u32,
    height: u32,
    format: VideoFormat,
    fps_num: u32,
    fps_den: u32,
    color_space: ColorSpace,
) -> Result<MediaFrame, SdrMappingError> {
    let codec = obs_format_to_omt_codec(format)?;
    let stride = frame.linesize[0];
    if frame.data[0].is_null() {
        return Err(SdrMappingError::NullPlane);
    }
    let mut data = vec![0u8; packed_frame_len(format, width, height, stride)];
    unsafe {
        match format {
            VideoFormat::NV12 => {
                let y_len = stride as usize * height as usize;
                let uv_len = stride as usize * (height as usize).div_ceil(2);
                std::ptr::copy_nonoverlapping(frame.data[0], data.as_mut_ptr(), y_len);
                if frame.data[1].is_null() {
                    return Err(SdrMappingError::NullPlane);
                }
                std::ptr::copy_nonoverlapping(frame.data[1], data.as_mut_ptr().add(y_len), uv_len);
            }
            _ => {
                std::ptr::copy_nonoverlapping(frame.data[0], data.as_mut_ptr(), data.len());
            }
        }
    }
    Ok(MediaFrame {
        frame_type: FrameType::VIDEO,
        timestamp: crate::clock::obs_ns_to_omt_ticks(frame.timestamp),
        codec: codec as i32,
        width: width as i32,
        height: height as i32,
        stride: stride as i32,
        flags: video_flags_for(format),
        frame_rate_n: fps_num as i32,
        frame_rate_d: fps_den.max(1) as i32,
        aspect_ratio: if height == 0 {
            16.0 / 9.0
        } else {
            width as f32 / height as f32
        },
        color_space,
        data,
        ..MediaFrame::default()
    })
}

pub fn copy_audio_data(
    frame: &audio_data,
    channels: u32,
    sample_rate: u32,
) -> Result<MediaFrame, SdrMappingError> {
    let ch = channels.max(1) as usize;
    let samples = frame.frames as usize;
    if frame.data[0].is_null() {
        return Err(SdrMappingError::NullPlane);
    }
    let mut data = vec![0u8; ch * samples * 4];
    unsafe {
        for c in 0..ch {
            let src = frame.data[c];
            if src.is_null() {
                continue;
            }
            let dst = data.as_mut_ptr().add(c * samples * 4);
            std::ptr::copy_nonoverlapping(src, dst, samples * 4);
        }
    }
    Ok(MediaFrame {
        frame_type: FrameType::AUDIO,
        timestamp: crate::clock::obs_ns_to_omt_ticks(frame.timestamp),
        codec: Codec::Fpa1 as i32,
        sample_rate: sample_rate as i32,
        channels: ch as i32,
        samples_per_channel: samples as i32,
        data,
        ..MediaFrame::default()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sdr_formats_map() {
        assert_eq!(
            obs_format_to_omt_codec(VideoFormat::BGRA).unwrap(),
            Codec::Bgra
        );
        assert_eq!(
            obs_format_to_omt_codec(VideoFormat::NV12).unwrap(),
            Codec::Nv12
        );
        assert_eq!(
            obs_format_to_omt_codec(VideoFormat::P010).unwrap_err(),
            SdrMappingError::UnsupportedHdr
        );
    }

    #[test]
    fn packed_len_nv12() {
        assert_eq!(
            packed_frame_len(VideoFormat::NV12, 1280, 720, 1280),
            1_382_400
        );
    }
}
