use std::path::PathBuf;

use crate::section::Section;

#[derive(Debug, Default)]
pub struct DebugBoot {
    pub section: Option<Section>,
    pub rom: Option<PathBuf>,
    pub generation: Option<i32>,
    pub search: Option<String>,
    pub dialog: Option<String>,
    pub update_version: Option<String>,
    pub app_update: Option<String>,
    pub app_update_stage: Option<String>,
}

impl DebugBoot {
    pub fn from_env() -> Self {
        Self {
            section: variable("RANDOMON_DEBUG_SECTION").and_then(|name| Section::from_slug(&name)),
            rom: variable("RANDOMON_DEBUG_ROM").map(PathBuf::from),
            generation: variable("RANDOMON_DEBUG_GENERATION").and_then(|value| value.parse().ok()),
            search: variable("RANDOMON_DEBUG_SEARCH"),
            dialog: variable("RANDOMON_DEBUG_DIALOG"),
            update_version: variable("RANDOMON_DEBUG_UPDATE_VERSION"),
            app_update: variable("RANDOMON_DEBUG_APP_UPDATE"),
            app_update_stage: variable("RANDOMON_DEBUG_APP_UPDATE_STAGE"),
        }
    }
}

fn variable(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .filter(|value| !value.trim().is_empty())
}
