use iced::widget::{
    Column, button, column, container, responsive, row, scrollable, space, stack, text, text_input,
};
use iced::{Alignment, Element, Fill, Length};

use super::{Message, OPTIONS_SCROLL_ID, Randomon, Status};
use crate::options::{self, CardMatch, catalog};
use crate::rom;
use crate::section::Section;
use crate::support;
use crate::ui::dialogs;
use crate::ui::options_view::{OptionsView, SEMIBOLD};
use crate::ui::style::{self, SPACE_1, SPACE_2, SPACE_3};

const COMPACT_ACTION_BAR_WIDTH: f32 = 1180.0;

impl Randomon {
    pub(super) fn view(&self) -> Element<'_, Message> {
        let content = column![self.rom_card(), self.section_body(), self.action_bar()]
            .width(Fill)
            .height(Fill);
        let base: Element<'_, Message> = row![self.sidebar(), content].height(Fill).into();
        match &self.dialog {
            Some(dialog) => stack![
                base,
                dialogs::modal(
                    dialogs::view(dialog, &self.custom_names).map(Message::Dialog),
                    Message::Dialog(dialogs::DialogMessage::Close),
                )
            ]
            .into(),
            None => base,
        }
    }

    fn options_view(&self) -> OptionsView<'_> {
        OptionsView {
            settings: &self.settings,
            caps: &self.caps,
            starters: &self.starters,
            disclosures: &self.disclosures,
        }
    }

    fn search_results(&self) -> Vec<CardMatch> {
        options::search(&self.search, &self.caps)
    }

    fn is_searching(&self) -> bool {
        !self.search.trim().is_empty()
    }

    fn sidebar(&self) -> Element<'_, Message> {
        let brand = column![
            text("randomon").size(22).font(SEMIBOLD),
            text(format!("v{}", env!("CARGO_PKG_VERSION")))
                .size(12)
                .style(style::muted_text),
        ]
        .spacing(2);

        let search = text_input("Search options", &self.search)
            .on_input(Message::SearchChanged)
            .padding([8, 12])
            .size(14);

        let results = self.is_searching().then(|| self.search_results());
        let navigation = Section::ALL
            .iter()
            .fold(Column::new().spacing(4), |list, &section| {
                let selected = !self.is_searching() && self.section == section;
                let badge: Element<'_, Message> = match &results {
                    Some(results) => match options::matches_per_section(results, section) {
                        0 => space().into(),
                        count => container(text(count.to_string()).size(11))
                            .padding([1, 7])
                            .style(style::count_badge)
                            .into(),
                    },
                    None => space().into(),
                };
                list.push(
                    button(
                        row![
                            container(text(section.glyph()).size(13))
                                .width(18)
                                .center_x(18),
                            text(section.title()).size(14),
                            space::horizontal(),
                            badge,
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

        let tools = button(
            row![
                container(text("✎").size(13)).width(18).center_x(18),
                text(crate::bundle::label("GUI.customNamesEditorMenuItem.text")).size(14),
            ]
            .spacing(10)
            .align_y(Alignment::Center),
        )
        .width(Fill)
        .padding([8, 12])
        .style(style::nav_item(false))
        .on_press(Message::OpenCustomNames);

        let support_links = column![
            text("Enjoying randomon?").size(12).style(style::muted_text),
            support_button(
                support::github_sponsors_icon(),
                "Sponsor on GitHub",
                support::GITHUB_SPONSORS_URL
            ),
            support_button(
                support::ko_fi_icon(),
                "Support on Ko-fi",
                support::KO_FI_URL
            ),
            row![
                link_button("Discord", support::DISCORD_URL),
                link_button("GitHub", support::REPOSITORY_URL),
            ]
            .spacing(4),
        ]
        .spacing(SPACE_1);

        container(
            column![
                brand,
                search,
                navigation,
                tools,
                space::vertical(),
                support_links
            ]
            .spacing(SPACE_2)
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
                text(&loaded.file_name).size(18).font(SEMIBOLD),
                row![
                    container(text(loaded.platform.label()).size(12))
                        .padding([3, 10])
                        .style(style::pill),
                    container(text(self.generation_label()).size(12))
                        .padding([3, 10])
                        .style(style::pill),
                    text(rom::human_size(loaded.size_bytes))
                        .size(12)
                        .style(style::muted_text),
                ]
                .spacing(SPACE_1)
                .align_y(Alignment::Center),
                text(loaded.path.display().to_string())
                    .size(11)
                    .style(style::muted_text),
            ]
            .spacing(6)
            .into(),
            None => column![
                text("No ROM loaded").size(18).font(SEMIBOLD),
                text("Open a ROM or drop one anywhere in the window. Until then, every option is shown so you can prepare settings.")
                    .size(13)
                    .style(style::muted_text),
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
        .spacing(SPACE_1)
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
            container(
                row![details, space::horizontal(), actions]
                    .spacing(SPACE_2)
                    .align_y(Alignment::Center),
            )
            .padding(20)
            .width(Fill)
            .style(style::card),
        )
        .padding(iced::Padding::from([SPACE_2, SPACE_3]).bottom(SPACE_1))
        .into()
    }

    fn generation_label(&self) -> String {
        let detected_by_engine = self
            .rom
            .as_ref()
            .is_some_and(|loaded| loaded.engine.is_some());
        match (self.caps.generation, detected_by_engine) {
            (Some(generation), true) => format!("Generation {generation}"),
            (Some(generation), false) => format!("Generation {generation} · preview"),
            (None, _) => "Any generation".to_owned(),
        }
    }

    fn section_body(&self) -> Element<'_, Message> {
        let header = if self.is_searching() {
            self.search_header()
        } else {
            self.section_header()
        };
        let body = responsive(move |size| {
            let width = size.width - 2.0 * SPACE_3;
            let content = if self.is_searching() {
                self.search_content(width)
            } else {
                self.section_content(width)
            };
            scrollable(
                container(content).padding(iced::Padding::from([0.0, SPACE_3]).bottom(SPACE_3)),
            )
            .id(OPTIONS_SCROLL_ID)
            .height(Fill)
            .into()
        });
        column![
            container(header).padding(iced::Padding::from([SPACE_2, SPACE_3]).top(SPACE_1)),
            body
        ]
        .height(Fill)
        .into()
    }

    fn section_header(&self) -> Element<'_, Message> {
        row![
            column![
                text(self.section.title()).size(26).font(SEMIBOLD),
                text(self.section.subtitle())
                    .size(14)
                    .style(style::muted_text),
            ]
            .spacing(4),
            space::horizontal(),
            button(text("Reset section").size(13))
                .padding([8, 14])
                .style(style::secondary_button)
                .on_press(Message::ResetSection),
        ]
        .align_y(Alignment::Center)
        .into()
    }

    fn search_header(&self) -> Element<'_, Message> {
        let count: usize = self
            .search_results()
            .iter()
            .map(CardMatch::match_count)
            .sum();
        let noun = if count == 1 { "option" } else { "options" };
        row![
            column![
                text(format!("Results for “{}”", self.search.trim()))
                    .size(26)
                    .font(SEMIBOLD),
                text(format!("{count} matching {noun} across all sections"))
                    .size(14)
                    .style(style::muted_text),
            ]
            .spacing(4),
            space::horizontal(),
            button(text("Clear search").size(13))
                .padding([8, 14])
                .style(style::secondary_button)
                .on_press(Message::ClearSearch),
        ]
        .align_y(Alignment::Center)
        .into()
    }

    fn section_content(&self, width: f32) -> Element<'_, Message> {
        let matches: Vec<CardMatch> = catalog::cards_in(self.section)
            .map(|card| CardMatch::whole(card, &self.caps))
            .collect();
        self.options_view()
            .cards(&matches, width, true)
            .map(Message::Option)
    }

    fn search_content(&self, width: f32) -> Element<'_, Message> {
        let results = self.search_results();
        if results.is_empty() {
            return container(
                text("No options match your search.")
                    .size(14)
                    .style(style::muted_text),
            )
            .padding(SPACE_3)
            .width(Fill)
            .into();
        }
        let groups = Section::ALL.iter().filter_map(|section| {
            let section_results: Vec<CardMatch> = results
                .iter()
                .filter(|result| result.card.section == *section)
                .cloned()
                .collect();
            if section_results.is_empty() {
                return None;
            }
            Some(
                column![
                    button(
                        text(section.title())
                            .size(13)
                            .font(SEMIBOLD)
                            .style(style::accent_text)
                    )
                    .padding([2, 0])
                    .style(button::text)
                    .on_press(Message::SectionSelected(*section)),
                    self.options_view()
                        .cards(&section_results, width, false)
                        .map(Message::Option),
                ]
                .spacing(SPACE_1)
                .into(),
            )
        });
        Column::with_children(groups).spacing(SPACE_3).into()
    }

    fn action_bar(&self) -> Element<'_, Message> {
        let settings = row![
            text_input(
                "Load a ROM or paste a settings string",
                &self.settings_string
            )
            .on_input(Message::SettingsStringChanged)
            .padding(10)
            .width(Length::FillPortion(3)),
            button(text("Copy"))
                .padding([10, 14])
                .style(style::secondary_button)
                .on_press_maybe(
                    (!self.settings_string.is_empty()).then_some(Message::CopySettingsString)
                ),
            secondary_action("Load", Some(Message::LoadQuickSettings)),
            secondary_action(
                "Save",
                self.can_save_settings()
                    .then_some(Message::SaveQuickSettings),
            ),
            secondary_action("Preset", Some(Message::LoadPreset)),
        ]
        .spacing(SPACE_1)
        .align_y(Alignment::Center);

        let seed = row![
            text_input("Seed", &self.seed)
                .on_input(Message::SeedChanged)
                .padding(10)
                .width(Length::Fixed(180.0)),
            button(text("🎲"))
                .padding([10, 14])
                .style(style::secondary_button)
                .on_press(Message::RollSeed),
        ]
        .spacing(SPACE_1)
        .align_y(Alignment::Center);

        let randomize = button(text("Randomize").size(16))
            .padding([10, 28])
            .style(style::primary_button)
            .on_press_maybe(
                (self.rom.is_some() && !self.randomizing).then_some(Message::Randomize),
            );

        let status: Element<'_, Message> = match &self.status {
            Some(Status::Info(message)) => text(message).size(12).style(style::muted_text).into(),
            Some(Status::Error(message)) => text(message).size(12).style(text::danger).into(),
            None => space().into(),
        };

        let controls: Element<'_, Message> = if self.window_width < COMPACT_ACTION_BAR_WIDTH {
            column![
                settings,
                row![seed, space::horizontal(), randomize]
                    .spacing(12)
                    .align_y(Alignment::Center),
            ]
            .spacing(SPACE_1)
            .into()
        } else {
            row![settings, seed, randomize]
                .spacing(12)
                .align_y(Alignment::Center)
                .into()
        };

        container(column![controls, status].spacing(SPACE_1))
            .padding([SPACE_2, SPACE_3])
            .width(Fill)
            .style(style::action_bar)
            .into()
    }
}

fn secondary_action(label: &'static str, message: Option<Message>) -> Element<'static, Message> {
    button(text(label))
        .padding([10, 14])
        .style(style::secondary_button)
        .on_press_maybe(message)
        .into()
}

fn link_button(label: &'static str, url: &'static str) -> Element<'static, Message> {
    button(text(label).size(12))
        .padding([4, 8])
        .style(button::text)
        .on_press(Message::OpenLink(url))
        .into()
}

fn support_button<'a>(
    icon: Element<'a, Message>,
    label: &'a str,
    url: &'static str,
) -> Element<'a, Message> {
    button(
        row![icon, text(label).size(13)]
            .spacing(10)
            .align_y(Alignment::Center),
    )
    .width(Fill)
    .padding([8, 12])
    .style(style::support_button)
    .on_press(Message::OpenLink(url))
    .into()
}
