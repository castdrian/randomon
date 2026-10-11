mod game_update;
mod view;

use std::collections::HashSet;
use std::path::PathBuf;

use iced::theme::Mode;
use iced::widget::operation::{self, RelativeOffset};
use iced::{Subscription, Task, Theme, event, window};
use randomon_settings::{CustomNames, Settings};

use crate::bdsp::{self, BdspMessage, BdspSession};
use crate::capabilities::Capabilities;
use crate::debug::DebugBoot;
use crate::engine;
use crate::game_picker;
use crate::keys::{self, KeysRetry};
use crate::options::spec::Action;
use crate::options::{self, effective, rules};
use crate::rom::{self, LoadedRom};
use crate::section::Section;
use crate::settings_files::{self, ImportedSettings};
use crate::support;
use crate::ui::choices::{self, StarterChoice};
use crate::ui::dialogs::{Dialog, DialogMessage, Feedback, GenerationLimitDraft, NamesTab};
use crate::ui::options_view::OptionMessage;
use crate::ui::theme;
use game_update::GameUpdate;

const SEED_SPACE: u64 = 1 << 48;
const OPTIONS_SCROLL_ID: &str = "options-scroll";
const INITIAL_WINDOW_WIDTH: f32 = 1240.0;

pub fn run() -> iced::Result {
    iced::application(Randomon::boot, Randomon::update, Randomon::view)
        .title("randomon")
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
    game_update: Option<GameUpdate>,
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
    bdsp: Option<BdspSession>,
}

#[derive(Debug, Clone)]
enum Status {
    Info(String),
    Error(String),
    KeysNeeded(String, KeysRetry),
}

fn error_status(message: String, retry: KeysRetry) -> Status {
    if keys::needs_keys(&message) {
        Status::KeysNeeded(message, retry)
    } else {
        Status::Error(message)
    }
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
    OpenGame,
    GamePicked(Option<PathBuf>),
    FileDropped(PathBuf),
    OpenGameUpdate,
    GameUpdatePicked(Option<PathBuf>),
    GameUpdateChecked(PathBuf, Result<Option<String>, String>),
    AskToRemoveGameUpdate,
    OpenSettingsSharing,
    SettingsPasted(Option<String>),
    SettingsImported(Option<Result<Box<ImportedSettings>, String>>),
    SettingsExported(Option<Result<PathBuf, String>>),
    SeedChanged(String),
    RollSeed,
    Randomize,
    OutputChosen(Option<PathBuf>),
    Randomized(Result<engine::RandomizedGame, String>),
    OpenLink(&'static str),
    ChooseKeysFile,
    KeysFilePicked(Option<PathBuf>),
    Bdsp(BdspMessage),
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
            bdsp: None,
        };
        state.settings_changed();
        state
    }

    fn boot() -> (Self, Task<Message>) {
        keys::apply_remembered();
        let mut state = Self::new();
        let boot = DebugBoot::from_env();
        let opens_picker = boot.dialog.as_deref() == Some("open-game");
        state.apply_debug_boot(boot);
        let theme = iced::system::theme().map(Message::SystemThemeChanged);
        if opens_picker {
            return (state, Task::batch([theme, Task::done(Message::OpenGame)]));
        }
        (state, theme)
    }

    fn apply_debug_boot(&mut self, boot: DebugBoot) {
        self.generation_override = boot.generation;
        if let Some(path) = boot.rom {
            self.load_game(path);
        }
        if let Some(section) = boot.section {
            self.section = section;
        }
        if let Some(query) = boot.search {
            self.search = query;
        }
        if let Some(version) = boot.update_version {
            self.game_update = Some(GameUpdate::verified(
                PathBuf::from("update.cia"),
                Some(version),
            ));
        }
        self.dialog = match boot.dialog.as_deref() {
            Some("generations") => Some(Dialog::GenerationLimits(GenerationLimitDraft::open(
                self.settings.current_restrictions,
                &self.caps,
            ))),
            Some("names") => Some(Dialog::CustomNames(NamesTab::TrainerNames)),
            Some("settings") => Some(Dialog::SettingsSharing(None)),
            Some("remove-update") => Some(Dialog::ConfirmRemoveUpdate),
            _ => None,
        };
        if boot.dialog.as_deref() == Some("bdsp-settings")
            && let Some(session) = &mut self.bdsp
        {
            let _ = session.update(BdspMessage::OpenSharing);
        }
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
            Message::Dialog(message) => return self.handle_dialog(message),
            Message::OpenCustomNames => {
                self.dialog = Some(Dialog::CustomNames(NamesTab::TrainerNames));
            }
            Message::OpenGame => {
                return Task::perform(game_picker::pick_game(), Message::GamePicked);
            }
            Message::GamePicked(Some(path)) | Message::FileDropped(path) => self.load_game(path),
            Message::GamePicked(None) | Message::GameUpdatePicked(None) => {}
            Message::OpenGameUpdate => {
                return Task::perform(pick_game_update(), Message::GameUpdatePicked);
            }
            Message::GameUpdatePicked(Some(path)) => return self.check_game_update(path),
            Message::GameUpdateChecked(path, result) => self.finish_game_update(path, result),
            Message::AskToRemoveGameUpdate => self.dialog = Some(Dialog::ConfirmRemoveUpdate),
            Message::OpenSettingsSharing => self.dialog = Some(Dialog::SettingsSharing(None)),
            Message::SettingsPasted(contents) => {
                let outcome = self.paste_settings_string(contents.as_deref().unwrap_or_default());
                self.finish_sharing(outcome);
            }
            Message::SettingsImported(None) | Message::SettingsExported(None) => {}
            Message::SettingsImported(Some(result)) => {
                let outcome = result.map(|imported| self.apply_imported(*imported));
                self.finish_sharing(outcome);
            }
            Message::SettingsExported(Some(result)) => {
                let outcome = result.map(|path| {
                    self.status = Some(Status::Info(format!("Saved {}", path.display())));
                });
                self.finish_sharing(outcome);
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
            Message::ChooseKeysFile => {
                return Task::perform(keys::pick_keys_file(), Message::KeysFilePicked);
            }
            Message::KeysFilePicked(None) => {}
            Message::KeysFilePicked(Some(path)) => return self.use_keys_file(path),
            Message::Bdsp(BdspMessage::Close) => self.bdsp = None,
            Message::Bdsp(BdspMessage::OpenGame) => {
                return Task::perform(game_picker::pick_game(), Message::GamePicked);
            }
            Message::Bdsp(message) => {
                if let Some(session) = &mut self.bdsp {
                    return session.update(message).map(Message::Bdsp);
                }
            }
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

    fn handle_dialog(&mut self, message: DialogMessage) -> Task<Message> {
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
            (DialogMessage::CopySettingsString, Some(Dialog::SettingsSharing(feedback))) => {
                *feedback = Some(Feedback::Done(
                    "Settings string copied to the clipboard.".to_owned(),
                ));
                return iced::clipboard::write(self.settings_string.clone());
            }
            (DialogMessage::PasteSettingsString, _) => {
                return iced::clipboard::read().map(Message::SettingsPasted);
            }
            (DialogMessage::ImportSettingsFile, _) => {
                return Task::perform(settings_files::import_settings_file(), |result| {
                    Message::SettingsImported(result.map(|imported| imported.map(Box::new)))
                });
            }
            (DialogMessage::ExportSettingsFile, _) => {
                return Task::perform(
                    settings_files::export_settings_file(self.output_settings()),
                    Message::SettingsExported,
                );
            }
            (DialogMessage::RemoveUpdate, _) => {
                self.game_update = None;
                self.dialog = None;
                self.status = Some(Status::Info(
                    "Update removed. Randomizing now writes a full ROM.".to_owned(),
                ));
            }
            (DialogMessage::Close, _) => self.dialog = None,
            _ => {}
        }
        Task::none()
    }

    fn finish_sharing(&mut self, outcome: Result<(), String>) {
        match outcome {
            Ok(()) => {
                if matches!(self.dialog, Some(Dialog::SettingsSharing(_))) {
                    self.dialog = None;
                }
            }
            Err(error) => match &mut self.dialog {
                Some(Dialog::SettingsSharing(feedback)) => {
                    *feedback = Some(Feedback::Problem(error));
                }
                _ => self.status = Some(Status::Error(error)),
            },
        }
    }

    fn check_game_update(&mut self, path: PathBuf) -> Task<Message> {
        let Some(game) = self.rom.as_ref().and_then(|loaded| loaded.engine.clone()) else {
            self.game_update = Some(GameUpdate::verified(path, None));
            return Task::none();
        };
        self.game_update = Some(GameUpdate::checking(path.clone()));
        Task::perform(
            engine::game_update_version(game, path.clone()),
            move |result| Message::GameUpdateChecked(path.clone(), result),
        )
    }

    fn finish_game_update(&mut self, path: PathBuf, result: Result<Option<String>, String>) {
        if self.game_update.as_ref().map(GameUpdate::path) != Some(path.as_path()) {
            return;
        }
        match result {
            Ok(version) => {
                let update = GameUpdate::verified(path, version);
                self.status = Some(Status::Info(format!(
                    "{} loaded. Randomizing writes a LayeredFS folder.",
                    update.label()
                )));
                self.game_update = Some(update);
            }
            Err(error) => {
                self.game_update = None;
                self.status = Some(error_status(error, KeysRetry::Update(path)));
            }
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
            engine::choose_output(game, loaded.file_name.clone(), self.update_path()),
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
                self.update_path(),
            ),
            Message::Randomized,
        )
    }

    fn update_path(&self) -> Option<PathBuf> {
        self.game_update
            .as_ref()
            .filter(|update| update.is_ready())
            .map(|update| update.path().to_path_buf())
    }

    fn update_is_checking(&self) -> bool {
        self.game_update
            .as_ref()
            .is_some_and(|update| !update.is_ready())
    }

    fn load_game(&mut self, path: PathBuf) {
        if rom::Platform::from_path(&path).is_none()
            && let Some(romfs) = bdsp::romfs_root_containing(&path)
        {
            self.dialog = None;
            self.bdsp = Some(BdspSession::open(romfs));
            return;
        }
        if path.is_dir() {
            self.status = Some(Status::Error(
                "That folder isn't a ROM or a Brilliant Diamond / Shining Pearl romfs.".to_owned(),
            ));
            return;
        }
        match rom::inspect(path.clone()) {
            Ok(loaded) => self.loaded_rom(loaded),
            Err(message) => self.status = Some(error_status(message, KeysRetry::Game(path))),
        }
    }

    fn use_keys_file(&mut self, path: PathBuf) -> Task<Message> {
        let retry = match &self.status {
            Some(Status::KeysNeeded(_, retry)) => Some(retry.clone()),
            _ => None,
        };
        if let Err(error) = keys::use_keys_file(path) {
            self.status = Some(match retry {
                Some(retry) => Status::KeysNeeded(error, retry),
                None => Status::Error(error),
            });
            return Task::none();
        }
        match retry {
            Some(KeysRetry::Game(game)) => self.load_game(game),
            Some(KeysRetry::Update(update)) => return self.check_game_update(update),
            None => self.status = Some(Status::Info("3DS keys file saved.".to_owned())),
        }
        Task::none()
    }

    fn loaded_rom(&mut self, loaded: LoadedRom) {
        self.bdsp = None;
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

    fn paste_settings_string(&mut self, value: &str) -> Result<(), String> {
        if value.trim().is_empty() {
            return Err("The clipboard doesn't contain a settings string.".to_owned());
        }
        let parsed = settings_files::parse_settings_string(value)
            .map_err(|error| format!("That settings string isn't valid: {error}"))?;
        self.apply_settings(parsed.settings, parsed.description);
        if self.settings_string.is_empty() {
            self.settings_string = value.trim().to_owned();
        }
        Ok(())
    }

    fn apply_imported(&mut self, imported: ImportedSettings) {
        match imported {
            ImportedSettings::QuickSettings(parsed) => {
                self.apply_settings(parsed.settings, parsed.description);
            }
            ImportedSettings::Preset(preset) => {
                self.seed = preset.seed.to_string();
                let description = format!(
                    "Preset loaded with seed {}. It needs {}.",
                    preset.seed, preset.rom_name
                );
                self.apply_settings(preset.settings, description);
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
        app.load_game(path);
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
        target.paste_settings_string(&text).expect("valid string");
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
        app.paste_settings_string(&source.settings_string)
            .expect("valid string");
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
        let before = app.settings_string.clone();
        assert!(app.paste_settings_string("322garbage").is_err());
        assert!(app.paste_settings_string("   ").is_err());
        assert!(app.settings.race_mode);
        assert_eq!(app.settings_string, before);
    }

    #[test]
    fn failed_pastes_stay_in_the_sharing_dialog() {
        let mut app = app_with_rom("gba");
        app.dialog = Some(Dialog::SettingsSharing(None));
        let _ = app.update(Message::SettingsPasted(Some("322garbage".to_owned())));
        assert!(matches!(
            app.dialog,
            Some(Dialog::SettingsSharing(Some(Feedback::Problem(_))))
        ));
        let valid = app.settings_string.clone();
        let _ = app.update(Message::SettingsPasted(Some(valid)));
        assert!(app.dialog.is_none());
    }

    #[test]
    fn a_file_inside_a_bdsp_romfs_opens_the_bdsp_panel() {
        let romfs = std::env::temp_dir().join("randomon-app-tests/bdsp-pick/romfs");
        let data = romfs.join("Data/StreamingAssets/AssetAssistant");
        for bundle in [
            "Dpr/masterdatas",
            "Pml/personal_masterdatas",
            "Dpr/scriptableobjects/gamesettings",
            "Dpr/ev_script",
            "UnderGround/data/ugdata",
            "Message/english",
        ] {
            let path = data.join(bundle);
            std::fs::create_dir_all(path.parent().expect("parent")).expect("bundle dir");
            std::fs::write(&path, []).expect("bundle file");
        }
        let mut app = app_with_rom("gba");
        app.load_game(data.join("Dpr/masterdatas"));
        assert!(app.bdsp.is_some());
        app.load_game(std::env::temp_dir().join("randomon-app-tests/Test Game.gba"));
        assert!(app.bdsp.is_none());
    }

    #[test]
    fn removing_the_update_asks_first_and_ignores_stale_checks() {
        let mut app = app_with_rom("3ds");
        app.game_update = Some(GameUpdate::checking(PathBuf::from("a.cia")));
        let _ = app.update(Message::GameUpdateChecked(
            PathBuf::from("b.cia"),
            Ok(Some("1.4".to_owned())),
        ));
        assert!(app.update_is_checking());
        let _ = app.update(Message::GameUpdateChecked(
            PathBuf::from("a.cia"),
            Ok(Some("1.4".to_owned())),
        ));
        assert_eq!(app.update_path(), Some(PathBuf::from("a.cia")));
        let _ = app.update(Message::AskToRemoveGameUpdate);
        assert_eq!(app.dialog, Some(Dialog::ConfirmRemoveUpdate));
        let _ = app.update(Message::Dialog(DialogMessage::Close));
        assert!(app.game_update.is_some());
        let _ = app.update(Message::AskToRemoveGameUpdate);
        let _ = app.update(Message::Dialog(DialogMessage::RemoveUpdate));
        assert!(app.game_update.is_none());
        assert!(app.dialog.is_none());
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
