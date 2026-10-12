use std::path::PathBuf;

use crate::run::{self, Finished, RunProgress, RunState};
use crate::section::Section;

const PREVIEW_PROGRESS: f32 = 0.4;
const PREVIEW_CHECK_VALUE: i32 = 0x5A3C_91E7;

#[derive(Debug, Default)]
pub struct DebugBoot {
    pub section: Option<Section>,
    pub rom: Option<PathBuf>,
    pub generation: Option<i32>,
    pub search: Option<String>,
    pub dialog: Option<String>,
    pub update_version: Option<String>,
    pub update: Option<PathBuf>,
    pub app_update: Option<String>,
    pub app_update_stage: Option<String>,
    pub output: Option<PathBuf>,
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
            update: variable("RANDOMON_DEBUG_UPDATE").map(PathBuf::from),
            app_update: variable("RANDOMON_DEBUG_APP_UPDATE"),
            app_update_stage: variable("RANDOMON_DEBUG_APP_UPDATE_STAGE"),
            output: variable("RANDOMON_DEBUG_OUTPUT").map(PathBuf::from),
        }
    }

    fn preview_output(&self, fallback: &str) -> PathBuf {
        self.output
            .clone()
            .unwrap_or_else(|| PathBuf::from(fallback))
    }

    pub fn run_preview(&self) -> Option<RunState> {
        let output = self.preview_output("Pokémon Yellow (Randomized).gbc");
        let log = PathBuf::from(format!("{}.log", output.display()));
        let finished = |race_code: Option<String>| Finished {
            headline: "Your randomized game is ready",
            output: output.clone(),
            log: Some(log.clone()),
            race_code,
            install_hint: None,
        };
        match self.dialog.as_deref()? {
            "progress" => Some(RunState::Running(RunProgress {
                fraction: PREVIEW_PROGRESS,
                label: "Randomizing wild Pokémon",
                detail: Some("Route 104 · 12 encounters".to_owned()),
            })),
            "result" => Some(RunState::Done(finished(None))),
            "race-result" => Some(RunState::Done(finished(Some(run::race_code(
                PREVIEW_CHECK_VALUE,
            ))))),
            "error" => Some(RunState::Failed(
                "Couldn't write the randomized game: the disk is full.".to_owned(),
            )),
            _ => None,
        }
    }

    pub fn bdsp_run_preview(&self) -> Option<RunState> {
        match self.dialog.as_deref()? {
            "bdsp-progress" => Some(RunState::Running(RunProgress {
                fraction: PREVIEW_PROGRESS,
                label: "Randomizing trainers",
                detail: Some("Trainer 214 of 856".to_owned()),
            })),
            "bdsp-result" => {
                let directory = self.preview_output("BDSP Mod");
                Some(RunState::Done(Finished {
                    headline: "Your randomized mod is ready",
                    output: directory.join("atmosphere"),
                    log: Some(directory.join(randomon_bdsp::LOG_FILE_NAME)),
                    race_code: None,
                    install_hint: Some(
                        randomon_bdsp::OutputLayout::Atmosphere
                            .install_hint()
                            .to_owned(),
                    ),
                }))
            }
            _ => None,
        }
    }
}

fn variable(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .filter(|value| !value.trim().is_empty())
}
