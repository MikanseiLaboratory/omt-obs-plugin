//! Dynamic receive bandwidth policy (Issue #6).

use obs_wrapper::obs_string;
use obs_wrapper::properties::{ListProp, Properties};
use obs_wrapper::string::ObsString;

/// When the OMT source should request the sender's low-bandwidth preview stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BandwidthPolicy {
    /// Always use the configured suggested quality (full).
    #[default]
    None,
    /// Preview unless the source is on Program (`obs_source_active`).
    NotOnProgram,
    /// Preview unless the source is shown anywhere (`obs_source_showing`).
    NotOnPreviewProgram,
    /// Always request preview (1/8) video.
    Always,
}

impl BandwidthPolicy {
    pub fn from_i64(v: i64) -> Self {
        match v {
            1 => Self::NotOnProgram,
            2 => Self::NotOnPreviewProgram,
            3 => Self::Always,
            _ => Self::None,
        }
    }

    pub fn as_i64(self) -> i64 {
        match self {
            Self::None => 0,
            Self::NotOnProgram => 1,
            Self::NotOnPreviewProgram => 2,
            Self::Always => 3,
        }
    }

    /// Resolve the policy from the new key, falling back to legacy `omtobs_preview`.
    pub fn from_settings(policy: Option<i64>, legacy_preview: bool) -> Self {
        match policy {
            Some(v) => Self::from_i64(v),
            None if legacy_preview => Self::Always,
            None => Self::None,
        }
    }

    /// Whether the receiver should request OMT preview given OBS visibility.
    ///
    /// Nested scenes are already reflected in `active` / `showing`. Multiview
    /// and projectors increment `showing` but not `active`. Preview vs Multiview
    /// cannot be distinguished on OBS 32.2.
    pub fn use_preview(self, active: bool, showing: bool) -> bool {
        match self {
            Self::Always => true,
            Self::NotOnProgram => !active,
            Self::NotOnPreviewProgram => !showing,
            Self::None => false,
        }
    }
}

pub fn add_bandwidth_policy_list(props: &mut Properties, key: &str) {
    let mut list: ListProp<i64> = props.add_list(
        ObsString::from(key),
        obs_string!("Save bandwidth when"),
        false,
    );
    list.push(obs_string!("None (always full)"), 0);
    list.push(obs_string!("Not on Program"), 1);
    list.push(obs_string!("Not on Preview/Program"), 2);
    list.push(obs_string!("Always"), 3);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        assert_eq!(BandwidthPolicy::from_i64(0), BandwidthPolicy::None);
        assert_eq!(BandwidthPolicy::from_i64(1).as_i64(), 1);
        assert_eq!(
            BandwidthPolicy::from_i64(2),
            BandwidthPolicy::NotOnPreviewProgram
        );
        assert_eq!(BandwidthPolicy::from_i64(3), BandwidthPolicy::Always);
        assert_eq!(BandwidthPolicy::from_i64(99), BandwidthPolicy::None);
        assert_eq!(
            BandwidthPolicy::from_settings(None, true),
            BandwidthPolicy::Always
        );
        assert_eq!(
            BandwidthPolicy::from_settings(None, false),
            BandwidthPolicy::None
        );
        assert_eq!(
            BandwidthPolicy::from_settings(Some(1), true),
            BandwidthPolicy::NotOnProgram
        );
    }

    #[test]
    fn use_preview_matrix() {
        let cases = [
            (BandwidthPolicy::Always, false, false, true),
            (BandwidthPolicy::Always, true, true, true),
            (BandwidthPolicy::None, false, false, false),
            (BandwidthPolicy::None, true, true, false),
            (BandwidthPolicy::NotOnProgram, false, false, true),
            (BandwidthPolicy::NotOnProgram, false, true, true),
            (BandwidthPolicy::NotOnProgram, true, true, false),
            (BandwidthPolicy::NotOnPreviewProgram, false, false, true),
            (BandwidthPolicy::NotOnPreviewProgram, false, true, false),
            (BandwidthPolicy::NotOnPreviewProgram, true, true, false),
        ];
        for (policy, active, showing, want) in cases {
            assert_eq!(
                policy.use_preview(active, showing),
                want,
                "{policy:?} active={active} showing={showing}"
            );
        }
    }
}
