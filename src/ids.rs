#![allow(dead_code)]

//! OBS / OMT identifier constants.

use obs_wrapper::{obs_string, string::ObsString};

pub const SOURCE_ID: &str = "omtsource";
pub const OUTPUT_ID: &str = "omtoutput";
pub const SETTINGS_ID: &str = "omtoutputsettings";
pub const FILTER_ID: &str = "omt_filter";

pub const PROP_SOURCE: &str = "sourceProperty";
pub const PROP_QUALITY: &str = "qualityProperty";
pub const PROP_COLOR_SPACE: &str = "csProperty";
pub const PROP_PREVIEW: &str = "previewProperty";
pub const PROP_ENABLED: &str = "enabledProperty";
pub const PROP_NAME: &str = "nameProperty";
pub const PROP_PREVIEW_ENABLED: &str = "previewEnabledProperty";
pub const PROP_PREVIEW_NAME: &str = "previewNameProperty";
pub const PROP_FILTER_NAME: &str = "filterNameProperty";

pub const DEFAULT_OUTPUT_NAME: &str = "OBS Output";
pub const DEFAULT_PREVIEW_NAME: &str = "OBS Preview";
pub const DEFAULT_FILTER_NAME: &str = "${source}";

pub fn source_id() -> ObsString {
    obs_string!("omtsource")
}
pub fn output_id() -> ObsString {
    obs_string!("omtoutput")
}
pub fn settings_id() -> ObsString {
    obs_string!("omtoutputsettings")
}
pub fn filter_id() -> ObsString {
    obs_string!("omt_filter")
}
