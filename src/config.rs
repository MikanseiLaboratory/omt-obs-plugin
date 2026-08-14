//! Persistent OMT output settings (`omtplugin.json`).

use std::fs;
use std::path::Path;

use log::info;
use obs_wrapper::data::DataObj;
use obs_wrapper::string::ObsString;

use crate::ids::{
    DEFAULT_OUTPUT_NAME, DEFAULT_PREVIEW_NAME, PROP_ENABLED, PROP_NAME, PROP_PREVIEW_ENABLED,
    PROP_PREVIEW_NAME,
};

#[derive(Debug, Clone)]
pub struct OutputConfig {
    pub enabled: bool,
    pub name: String,
    pub preview_enabled: bool,
    pub preview_name: String,
}

impl Default for OutputConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            name: DEFAULT_OUTPUT_NAME.to_string(),
            preview_enabled: false,
            preview_name: DEFAULT_PREVIEW_NAME.to_string(),
        }
    }
}

impl OutputConfig {
    pub fn from_data(data: &DataObj<'_>) -> Self {
        let mut cfg = Self::default();
        if let Some(v) = data.get::<bool>(ObsString::from(PROP_ENABLED)) {
            cfg.enabled = v;
        }
        if let Some(v) = data.get::<obs_wrapper::string::ObsString>(ObsString::from(PROP_NAME)) {
            let s = v.as_str().trim();
            if !s.is_empty() {
                cfg.name = s.to_string();
            }
        }
        if let Some(v) = data.get::<bool>(ObsString::from(PROP_PREVIEW_ENABLED)) {
            cfg.preview_enabled = v;
        }
        if let Some(v) =
            data.get::<obs_wrapper::string::ObsString>(ObsString::from(PROP_PREVIEW_NAME))
        {
            let s = v.as_str().trim();
            if !s.is_empty() {
                cfg.preview_name = s.to_string();
            }
        }
        cfg
    }

    pub fn apply_defaults(data: &mut DataObj<'_>) {
        data.set_default::<bool>(ObsString::from(PROP_ENABLED), false);
        data.set_default::<obs_wrapper::string::ObsString>(
            ObsString::from(PROP_NAME),
            ObsString::from(DEFAULT_OUTPUT_NAME),
        );
        data.set_default::<bool>(ObsString::from(PROP_PREVIEW_ENABLED), false);
        data.set_default::<obs_wrapper::string::ObsString>(
            ObsString::from(PROP_PREVIEW_NAME),
            ObsString::from(DEFAULT_PREVIEW_NAME),
        );
    }

    pub fn write_to(&self, data: &mut DataObj<'_>) {
        data.set_bool(ObsString::from(PROP_ENABLED), self.enabled);
        data.set_string(
            ObsString::from(PROP_NAME),
            ObsString::from(self.name.as_str()),
        );
        data.set_bool(ObsString::from(PROP_PREVIEW_ENABLED), self.preview_enabled);
        data.set_string(
            ObsString::from(PROP_PREVIEW_NAME),
            ObsString::from(self.preview_name.as_str()),
        );
    }

    pub fn load_file(path: &str) -> Self {
        if Path::new(path).exists() {
            if let Some(data) = DataObj::from_json_file(ObsString::from(path), None) {
                return Self::from_data(&data);
            }
        }
        Self::default()
    }

    pub fn save_file(&self, path: &str) {
        if let Some(parent) = Path::new(path).parent() {
            let _ = fs::create_dir_all(parent);
        }
        let mut data = DataObj::new();
        self.write_to(&mut data);
        if data.save_json(ObsString::from(path)) {
            info!("saved OMT config to {path}");
        }
    }
}
