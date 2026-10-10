use std::path::PathBuf;

use iced::theme::Mode;
use iced::widget::{
    Column, button, center, column, container, row, scrollable, space, text, text_input,
};
use iced::{Alignment, Element, Fill, Length, Subscription, Task, Theme, event, window};

use crate::rom::{self, LoadedRom};
use crate::section::Section;
use crate::support;
use crate::ui::{style, theme};

const SEED_SPACE: u64 = 1 << 48;

pub fn run() -> iced::Result {
    iced::application(Randomon::boot, Randomon::update, Randomon::view)
        .title("Randomon")
        .theme(Randomon::theme)
        .subscription(Randomon::subscription)
        .window(window_settings())
        .centered()
        .run()
}

#[derive(Default)]
struct Randomon {
    mode: Mode,
    section: Option<Section>,
    rom: Option<LoadedRom>,
    game_update: Option<PathBuf>,
    settings_string: String,
    seed: String,
    status: Option<Status>,
}

#[derive(Debug, Clone)]
enum Status {
    Info(String),
    Error(String),
}

#[derive(Debug, Clone)]
enum Message {
    SystemThemeChanged(Mode),
    SectionSelected(Section),
    OpenRom,
    RomPicked(Option<PathBuf>),
    FileDropped(PathBuf),
    OpenGameUpdate,
    GameUpdatePicked(Option<PathBuf>),
    SettingsStringChanged(String),
    CopySettingsString,
    SeedChanged(String),
    RollSeed,
    Randomize,
    OpenLink(&'static str),
}

impl Randomon {
    fn boot() -> (Self, Task<Message>) {
        let state = Self {
            section: Some(Section::PokemonTraits),
            seed: random_seed().to_string(),
            ..Self::default()
        };
        (
            state,
            iced::system::theme().map(Message::SystemThemeChanged),
        )
    }

    fn theme(&self) -> Theme {
        theme::for_mode(self.mode)
    }

    fn subscription(&self) -> Subscription<Message> {
        Subscription::batch([
            iced::system::theme_changes().map(Message::SystemThemeChanged),
            event::listen_with(file_drop_message),
        ])
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::SystemThemeChanged(mode) => self.mode = mode,
            Message::SectionSelected(section) => self.section = Some(section),
            Message::OpenRom => return Task::perform(pick_rom(), Message::RomPicked),
            Message::RomPicked(Some(path)) | Message::FileDropped(path) => self.load_rom(path),
            Message::RomPicked(None) | Message::GameUpdatePicked(None) => {}
            Message::OpenGameUpdate => {
                return Task::perform(pick_game_update(), Message::GameUpdatePicked);
            }
            Message::GameUpdatePicked(Some(path)) => self.game_update = Some(path),
            Message::SettingsStringChanged(value) => self.settings_string = value,
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
            Message::Randomize => {
                self.status = Some(Status::Info(
                    "The randomization engine is still being ported. This build only previews the interface."
                        .to_owned(),
                ));
            }
            Message::OpenLink(url) => support::open_link(url),
        }
        Task::none()
    }

    fn load_rom(&mut self, path: PathBuf) {
        match rom::inspect(path) {
            Ok(loaded) => {
                self.status = Some(Status::Info(format!("Loaded {}", loaded.file_name)));
                self.game_update = None;
                self.rom = Some(loaded);
            }
            Err(message) => self.status = Some(Status::Error(message)),
        }
    }

    fn view(&self) -> Element<'_, Message> {
        let content = column![self.rom_card(), self.section_body(), self.action_bar()]
            .spacing(0)
            .width(Fill)
            .height(Fill);
        row![self.sidebar(), content].height(Fill).into()
    }

    fn sidebar(&self) -> Element<'_, Message> {
        let brand = column![
            text("Randomon").size(22),
            text(format!("v{}", env!("CARGO_PKG_VERSION"))).size(12),
        ]
        .spacing(2);

        let navigation = Section::ALL
            .iter()
            .fold(Column::new().spacing(4), |list, &section| {
                let selected = self.section == Some(section);
                list.push(
                    button(
                        row![
                            container(text(section.glyph()).size(13))
                                .width(18)
                                .center_x(18),
                            text(section.title()).size(14)
                        ]
                        .spacing(10)
                        .align_y(Alignment::Center),
                    )
                    .width(Fill)
                    .padding([8, 12])
                    .style(style::nav_item(selected))
                    .on_press(Message::SectionSelected(section)),
                )
            });

        let support_links = column![
            text("Enjoying Randomon?").size(12),
            button(text("♥  Sponsor on GitHub").size(13))
                .width(Fill)
                .padding([8, 12])
                .style(style::support_button)
                .on_press(Message::OpenLink(support::GITHUB_SPONSORS_URL)),
            button(text("☕  Support on Ko-fi").size(13))
                .width(Fill)
                .padding([8, 12])
                .style(style::support_button)
                .on_press(Message::OpenLink(support::KO_FI_URL)),
            row![
                link_button("Discord", support::DISCORD_URL),
                link_button("GitHub", support::REPOSITORY_URL),
                link_button("UPR ZX", support::UPSTREAM_URL),
            ]
            .spacing(4),
        ]
        .spacing(8);

        container(
            column![brand, navigation, space::vertical(), support_links]
                .spacing(24)
                .height(Fill),
        )
        .width(260)
        .height(Fill)
        .padding(20)
        .style(style::sidebar)
        .into()
    }

    fn rom_card(&self) -> Element<'_, Message> {
        let details: Element<'_, Message> = match &self.rom {
            Some(loaded) => column![
                text(&loaded.file_name).size(18),
                row![
                    container(text(loaded.platform.label()).size(12))
                        .padding([3, 10])
                        .style(style::pill),
                    text(rom::human_size(loaded.size_bytes)).size(12),
                ]
                .spacing(10)
                .align_y(Alignment::Center),
                text(loaded.path.display().to_string()).size(11),
            ]
            .spacing(6)
            .into(),
            None => column![
                text("No ROM loaded").size(18),
                text("Open a ROM or drop one anywhere in the window.").size(13),
            ]
            .spacing(6)
            .into(),
        };

        let mut actions = row![
            button(text("Open ROM"))
                .padding([8, 16])
                .style(style::primary_button)
                .on_press(Message::OpenRom)
        ]
        .spacing(8)
        .align_y(Alignment::Center);
        if self
            .rom
            .as_ref()
            .is_some_and(|loaded| loaded.platform.supports_game_updates())
        {
            let label = match &self.game_update {
                Some(_) => "Update loaded",
                None => "Load update (.cia)",
            };
            actions = actions.push(
                button(text(label))
                    .padding([8, 16])
                    .style(style::secondary_button)
                    .on_press(Message::OpenGameUpdate),
            );
        }

        container(
            container(row![details, space::horizontal(), actions].align_y(Alignment::Center))
                .padding(20)
                .width(Fill)
                .style(style::card),
        )
        .padding([20, 24])
        .into()
    }

    fn section_body(&self) -> Element<'_, Message> {
        let section = self.section.unwrap_or(Section::PokemonTraits);
        let heading = column![
            text(section.title()).size(26),
            text(section.subtitle()).size(14)
        ]
        .spacing(4);

        let placeholder: Element<'_, Message> = if self.rom.is_some() {
            container(
                column![
                    text("Options coming soon").size(16),
                    text("The settings for this section appear here as each generation's engine is ported.")
                        .size(13),
                ]
                .spacing(6),
            )
            .padding(20)
            .width(Fill)
            .style(style::card)
            .into()
        } else {
            container(center(
                column![
                    text("Drop a ROM to begin").size(18),
                    text("GB, GBC, GBA, NDS and 3DS games are supported").size(13),
                    button(text("Choose a file"))
                        .padding([8, 16])
                        .style(style::primary_button)
                        .on_press(Message::OpenRom),
                ]
                .spacing(10)
                .align_x(Alignment::Center),
            ))
            .height(220)
            .width(Fill)
            .style(style::drop_zone)
            .into()
        };

        scrollable(column![heading, placeholder].spacing(20).padding([4, 24]))
            .height(Fill)
            .into()
    }

    fn action_bar(&self) -> Element<'_, Message> {
        let settings = row![
            text_input("Paste a settings string", &self.settings_string)
                .on_input(Message::SettingsStringChanged)
                .padding(10)
                .width(Length::FillPortion(3)),
            button(text("Copy"))
                .padding([10, 14])
                .style(style::secondary_button)
                .on_press(Message::CopySettingsString),
        ]
        .spacing(8)
        .align_y(Alignment::Center);

        let seed = row![
            text_input("Seed", &self.seed)
                .on_input(Message::SeedChanged)
                .padding(10)
                .width(Length::Fixed(200.0)),
            button(text("🎲"))
                .padding([10, 14])
                .style(style::secondary_button)
                .on_press(Message::RollSeed),
        ]
        .spacing(8)
        .align_y(Alignment::Center);

        let randomize = button(text("Randomize").size(16))
            .padding([10, 28])
            .style(style::primary_button)
            .on_press_maybe(self.rom.as_ref().map(|_| Message::Randomize));

        let status: Element<'_, Message> = match &self.status {
            Some(Status::Info(message)) => text(message).size(12).into(),
            Some(Status::Error(message)) => text(message).size(12).style(text::danger).into(),
            None => space().into(),
        };

        container(
            column![
                row![settings, seed, randomize]
                    .spacing(12)
                    .align_y(Alignment::Center),
                status,
            ]
            .spacing(8),
        )
        .padding([16, 24])
        .width(Fill)
        .style(style::action_bar)
        .into()
    }
}

fn window_settings() -> window::Settings {
    window::Settings {
        size: iced::Size::new(1180.0, 760.0),
        min_size: Some(iced::Size::new(900.0, 600.0)),
        icon: window::icon::from_file_data(include_bytes!("../../../assets/icon-512.png"), None)
            .ok(),
        ..window::Settings::default()
    }
}

fn link_button(label: &'static str, url: &'static str) -> Element<'static, Message> {
    button(text(label).size(12))
        .padding([4, 8])
        .style(button::text)
        .on_press(Message::OpenLink(url))
        .into()
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
