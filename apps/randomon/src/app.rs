mod view;

use std::collections::HashSet;
use std::path::PathBuf;

use iced::theme::Mode;
use iced::widget::operation::{self, RelativeOffset};
use iced::{Subscription, Task, Theme, event, window};
use randomon_settings::{CustomNames, Settings};

use crate::capabilities::Capabilities;
use crate::debug::DebugBoot;
use crate::engine;
use crate::options::spec::Action;
use crate::options::{self, effective, rules};
use crate::rom::{self, LoadedRom};
use crate::section::Section;
use crate::settings_files::{self, LoadedPreset, ParsedSettings};
use crate::support;
use crate::ui::choices::{self, StarterChoice};
use crate::ui::dialogs::{Dialog, DialogMessage, GenerationLimitDraft, NamesTab};
use crate::ui::options_view::OptionMessage;
use crate::ui::theme;

const SEED_SPACE: u64 = 1 << 48;
const OPTIONS_SCROLL_ID: &str = "options-scroll";
const INITIAL_WINDOW_WIDTH: f32 = 1240.0;

pub fn run() -> iced::Result {
    iced::application(Randomon::boot, Randomon::update, Randomon::view)
        .title("Randomon")
        .theme(Randomon::theme)
        .subscription(Randomon::subscription)
        .window(window_settings())
        .centered()
        .run()
}

struct Randomon {
    mode: Mode,
    window_width: f32,
    section: Section,
    rom: Option<LoadedRom>,
    game_update: Option<PathBuf>,
    settings: Settings,
    caps: Capabilities,
    generation_override: Option<i32>,
    starters: Vec<StarterChoice>,
    settings_string: String,
    seed: String,
    search: String,
    disclosures: HashSet<&'static str>,
    dialog: Option<Dialog>,
    custom_names: CustomNames,
    status: Option<Status>,
    randomizing: bool,
}

#[derive(Debug, Clone)]
enum Status {
    Info(String),
    Error(String),
}

#[derive(Debug, Clone)]
enum Message {
    SystemThemeChanged(Mode),
    WindowResized(f32),
    SectionSelected(Section),
    SearchChanged(String),
    ClearSearch,
    ResetSection,
    Option(OptionMessage),
    Dialog(DialogMessage),
    OpenCustomNames,
    OpenRom,
    RomPicked(Option<PathBuf>),
    FileDropped(PathBuf),
    OpenGameUpdate,
    GameUpdatePicked(Option<PathBuf>),
    SettingsStringChanged(String),
    CopySettingsString,
    LoadQuickSettings,
    QuickSettingsLoaded(Option<Result<ParsedSettings, String>>),
    SaveQuickSettings,
    QuickSettingsSaved(Option<Result<PathBuf, String>>),
    LoadPreset,
    PresetLoaded(Option<Result<LoadedPreset, String>>),
    SeedChanged(String),
    RollSeed,
    Randomize,
    OutputChosen(Option<PathBuf>),
    Randomized(Result<engine::RandomizedGame, String>),
    OpenLink(&'static str),
}

impl Randomon {
    fn new() -> Self {
        let caps = Capabilities::preview();
        let mut state = Self {
            mode: Mode::None,
            window_width: INITIAL_WINDOW_WIDTH,
            section: Section::PokemonTraits,
            rom: None,
            game_update: None,
            settings: Settings::default(),
            caps,
            generation_override: None,
            starters: choices::starter_choices(&caps),
            settings_string: String::new(),
            seed: random_seed().to_string(),
            search: String::new(),
            disclosures: HashSet::new(),
            dialog: None,
            custom_names: CustomNames::default_names(),
            status: None,
            randomizing: false,
        };
        state.settings_changed();
        state
    }

    fn boot() -> (Self, Task<Message>) {
        let mut state = Self::new();
        state.apply_debug_boot(DebugBoot::from_env());
        (
            state,
            iced::system::theme().map(Message::SystemThemeChanged),
        )
    }

    fn apply_debug_boot(&mut self, boot: DebugBoot) {
        self.generation_override = boot.generation;
        if let Some(path) = boot.rom {
            self.load_rom(path);
        }
        if let Some(section) = boot.section {
            self.section = section;
        }
        if let Some(query) = boot.search {
            self.search = query;
        }
        self.dialog = match boot.dialog.as_deref() {
            Some("generations") => Some(Dialog::GenerationLimits(GenerationLimitDraft::open(
                self.settings.current_restrictions,
                &self.caps,
            ))),
            Some("names") => Some(Dialog::CustomNames(NamesTab::TrainerNames)),
            _ => None,
        };
    }

    fn theme(&self) -> Theme {
        theme::for_mode(self.mode)
    }

    fn subscription(&self) -> Subscription<Message> {
        Subscription::batch([
            iced::system::theme_changes().map(Message::SystemThemeChanged),
            event::listen_with(file_drop_message),
            window::resize_events().map(|(_, size)| Message::WindowResized(size.width)),
        ])
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::SystemThemeChanged(mode) => self.mode = mode,
            Message::WindowResized(width) => self.window_width = width,
            Message::SectionSelected(section) => {
                self.section = section;
                self.search.clear();
                return operation::snap_to(OPTIONS_SCROLL_ID, RelativeOffset::START);
            }
            Message::SearchChanged(query) => {
                self.search = query;
                return operation::snap_to(OPTIONS_SCROLL_ID, RelativeOffset::START);
            }
            Message::ClearSearch => self.search.clear(),
            Message::ResetSection => {
                options::reset_section(&mut self.settings, self.section);
                self.settings_changed();
                self.status = Some(Status::Info(format!(
                    "{} reset to defaults",
                    self.section.title()
                )));
            }
            Message::Option(message) => self.handle_option(message),
            Message::Dialog(message) => self.handle_dialog(message),
            Message::OpenCustomNames => {
                self.dialog = Some(Dialog::CustomNames(NamesTab::TrainerNames));
            }
            Message::OpenRom => return Task::perform(pick_rom(), Message::RomPicked),
            Message::RomPicked(Some(path)) | Message::FileDropped(path) => self.load_rom(path),
            Message::RomPicked(None) | Message::GameUpdatePicked(None) => {}
            Message::OpenGameUpdate => {
                return Task::perform(pick_game_update(), Message::GameUpdatePicked);
            }
            Message::GameUpdatePicked(Some(path)) => self.game_update = Some(path),
            Message::SettingsStringChanged(value) => self.paste_settings_string(value),
            Message::LoadQuickSettings => {
                return Task::perform(
                    settings_files::load_quick_settings(),
                    Message::QuickSettingsLoaded,
                );
            }
            Message::QuickSettingsLoaded(None)
            | Message::QuickSettingsSaved(None)
            | Message::PresetLoaded(None) => {}
            Message::QuickSettingsLoaded(Some(Ok(parsed))) => {
                self.apply_settings(parsed.settings, parsed.description);
            }
            Message::SaveQuickSettings => {
                return Task::perform(
                    settings_files::save_quick_settings(self.output_settings()),
                    Message::QuickSettingsSaved,
                );
            }
            Message::QuickSettingsSaved(Some(Ok(path))) => {
                self.status = Some(Status::Info(format!("Saved {}", path.display())));
            }
            Message::LoadPreset => {
                return Task::perform(settings_files::load_preset(), Message::PresetLoaded);
            }
            Message::PresetLoaded(Some(Ok(preset))) => {
                self.seed = preset.seed.to_string();
                let description = format!("Preset loaded. It needs {}.", preset.rom_name);
                self.apply_settings(preset.settings, description);
            }
            Message::QuickSettingsLoaded(Some(Err(error)))
            | Message::QuickSettingsSaved(Some(Err(error)))
            | Message::PresetLoaded(Some(Err(error))) => self.status = Some(Status::Error(error)),
            Message::CopySettingsString => {
                return iced::clipboard::write(self.settings_string.clone());
            }
            Message::SeedChanged(value) => {
                if value
                    .chars()
                    .all(|character| character.is_ascii_digit() || character == '-')
                {
                    self.seed = value;
                }
            }
            Message::RollSeed => self.seed = random_seed().to_string(),
            Message::Randomize => return self.start_randomization(),
            Message::OutputChosen(None) => {}
            Message::OutputChosen(Some(output)) => return self.run_randomization(output),
            Message::Randomized(result) => {
                self.randomizing = false;
                self.status = Some(match result {
                    Ok(done) => Status::Info(match &done.log {
                        Some(log) => format!(
                            "Saved {} and its log {} (check value {:08X})",
                            done.output.display(),
                            log.display(),
                            done.check_value
                        ),
                        None => format!(
                            "Saved {} (check value {:08X})",
                            done.output.display(),
                            done.check_value
                        ),
                    }),
                    Err(error) => Status::Error(format!("Randomization failed: {error}")),
                });
            }
            Message::OpenLink(url) => support::open_link(url),
        }
        Task::none()
    }

    fn handle_option(&mut self, message: OptionMessage) {
        match message {
            OptionMessage::Edit(edit) => {
                edit.apply(&mut self.settings);
                self.settings_changed();
            }
            OptionMessage::Action(Action::EditGenerationLimits) => {
                self.dialog = Some(Dialog::GenerationLimits(GenerationLimitDraft::open(
                    self.settings.current_restrictions,
                    &self.caps,
                )));
            }
            OptionMessage::ToggleDisclosure(card) => {
                if !self.disclosures.remove(card) {
                    self.disclosures.insert(card);
                }
            }
            OptionMessage::Ignore => {}
        }
    }

    fn handle_dialog(&mut self, message: DialogMessage) {
        match (message, &mut self.dialog) {
            (
                DialogMessage::SetGeneration(generation, allowed),
                Some(Dialog::GenerationLimits(draft)),
            ) => {
                draft.set_generation(generation, allowed);
            }
            (DialogMessage::SetRelatives(allowed), Some(Dialog::GenerationLimits(draft))) => {
                draft.set_relatives(allowed);
            }
            (DialogMessage::ConfirmGenerationLimits, Some(Dialog::GenerationLimits(draft))) => {
                self.settings.current_restrictions = Some(draft.restrictions);
                if !rules::restrictions_allow_mega_swaps(&self.settings, &self.caps, true) {
                    self.settings.swap_trainer_mega_evos = false;
                }
                self.dialog = None;
                self.settings_changed();
            }
            (DialogMessage::ShowNames(tab), Some(Dialog::CustomNames(current))) => *current = tab,
            (DialogMessage::Close, _) => self.dialog = None,
            _ => {}
        }
    }

    fn start_randomization(&mut self) -> Task<Message> {
        let Some(loaded) = &self.rom else {
            return Task::none();
        };
        let Some(game) = loaded.engine.clone() else {
            self.status = Some(Status::Error(
                "Randomizing this generation isn't ported yet. Use UPR ZX for it for now."
                    .to_owned(),
            ));
            return Task::none();
        };
        if self.seed.trim().parse::<i64>().is_err() {
            self.status = Some(Status::Error(
                "The seed has to be a whole number.".to_owned(),
            ));
            return Task::none();
        }
        Task::perform(
            engine::choose_output(game, loaded.file_name.clone(), self.game_update.clone()),
            Message::OutputChosen,
        )
    }

    fn run_randomization(&mut self, output: PathBuf) -> Task<Message> {
        let Some(game) = self.rom.as_ref().and_then(|loaded| loaded.engine.clone()) else {
            return Task::none();
        };
        let Ok(seed) = self.seed.trim().parse::<i64>() else {
            return Task::none();
        };
        self.randomizing = true;
        self.status = Some(Status::Info("Randomizing…".to_owned()));
        Task::perform(
            engine::randomize(
                game,
                self.output_settings(),
                seed,
                output,
                self.game_update.clone(),
            ),
            Message::Randomized,
        )
    }

    fn load_rom(&mut self, path: PathBuf) {
        match rom::inspect(path) {
            Ok(loaded) => {
                self.status = Some(Status::Info(format!("Loaded {}", loaded.file_name)));
                self.game_update = None;
                self.caps = match (&loaded.engine, self.generation_override) {
                    (Some(game), _) => game.capabilities,
                    (None, Some(generation)) => Capabilities::for_generation(generation),
                    (None, None) => Capabilities::provisional(loaded.platform),
                };
                self.status = Some(match &loaded.engine {
                    Some(game) => Status::Info(format!(
                        "Loaded {} ({}, generation {}, support: {})",
                        game.rom_name, game.rom_code, game.generation, game.support_level
                    )),
                    None => Status::Info(format!(
                        "Loaded {}. Randomizing this generation isn't ported yet, so its options are a preview.",
                        loaded.file_name
                    )),
                });
                self.starters = choices::starter_choices(&self.caps);
                self.rom = Some(loaded);
                self.limit_restrictions_to_game();
                self.settings_changed();
            }
            Err(message) => self.status = Some(Status::Error(message)),
        }
    }

    fn paste_settings_string(&mut self, value: String) {
        if value.trim().is_empty() {
            self.settings_string = value;
            self.status = None;
            return;
        }
        match settings_files::parse_settings_string(&value) {
            Ok(parsed) => {
                self.settings_string = value;
                self.apply_settings(parsed.settings, parsed.description);
            }
            Err(error) => {
                self.settings_string = value;
                self.status = Some(Status::Error(format!(
                    "That settings string isn't valid: {error}"
                )));
            }
        }
    }

    fn apply_settings(&mut self, settings: Settings, description: String) {
        let mismatch = match (&self.rom, settings.rom_name.as_deref()) {
            (Some(loaded), Some(name)) if name != loaded.rom_name => Some(name.to_owned()),
            _ => None,
        };
        self.settings = settings;
        self.limit_restrictions_to_game();
        self.settings_changed();
        self.status = Some(Status::Info(match mismatch {
            Some(name) => format!(
                "{description}. They were made for {name}, so some options may not apply to this game."
            ),
            None => description,
        }));
    }

    fn limit_restrictions_to_game(&mut self) {
        if let (Some(restrictions), Some(generation)) = (
            self.settings.current_restrictions.as_mut(),
            self.caps.generation,
        ) {
            restrictions.limit_to_gen(generation);
        }
    }

    fn settings_changed(&mut self) {
        rules::normalize(&mut self.settings, &self.caps);
        match self.output_settings().to_versioned_settings_string() {
            Ok(text) => self.settings_string = text,
            Err(error) => log::debug!("settings string unavailable: {error}"),
        }
    }

    fn output_settings(&self) -> Settings {
        effective::settings_for_output(
            &self.settings,
            &self.caps,
            self.rom.as_ref().map(|loaded| loaded.rom_name.as_str()),
        )
    }

    fn can_save_settings(&self) -> bool {
        self.output_settings().rom_name.is_some()
    }
}

fn window_settings() -> window::Settings {
    window::Settings {
        size: iced::Size::new(INITIAL_WINDOW_WIDTH, 820.0),
        min_size: Some(iced::Size::new(900.0, 600.0)),
        icon: window::icon::from_file_data(include_bytes!("../../../assets/icon-512.png"), None)
            .ok(),
        ..window::Settings::default()
    }
}

fn file_drop_message(
    event: iced::Event,
    _status: event::Status,
    _window: window::Id,
) -> Option<Message> {
    match event {
        iced::Event::Window(window::Event::FileDropped(path)) => Some(Message::FileDropped(path)),
        _ => None,
    }
}

async fn pick_rom() -> Option<PathBuf> {
    rfd::AsyncFileDialog::new()
        .set_title("Open a Pokémon ROM")
        .add_filter("Pokémon ROMs", rom::SUPPORTED_EXTENSIONS)
        .pick_file()
        .await
        .map(|handle| handle.path().to_path_buf())
}

async fn pick_game_update() -> Option<PathBuf> {
    rfd::AsyncFileDialog::new()
        .set_title("Open a game update")
        .add_filter("3DS game update", &["cia"])
        .pick_file()
        .await
        .map(|handle| handle.path().to_path_buf())
}

fn random_seed() -> u64 {
    let mut bytes = [0u8; 8];
    if getrandom::fill(&mut bytes).is_err() {
        return 0;
    }
    u64::from_le_bytes(bytes) % SEED_SPACE
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::options::Edit;
    use randomon_settings::{BaseStatisticsMod, MiscTweak, WildPokemonMod};

    fn app_with_rom(extension: &str) -> Randomon {
        let directory = std::env::temp_dir().join("randomon-app-tests");
        std::fs::create_dir_all(&directory).expect("temp dir");
        let path = directory.join(format!("Test Game.{extension}"));
        std::fs::write(&path, [0u8; 16]).expect("rom file");
        let mut app = Randomon::new();
        app.load_rom(path);
        app
    }

    fn edit(app: &mut Randomon, edit: Edit) {
        app.handle_option(OptionMessage::Edit(edit));
    }

    #[test]
    fn controls_update_the_settings_string_live() {
        let mut app = app_with_rom("gba");
        let before = app.settings_string.clone();
        assert!(!before.is_empty());
        edit(
            &mut app,
            Edit::Select(|s| s.base_statistics_mod = BaseStatisticsMod::Random),
        );
        assert_ne!(app.settings_string, before);
        let decoded = Settings::from_versioned_settings_string(&app.settings_string)
            .expect("valid")
            .settings;
        assert_eq!(decoded.base_statistics_mod, BaseStatisticsMod::Random);
        assert_eq!(decoded.rom_name.as_deref(), Some("Test Game"));
    }

    #[test]
    fn pasted_strings_replace_the_state_and_round_trip() {
        let mut source = app_with_rom("nds");
        edit(
            &mut source,
            Edit::Select(|s| s.wild_pokemon_mod = WildPokemonMod::AreaMapping),
        );
        edit(
            &mut source,
            Edit::Flag(|s, v| s.wild_levels_modified = v, true),
        );
        edit(
            &mut source,
            Edit::Number(|s, v| s.wild_level_modifier = v, 24),
        );
        edit(&mut source, Edit::MiscTweak(MiscTweak::FastestText, true));
        let text = source.settings_string.clone();

        let mut target = app_with_rom("nds");
        target.paste_settings_string(text.clone());
        assert_eq!(
            target.settings.wild_pokemon_mod,
            WildPokemonMod::AreaMapping
        );
        assert_eq!(target.settings.wild_level_modifier, 24);
        assert!(MiscTweak::FastestText.is_set_in(target.settings.current_misc_tweaks));
        assert_eq!(target.settings_string, text);
    }

    #[test]
    fn without_a_rom_the_last_valid_string_is_kept() {
        let mut app = Randomon::new();
        assert!(app.settings_string.is_empty());
        edit(&mut app, Edit::Flag(|s, v| s.race_mode = v, true));
        assert!(app.settings_string.is_empty());

        let mut source = app_with_rom("gbc");
        edit(&mut source, Edit::Flag(|s, v| s.race_mode = v, true));
        app.paste_settings_string(source.settings_string.clone());
        assert_eq!(app.settings_string, source.settings_string);
        edit(
            &mut app,
            Edit::Flag(|s, v| s.randomize_move_powers = v, true),
        );
        let decoded = Settings::from_versioned_settings_string(&app.settings_string)
            .expect("valid")
            .settings;
        assert!(decoded.randomize_move_powers);
        assert!(decoded.race_mode);
    }

    #[test]
    fn invalid_pastes_keep_the_previous_settings() {
        let mut app = app_with_rom("gba");
        edit(&mut app, Edit::Flag(|s, v| s.race_mode = v, true));
        app.paste_settings_string("322garbage".to_owned());
        assert!(app.settings.race_mode);
        assert!(matches!(app.status, Some(Status::Error(_))));
        assert_eq!(app.settings_string, "322garbage");
    }

    #[test]
    fn hidden_options_never_reach_the_string() {
        let mut app = app_with_rom("gb");
        edit(
            &mut app,
            Edit::Select(|s| s.trainers_mod = randomon_settings::TrainersMod::Random),
        );
        app.settings.double_battle_mode = true;
        app.settings_changed();
        let decoded = Settings::from_versioned_settings_string(&app.settings_string)
            .expect("valid")
            .settings;
        assert!(!decoded.double_battle_mode);
    }
}
