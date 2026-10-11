use std::fmt;
use std::path::{Path, PathBuf};

use iced::widget::{
    Column, button, column, container, pick_list, row, rule, scrollable, slider, space, text,
    toggler,
};
use iced::{Alignment, Element, Fill, Length, Task};
use randomon_bdsp::{
    BdspSettings, EVOLUTION_MODES, Game, LEARNSET_MODES, LEVEL_PERCENT_RANGE, ModeLabel,
    OutputLayout, SPEED_PERCENT_RANGE, STARTER_MODES, TM_COMPATIBILITY_MODES, TRAINER_MODES,
    WILD_MODES, detect_games, looks_like_bdsp_romfs, randomize_romfs,
};

use crate::ui::action_bar;
use crate::ui::dialogs::{self, Feedback};
use crate::ui::style::{self, SPACE_1, SPACE_2, SPACE_3};

const SEED_SPACE: u64 = 1 << 48;

fn is_bdsp_romfs(path: &Path) -> bool {
    path.is_dir() && looks_like_bdsp_romfs(path)
}

pub fn romfs_root_containing(path: &Path) -> Option<PathBuf> {
    let innermost = path.ancestors().find(|ancestor| is_bdsp_romfs(ancestor))?;
    let root = innermost
        .parent()
        .filter(|parent| is_bdsp_romfs(parent))
        .unwrap_or(innermost);
    Some(root.to_path_buf())
}

#[derive(Debug, Clone)]
pub struct BdspSession {
    romfs: PathBuf,
    games: Vec<Game>,
    settings: BdspSettings,
    settings_string: String,
    seed: String,
    layout: OutputLayout,
    status: Option<Result<String, String>>,
    busy: bool,
    sharing: Option<Option<Feedback>>,
}

type Flag = fn(&mut BdspSettings) -> &mut bool;
type Number = fn(&mut BdspSettings) -> &mut i32;
type Apply = fn(&mut BdspSettings, usize);

#[derive(Debug, Clone)]
pub enum BdspMessage {
    Toggle(Flag, bool),
    SetNumber(Number, i32),
    SetMode(Apply, usize),
    OpenSharing,
    CloseSharing,
    CopySettingsString,
    PasteSettingsString,
    SettingsPasted(Option<String>),
    OpenGame,
    SeedChanged(String),
    RollSeed,
    LayoutSelected(LayoutChoice),
    Randomize,
    OutputChosen(Option<PathBuf>),
    Finished(Result<String, String>),
    Close,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LayoutChoice(OutputLayout);

impl fmt::Display for LayoutChoice {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.0.label())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModeChoice {
    index: usize,
    label: &'static str,
}

impl fmt::Display for ModeChoice {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.label)
    }
}

fn mode_choices<T: ModeLabel>(all: &[T]) -> Vec<ModeChoice> {
    all.iter()
        .enumerate()
        .map(|(index, mode)| ModeChoice {
            index,
            label: mode.label(),
        })
        .collect()
}

fn selected_choice<T: ModeLabel>(all: &[T], value: T) -> Option<ModeChoice> {
    all.iter()
        .position(|&mode| mode == value)
        .map(|index| ModeChoice {
            index,
            label: value.label(),
        })
}

fn random_seed() -> u64 {
    let mut bytes = [0u8; 8];
    if getrandom::fill(&mut bytes).is_err() {
        return 0;
    }
    u64::from_le_bytes(bytes) % SEED_SPACE
}

impl BdspSession {
    pub fn open(romfs: PathBuf) -> Self {
        let games = detect_games(&romfs);
        let settings = BdspSettings::default();
        Self {
            settings_string: settings.to_settings_string(),
            romfs,
            games,
            settings,
            seed: random_seed().to_string(),
            layout: OutputLayout::default(),
            status: None,
            busy: false,
            sharing: None,
        }
    }

    fn settings_changed(&mut self) {
        self.settings_string = self.settings.to_settings_string();
    }

    pub fn update(&mut self, message: BdspMessage) -> Task<BdspMessage> {
        match message {
            BdspMessage::Toggle(flag, value) => {
                *flag(&mut self.settings) = value;
                self.settings_changed();
            }
            BdspMessage::SetNumber(number, value) => {
                *number(&mut self.settings) = value;
                self.settings_changed();
            }
            BdspMessage::SetMode(apply, index) => {
                apply(&mut self.settings, index);
                self.settings_changed();
            }
            BdspMessage::OpenSharing => self.sharing = Some(None),
            BdspMessage::CloseSharing => self.sharing = None,
            BdspMessage::CopySettingsString => {
                self.sharing = Some(Some(Feedback::Done(
                    "Settings string copied to the clipboard.".to_owned(),
                )));
                return iced::clipboard::write(self.settings_string.clone());
            }
            BdspMessage::PasteSettingsString => {
                return iced::clipboard::read().map(BdspMessage::SettingsPasted);
            }
            BdspMessage::SettingsPasted(contents) => {
                self.paste_settings_string(contents.as_deref().unwrap_or_default());
            }
            BdspMessage::SeedChanged(value) => {
                if value.chars().all(|character| character.is_ascii_digit()) {
                    self.seed = value;
                }
            }
            BdspMessage::RollSeed => self.seed = random_seed().to_string(),
            BdspMessage::LayoutSelected(choice) => self.layout = choice.0,
            BdspMessage::Randomize => {
                if self.seed.trim().parse::<u64>().is_err() {
                    self.status = Some(Err("The seed has to be a whole number.".to_owned()));
                    return Task::none();
                }
                return Task::perform(choose_output_folder(), BdspMessage::OutputChosen);
            }
            BdspMessage::OutputChosen(None) | BdspMessage::Close | BdspMessage::OpenGame => {}
            BdspMessage::OutputChosen(Some(output)) => {
                let Ok(seed) = self.seed.trim().parse::<u64>() else {
                    return Task::none();
                };
                self.busy = true;
                self.status = Some(Ok("Randomizing…".to_owned()));
                return Task::perform(
                    randomize(
                        self.romfs.clone(),
                        self.settings.clone(),
                        seed,
                        output,
                        self.layout,
                        self.games.clone(),
                    ),
                    BdspMessage::Finished,
                );
            }
            BdspMessage::Finished(result) => {
                self.busy = false;
                self.status = Some(result);
            }
        }
        Task::none()
    }

    fn paste_settings_string(&mut self, value: &str) {
        let value = value.trim();
        if value.is_empty() {
            self.sharing = Some(Some(Feedback::Problem(
                "The clipboard doesn't contain a settings string.".to_owned(),
            )));
            return;
        }
        match BdspSettings::from_settings_string(value) {
            Ok(settings) => {
                self.settings = settings;
                self.settings_changed();
                self.sharing = None;
                self.status = Some(Ok("Settings string applied.".to_owned()));
            }
            Err(error) => {
                self.sharing = Some(Some(Feedback::Problem(format!(
                    "That settings string isn't valid: {error}"
                ))));
            }
        }
    }

    pub fn dialog(&self) -> Option<Element<'_, BdspMessage>> {
        let feedback = self.sharing.as_ref()?;
        let mut content = column![
            dialogs::heading(
                "Settings",
                "Share your BDSP options or keep them for later. The seed isn't part of the settings string."
            ),
            dialogs::settings_string_field(
                &self.settings_string,
                "",
                Some(BdspMessage::CopySettingsString),
            ),
            rule::horizontal(1),
            dialogs::sharing_action(
                "Paste settings string",
                "Apply a BDSP settings string from your clipboard.",
                "Paste",
                Some(BdspMessage::PasteSettingsString),
            ),
        ]
        .spacing(SPACE_2);
        if let Some(feedback) = feedback {
            content = content.push(dialogs::feedback_note(feedback));
        }
        content = content.push(dialogs::done_footer(BdspMessage::CloseSharing));
        Some(dialogs::frame(content, 540.0))
    }

    pub fn view(&self, compact: bool) -> Element<'_, BdspMessage> {
        let cards = column![
            self.header(),
            self.starters_card(),
            self.wild_card(),
            self.trainers_card(),
            self.pokemon_card(),
            self.items_card(),
            self.tweaks_card(),
        ]
        .spacing(SPACE_2)
        .padding(SPACE_3);
        column![scrollable(cards).height(Fill), self.action_bar(compact)]
            .height(Fill)
            .width(Fill)
            .into()
    }

    fn header(&self) -> Element<'_, BdspMessage> {
        let games: Vec<&str> = self.games.iter().map(|game| game.name()).collect();
        container(
            row![
                column![
                    text("Brilliant Diamond / Shining Pearl").size(18),
                    text(format!("{} · {}", games.join(" / "), self.romfs.display()))
                        .size(12)
                        .style(style::muted_text),
                ]
                .spacing(6),
                space::horizontal(),
                button(text("Open game"))
                    .padding([8, 16])
                    .style(style::primary_button)
                    .on_press(BdspMessage::OpenGame),
                button(text("Close"))
                    .padding([8, 16])
                    .style(style::secondary_button)
                    .on_press(BdspMessage::Close),
            ]
            .spacing(SPACE_1)
            .align_y(Alignment::Center),
        )
        .padding(SPACE_2)
        .style(style::card)
        .into()
    }

    fn starters_card(&self) -> Element<'_, BdspMessage> {
        card(
            "Starters",
            vec![
                mode_row(
                    "Starter Pokémon",
                    &STARTER_MODES,
                    self.settings.starters,
                    |s, i| {
                        s.starters = STARTER_MODES[i];
                    },
                ),
                flag_row(
                    "Starters have different types",
                    self.settings.starters_distinct_types,
                    |s| &mut s.starters_distinct_types,
                ),
            ],
        )
    }

    fn wild_card(&self) -> Element<'_, BdspMessage> {
        let generations = (0..4).map(|index| {
            let flag: Flag = match index {
                0 => |s| &mut s.wild_generations[0],
                1 => |s| &mut s.wild_generations[1],
                2 => |s| &mut s.wild_generations[2],
                _ => |s| &mut s.wild_generations[3],
            };
            flag_row_owned(
                format!("Allow generation {} Pokémon", index + 1),
                self.settings.wild_generations[index],
                flag,
            )
        });
        let mut rows = vec![
            mode_row("Wild Pokémon", &WILD_MODES, self.settings.wild, |s, i| {
                s.wild = WILD_MODES[i];
            }),
            flag_row(
                "Similar strength",
                self.settings.wild_similar_strength,
                |s| &mut s.wild_similar_strength,
            ),
            flag_row(
                "Legendaries stay legendary",
                self.settings.wild_keep_legendaries,
                |s| &mut s.wild_keep_legendaries,
            ),
            flag_row(
                "Allow legendaries in the wild",
                self.settings.wild_allow_legendaries,
                |s| &mut s.wild_allow_legendaries,
            ),
            flag_row("Randomize the Great Marsh", self.settings.safari, |s| {
                &mut s.safari
            }),
            flag_row(
                "Randomize the Grand Underground",
                self.settings.underground,
                |s| &mut s.underground,
            ),
            number_row(
                "Wild level change (%)",
                self.settings.wild_level_percent,
                LEVEL_PERCENT_RANGE,
                |s| &mut s.wild_level_percent,
            ),
        ];
        rows.extend(generations);
        card("Wild Pokémon", rows)
    }

    fn trainers_card(&self) -> Element<'_, BdspMessage> {
        card(
            "Trainers",
            vec![
                mode_row(
                    "Trainer Pokémon",
                    &TRAINER_MODES,
                    self.settings.trainers,
                    |s, i| {
                        s.trainers = TRAINER_MODES[i];
                    },
                ),
                flag_row(
                    "Gym Leaders and Elite Four keep their type",
                    self.settings.trainer_type_themes,
                    |s| &mut s.trainer_type_themes,
                ),
                flag_row(
                    "Rival uses your randomized starters",
                    self.settings.trainer_rival_carries_starter,
                    |s| &mut s.trainer_rival_carries_starter,
                ),
                flag_row(
                    "Bosses have perfect IVs",
                    self.settings.trainer_max_ivs,
                    |s| &mut s.trainer_max_ivs,
                ),
                number_row(
                    "Trainer level change (%)",
                    self.settings.trainer_level_percent,
                    LEVEL_PERCENT_RANGE,
                    |s| &mut s.trainer_level_percent,
                ),
            ],
        )
    }

    fn pokemon_card(&self) -> Element<'_, BdspMessage> {
        card(
            "Pokémon data",
            vec![
                mode_row(
                    "Level-up moves",
                    &LEARNSET_MODES,
                    self.settings.learnsets,
                    |s, i| {
                        s.learnsets = LEARNSET_MODES[i];
                    },
                ),
                flag_row("Randomize TM moves", self.settings.tms, |s| &mut s.tms),
                mode_row(
                    "TM compatibility",
                    &TM_COMPATIBILITY_MODES,
                    self.settings.tm_compatibility,
                    |s, i| {
                        s.tm_compatibility = TM_COMPATIBILITY_MODES[i];
                    },
                ),
                flag_row("Randomize abilities", self.settings.abilities, |s| {
                    &mut s.abilities
                }),
                flag_row(
                    "Abilities follow evolutions",
                    self.settings.abilities_follow_evolutions,
                    |s| &mut s.abilities_follow_evolutions,
                ),
                mode_row(
                    "Evolutions",
                    &EVOLUTION_MODES,
                    self.settings.evolutions,
                    |s, i| {
                        s.evolutions = EVOLUTION_MODES[i];
                    },
                ),
            ],
        )
    }

    fn items_card(&self) -> Element<'_, BdspMessage> {
        card(
            "Items",
            vec![
                flag_row(
                    "Randomize field items (keeps TMs and key items)",
                    self.settings.field_items,
                    |s| &mut s.field_items,
                ),
                flag_row("Randomize shops", self.settings.shops, |s| &mut s.shops),
                flag_row(
                    "Keep the standard Poké Mart stock",
                    self.settings.shops_keep_basics,
                    |s| &mut s.shops_keep_basics,
                ),
            ],
        )
    }

    fn tweaks_card(&self) -> Element<'_, BdspMessage> {
        card(
            "Tweaks",
            vec![
                flag_row(
                    "HG/SS-style following speeds",
                    self.settings.hgss_following,
                    |s| &mut s.hgss_following,
                ),
                flag_row("True-size Pokémon", self.settings.true_size, |s| {
                    &mut s.true_size
                }),
                flag_row(
                    "Remove fog and disruptive weather",
                    self.settings.remove_weather,
                    |s| &mut s.remove_weather,
                ),
                flag_row("Bicycle everywhere", self.settings.bike_everywhere, |s| {
                    &mut s.bike_everywhere
                }),
                flag_row("60 FPS mode", self.settings.sixty_fps, |s| &mut s.sixty_fps),
                number_row(
                    "Game speed (%)",
                    self.settings.speed_multiplier_percent,
                    SPEED_PERCENT_RANGE,
                    |s| &mut s.speed_multiplier_percent,
                ),
            ],
        )
    }

    fn action_bar(&self, compact: bool) -> Element<'_, BdspMessage> {
        let layouts: Vec<LayoutChoice> = OutputLayout::ALL.into_iter().map(LayoutChoice).collect();
        let status: Option<Element<'_, BdspMessage>> = match &self.status {
            Some(Ok(message)) => Some(
                text(message.clone())
                    .size(12)
                    .style(style::muted_text)
                    .into(),
            ),
            Some(Err(message)) => Some(text(message.clone()).size(12).style(text::danger).into()),
            None => None,
        };
        let randomize = button(text(if self.busy {
            "Randomizing…"
        } else {
            "Randomize"
        }))
        .padding([10, 28])
        .style(style::primary_button)
        .on_press_maybe((!self.busy).then_some(BdspMessage::Randomize));
        let output = row![
            text("Output").size(13).style(style::muted_text),
            pick_list(
                layouts,
                Some(LayoutChoice(self.layout)),
                BdspMessage::LayoutSelected
            )
            .padding([9, 12]),
        ]
        .spacing(SPACE_1)
        .align_y(Alignment::Center);
        action_bar::bar(
            status,
            vec![
                action_bar::settings_button(BdspMessage::OpenSharing),
                output.into(),
                action_bar::seed_controls(
                    &self.seed,
                    BdspMessage::SeedChanged,
                    BdspMessage::RollSeed,
                ),
                randomize.into(),
            ],
            compact,
        )
    }
}

fn card<'a>(title: &'a str, rows: Vec<Element<'a, BdspMessage>>) -> Element<'a, BdspMessage> {
    let body = rows
        .into_iter()
        .fold(Column::new().spacing(SPACE_1), Column::push);
    container(column![text(title).size(16), body].spacing(SPACE_2))
        .padding(SPACE_2)
        .width(Fill)
        .style(style::card)
        .into()
}

fn flag_row<'a>(label: &'a str, value: bool, flag: Flag) -> Element<'a, BdspMessage> {
    flag_row_owned(label.to_owned(), value, flag)
}

fn flag_row_owned<'a>(label: String, value: bool, flag: Flag) -> Element<'a, BdspMessage> {
    toggler(value)
        .label(label)
        .text_size(14)
        .size(18)
        .on_toggle(move |enabled| BdspMessage::Toggle(flag, enabled))
        .into()
}

fn mode_row<'a, T: ModeLabel>(
    label: &'a str,
    all: &[T],
    value: T,
    apply: Apply,
) -> Element<'a, BdspMessage> {
    row![
        text(label).size(14).width(Length::Fixed(220.0)),
        pick_list(
            mode_choices(all),
            selected_choice(all, value),
            move |choice: ModeChoice| { BdspMessage::SetMode(apply, choice.index) }
        )
        .width(Fill),
    ]
    .spacing(SPACE_1)
    .align_y(Alignment::Center)
    .into()
}

fn number_row<'a>(
    label: &'a str,
    value: i32,
    range: std::ops::RangeInclusive<i32>,
    number: Number,
) -> Element<'a, BdspMessage> {
    row![
        text(label).size(14).width(Length::Fixed(220.0)),
        slider(range, value, move |value| BdspMessage::SetNumber(
            number, value
        ))
        .step(5)
        .width(Fill),
        container(text(value.to_string()).size(13)).width(64),
    ]
    .spacing(SPACE_1)
    .align_y(Alignment::Center)
    .into()
}

async fn choose_output_folder() -> Option<PathBuf> {
    rfd::AsyncFileDialog::new()
        .set_title("Choose where to write the mod folder")
        .pick_folder()
        .await
        .map(|handle| handle.path().to_path_buf())
}

async fn randomize(
    romfs: PathBuf,
    settings: BdspSettings,
    seed: u64,
    output: PathBuf,
    layout: OutputLayout,
    games: Vec<Game>,
) -> Result<String, String> {
    tokio::task::spawn_blocking(move || {
        crate::engine::guarded(|| randomize_blocking(romfs, settings, seed, output, layout, games))?
    })
    .await
    .map_err(|error| error.to_string())?
}

fn randomize_blocking(
    romfs: PathBuf,
    settings: BdspSettings,
    seed: u64,
    output: PathBuf,
    layout: OutputLayout,
    games: Vec<Game>,
) -> Result<String, String> {
    let randomized = randomize_romfs(&romfs, &settings, seed).map_err(|error| error.to_string())?;
    let written = randomized
        .write_to(&output, layout, &games)
        .map_err(|error| error.to_string())?;
    Ok(format!(
        "Wrote {} files for {} to {} (seed {seed}).",
        written.len(),
        layout.label(),
        output.display()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_the_romfs_root_from_any_file_inside_it() {
        let romfs = std::env::temp_dir().join("randomon-bdsp-root-test/0100000011D90000/romfs");
        let assets = romfs.join("Data/StreamingAssets/AssetAssistant");
        for bundle in [
            "Dpr/masterdatas",
            "Pml/personal_masterdatas",
            "Dpr/scriptableobjects/gamesettings",
            "Dpr/ev_script",
            "UnderGround/data/ugdata",
            "Message/english",
        ] {
            let path = assets.join(bundle);
            std::fs::create_dir_all(path.parent().expect("parent")).expect("bundle dir");
            std::fs::write(&path, []).expect("bundle file");
        }
        assert_eq!(
            romfs_root_containing(&assets.join("Message/english")),
            Some(romfs.clone())
        );
        assert_eq!(romfs_root_containing(&romfs), Some(romfs.clone()));
        assert_eq!(
            romfs_root_containing(&romfs.join("Data")),
            Some(romfs.clone())
        );
        assert_eq!(romfs_root_containing(&std::env::temp_dir()), None);
    }

    #[test]
    fn pasting_a_bad_string_keeps_the_dialog_open_with_an_error() {
        let mut session = BdspSession::open(std::env::temp_dir());
        let _ = session.update(BdspMessage::OpenSharing);
        let _ = session.update(BdspMessage::SettingsPasted(Some("nonsense".to_owned())));
        assert!(matches!(session.sharing, Some(Some(Feedback::Problem(_)))));
        let valid = BdspSettings {
            safari: !BdspSettings::default().safari,
            ..BdspSettings::default()
        }
        .to_settings_string();
        let _ = session.update(BdspMessage::SettingsPasted(Some(valid.clone())));
        assert!(session.sharing.is_none());
        assert_eq!(session.settings_string, valid);
    }
}
