//! Persistent OMT output settings (`omt-obs-plugin.json`).

use std::borrow::Cow;
use std::fs;
use std::path::Path;

use log::info;
use obs_wrapper::data::DataObj;
use obs_wrapper::string::ObsString;

use crate::ids::{
    DEFAULT_OUTPUT_NAME, DEFAULT_PREVIEW_NAME, PROP_ENABLED, PROP_NAME, PROP_PREVIEW_ENABLED,
    PROP_PREVIEW_NAME, PROP_PROGRAM_MODE,
};
use crate::media_mode::MediaMode;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputConfig {
    pub enabled: bool,
    pub name: String,
    pub program_mode: MediaMode,
    pub preview_enabled: bool,
    pub preview_name: String,
}

impl Default for OutputConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            name: DEFAULT_OUTPUT_NAME.to_string(),
            program_mode: MediaMode::Embedded,
            preview_enabled: false,
            preview_name: DEFAULT_PREVIEW_NAME.to_string(),
        }
    }
}

fn read_string(data: &DataObj<'_>, key: &str) -> Option<String> {
    let s = data.get::<Cow<str>>(ObsString::from(key))?;
    let s = s.trim();
    if s.is_empty() {
        None
    } else {
        Some(s.to_string())
    }
}

impl OutputConfig {
    pub fn from_data(data: &DataObj<'_>) -> Self {
        let mut cfg = Self::default();
        if let Some(v) = data.get::<bool>(ObsString::from(PROP_ENABLED)) {
            cfg.enabled = v;
        }
        if let Some(s) = read_string(data, PROP_NAME) {
            cfg.name = s;
        }
        if let Some(v) = data.get::<i64>(ObsString::from(PROP_PROGRAM_MODE)) {
            cfg.program_mode = MediaMode::from_i64(v);
        }
        if let Some(v) = data.get::<bool>(ObsString::from(PROP_PREVIEW_ENABLED)) {
            cfg.preview_enabled = v;
        }
        if let Some(s) = read_string(data, PROP_PREVIEW_NAME) {
            cfg.preview_name = s;
        }
        cfg
    }

    pub fn apply_defaults(data: &mut DataObj<'_>) {
        data.set_default::<bool>(ObsString::from(PROP_ENABLED), false);
        data.set_default::<obs_wrapper::string::ObsString>(
            ObsString::from(PROP_NAME),
            ObsString::from(DEFAULT_OUTPUT_NAME),
        );
        data.set_default::<i64>(ObsString::from(PROP_PROGRAM_MODE), 0);
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
        data.set_int(
            ObsString::from(PROP_PROGRAM_MODE),
            self.program_mode.as_i64(),
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
