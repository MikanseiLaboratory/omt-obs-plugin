//! Embedded / Video Only / Audio Only send mode.

use obs_wrapper::obs_string;
use obs_wrapper::properties::{ListProp, Properties};
use obs_wrapper::string::ObsString;
use openmediatransport::FrameType;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MediaMode {
    #[default]
    Embedded,
    Video,
    Audio,
}

impl MediaMode {
    pub fn from_i64(v: i64) -> Self {
        match v {
            1 => Self::Video,
            2 => Self::Audio,
            _ => Self::Embedded,
        }
    }

    pub fn as_i64(self) -> i64 {
        match self {
            Self::Embedded => 0,
            Self::Video => 1,
            Self::Audio => 2,
        }
    }

    pub fn frame_types(self) -> FrameType {
        match self {
            Self::Embedded => FrameType::VIDEO | FrameType::AUDIO | FrameType::METADATA,
            Self::Video => FrameType::VIDEO | FrameType::METADATA,
            Self::Audio => FrameType::AUDIO | FrameType::METADATA,
        }
    }

    pub fn video(self) -> bool {
        matches!(self, Self::Embedded | Self::Video)
    }

    pub fn audio(self) -> bool {
        matches!(self, Self::Embedded | Self::Audio)
    }
}

pub fn add_media_mode_list(props: &mut Properties, key: &str, label: ObsString) {
    let mut list: ListProp<i64> = props.add_list(ObsString::from(key), label, false);
    list.push(obs_string!("Embedded"), 0);
    list.push(obs_string!("Video Only"), 1);
    list.push(obs_string!("Audio Only"), 2);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        assert_eq!(MediaMode::from_i64(0), MediaMode::Embedded);
        assert_eq!(MediaMode::from_i64(1).as_i64(), 1);
        assert!(MediaMode::Video.video());
        assert!(!MediaMode::Video.audio());
        assert!(MediaMode::Audio.audio());
        assert!(!MediaMode::Audio.video());
    }
}
