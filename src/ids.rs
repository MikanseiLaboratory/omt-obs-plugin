#![allow(dead_code)]

//! OBS / OMT identifier constants.
//!
//! IDs, property keys, config filename, and default sender names are
//! intentionally distinct from official C# `omtplugin` so both plugins can
//! load in the same OBS process.

use obs_wrapper::data::DataObj;
use obs_wrapper::properties::{Properties, TextProp, TextType};
use obs_wrapper::{obs_string, string::ObsString};

pub const SOURCE_ID: &str = "omtobs_source";
pub const OUTPUT_ID: &str = "omtobs_output";
pub const SETTINGS_ID: &str = "omtobs_output_settings";
pub const FILTER_ID: &str = "omtobs_filter";

pub const CONFIG_FILE: &str = "omt-obs-plugin.json";
pub const OFFICIAL_PLUGIN_URL: &str = "https://github.com/openmediatransport/omtplugin";

pub const PROP_SOURCE: &str = "omtobs_source_url";
pub const PROP_QUALITY: &str = "omtobs_quality";
pub const PROP_COLOR_SPACE: &str = "omtobs_color_space";
pub const PROP_PREVIEW: &str = "omtobs_preview";
pub const PROP_BANDWIDTH_POLICY: &str = "omtobs_bandwidth_policy";
pub const PROP_ENABLED: &str = "omtobs_enabled";
pub const PROP_NAME: &str = "omtobs_name";
pub const PROP_PREVIEW_ENABLED: &str = "omtobs_preview_enabled";
pub const PROP_PREVIEW_NAME: &str = "omtobs_preview_name";
pub const PROP_FILTER_NAME: &str = "omtobs_filter_name";
pub const PROP_PROGRAM_MODE: &str = "omtobs_program_mode";
pub const PROP_PREVIEW_MODE: &str = "omtobs_preview_mode";
pub const PROP_FILTER_MODE: &str = "omtobs_filter_mode";
pub const PROP_OFFICIAL_URL: &str = "omtobs_official_url";

pub const DEFAULT_OUTPUT_NAME: &str = "OBS Output (Rust)";
pub const DEFAULT_PREVIEW_NAME: &str = "OBS Preview (Rust)";
pub const DEFAULT_FILTER_NAME: &str = "${source}";

pub fn source_id() -> ObsString {
    obs_string!("omtobs_source")
}
pub fn output_id() -> ObsString {
    obs_string!("omtobs_output")
}
pub fn settings_id() -> ObsString {
    obs_string!("omtobs_output_settings")
}
pub fn filter_id() -> ObsString {
    obs_string!("omtobs_filter")
}

pub fn apply_official_plugin_note_default(settings: &mut DataObj<'_>) {
    settings.set_default::<obs_wrapper::string::ObsString>(
        ObsString::from(PROP_OFFICIAL_URL),
        obs_string!("https://github.com/openmediatransport/omtplugin"),
    );
}

pub fn add_official_plugin_note(props: &mut Properties) {
    props.add(
        ObsString::from(PROP_OFFICIAL_URL),
        obs_string!("Official C# plugin (copy URL)"),
        TextProp::new(TextType::Default),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    const OFFICIAL_IDS: &[&str] = &["omtsource", "omtoutput", "omtoutputsettings", "omt_filter"];
    const OFFICIAL_KEYS: &[&str] = &[
        "sourceProperty",
        "qualityProperty",
        "csProperty",
        "previewProperty",
        "enabledProperty",
        "nameProperty",
        "previewEnabledProperty",
        "previewNameProperty",
        "filterNameProperty",
        "programModeProperty",
        "previewModeProperty",
        "filterModeProperty",
        "bandwidthPolicyProperty",
    ];

    #[test]
    fn ids_do_not_collide_with_official_omtplugin() {
        for id in [SOURCE_ID, OUTPUT_ID, SETTINGS_ID, FILTER_ID] {
            assert!(
                !OFFICIAL_IDS.contains(&id),
                "OBS id {id} collides with official omtplugin"
            );
        }
        assert_ne!(CONFIG_FILE, "omtplugin.json");
        assert_ne!(DEFAULT_OUTPUT_NAME, "OBS Output");
        assert_ne!(DEFAULT_PREVIEW_NAME, "OBS Preview");
    }

    #[test]
    fn property_keys_do_not_collide_with_official_omtplugin() {
        for key in [
            PROP_SOURCE,
            PROP_QUALITY,
            PROP_COLOR_SPACE,
            PROP_PREVIEW,
            PROP_BANDWIDTH_POLICY,
            PROP_ENABLED,
            PROP_NAME,
            PROP_PREVIEW_ENABLED,
            PROP_PREVIEW_NAME,
            PROP_FILTER_NAME,
            PROP_PROGRAM_MODE,
            PROP_PREVIEW_MODE,
            PROP_FILTER_MODE,
            PROP_OFFICIAL_URL,
        ] {
            assert!(
                !OFFICIAL_KEYS.contains(&key),
                "settings key {key} collides with official omtplugin"
            );
        }
    }
}
