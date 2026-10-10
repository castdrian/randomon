use std::collections::HashSet;
use std::time::Duration;

use iced::alignment::Horizontal;
use iced::font::Weight;
use iced::widget::{
    Column, button, column, container, grid, pick_list, row, slider, space, text, toggler, tooltip,
};
use iced::{Alignment, Element, Fill, Font, Length};
use randomon_settings::{MiscTweak, Settings};

use crate::capabilities::Capabilities;
use crate::options::spec::{
    Action, Attachment, Card, CardBody, Control, FlagGetter, FlagSetter, Item, Number,
    NumberFormat, NumberStyle, Pick, Radio, Row,
};
use crate::options::{self, CardMatch, Edit};
use crate::ui::choices::{ExpCurveChoice, GenerationChoice, StarterChoice, TrainerModeChoice};
use crate::ui::style::{self, NESTED_INDENT, SPACE_1, SPACE_2};

const TWO_COLUMN_MIN_WIDTH: f32 = 860.0;
const SEGMENT_LABEL_LIMIT: usize = 12;
const TWEAK_CHIP_WIDTH: f32 = 250.0;
const TWEAK_CHIP_HEIGHT: f32 = 108.0;
const TOOLTIP_WIDTH: f32 = 380.0;
const STARTER_PICK_WIDTH: f32 = 220.0;
const STARTER_PLACEHOLDER: &str = "Choose…";

pub const SEMIBOLD: Font = Font {
    weight: Weight::Semibold,
    ..Font::DEFAULT
};

#[derive(Debug, Clone)]
pub enum OptionMessage {
    Edit(Edit),
    Action(Action),
    ToggleDisclosure(&'static str),
    Ignore,
}

type El<'a> = Element<'a, OptionMessage>;

pub struct OptionsView<'a> {
    pub settings: &'a Settings,
    pub caps: &'a Capabilities,
    pub starters: &'a [StarterChoice],
    pub disclosures: &'a HashSet<&'static str>,
}

impl<'a> OptionsView<'a> {
    pub fn cards(&self, matches: &[CardMatch], width: f32, show_disclosures: bool) -> El<'a> {
        let cards: Vec<(f32, El<'a>)> = matches
            .iter()
            .map(|card_match| {
                (
                    self.estimated_height(card_match),
                    self.card(card_match, show_disclosures),
                )
            })
            .collect();
        if width < TWO_COLUMN_MIN_WIDTH || cards.len() < 2 {
            return Column::with_children(cards.into_iter().map(|(_, card)| card))
                .spacing(SPACE_2)
                .into();
        }
        two_columns(cards)
    }

    fn card(&self, card_match: &CardMatch, show_disclosure: bool) -> El<'a> {
        let card = card_match.card;
        if !card.is_visible(self.caps) {
            return unavailable_card(card);
        }
        let body = match card.body {
            CardBody::Rows => self.rows(&card_match.rows),
            CardBody::MiscTweaks => self.tweaks(&card_match.tweaks, card),
        };
        let mut content = column![card_header(card), body].spacing(SPACE_2);
        if show_disclosure && let Some(disclosure) = self.disclosure(card) {
            content = content.push(disclosure);
        }
        container(content)
            .padding(20)
            .width(Fill)
            .style(style::card)
            .into()
    }

    fn rows(&self, rows: &[&'static Row]) -> El<'a> {
        Column::with_children(
            rows.iter()
                .filter(|row| row.is_visible(self.caps))
                .map(|row| self.row(row)),
        )
        .spacing(SPACE_1)
        .into()
    }

    fn row(&self, row: &'static Row) -> El<'a> {
        match row {
            Row::Item(item) => self.item(item),
            Row::Radio(radio) => self.radio(radio),
            Row::Caption { text: caption, .. } => container(
                text(caption.label())
                    .size(12)
                    .font(SEMIBOLD)
                    .style(style::muted_text),
            )
            .padding(iced::Padding::default().top(SPACE_1))
            .into(),
        }
    }

    fn item(&self, item: &'static Item) -> El<'a> {
        let enabled = item.is_enabled(self.settings, self.caps);
        let content = match item.control {
            Control::Toggle {
                get,
                set,
                attachment,
            } => self.toggle(item, enabled, get, set, attachment),
            Control::Choice { .. } => self.choice_row(item),
            Control::Pick(pick) => row![
                text(item.label.label()).size(14),
                space::horizontal(),
                self.pick(pick, enabled)
            ]
            .spacing(SPACE_1)
            .align_y(Alignment::Center)
            .into(),
        };
        nest(content, item.nested)
    }

    fn toggle(
        &self,
        item: &'static Item,
        enabled: bool,
        get: FlagGetter,
        set: FlagSetter,
        attachment: Option<Attachment>,
    ) -> El<'a> {
        let value = get(self.settings);
        let switch = toggler(value)
            .label(item.label.label())
            .text_size(14)
            .size(18)
            .spacing(10)
            .style(style::toggle_switch)
            .on_toggle_maybe(
                enabled.then_some(move |value| OptionMessage::Edit(Edit::Flag(set, value))),
            );
        let switch = with_help(switch, item.help_text());
        let attachment_enabled = enabled && value;
        let line: El<'a> = match attachment {
            Some(Attachment::Pick(pick)) => row![
                switch,
                space::horizontal(),
                self.pick(pick, attachment_enabled)
            ]
            .spacing(SPACE_1)
            .align_y(Alignment::Center)
            .into(),
            Some(Attachment::Action(action)) => row![
                switch,
                space::horizontal(),
                self.action(action, attachment_enabled)
            ]
            .spacing(SPACE_1)
            .align_y(Alignment::Center)
            .into(),
            Some(Attachment::Number(_)) | None => switch,
        };
        match attachment {
            Some(Attachment::Number(number)) => {
                column![line, nest(self.number(number, attachment_enabled), true)]
                    .spacing(SPACE_1)
                    .into()
            }
            _ => line,
        }
    }

    fn number(&self, number: Number, enabled: bool) -> El<'a> {
        let value = number.displayed(self.settings);
        let set = number.set;
        let badge = container(text(format_number(number.format, value)).size(12))
            .padding([2, 8])
            .style(style::value_badge(enabled));
        let control: El<'a> = match number.style {
            NumberStyle::Slider => row![
                slider(number.min..=number.max, value, move |value| {
                    if enabled {
                        OptionMessage::Edit(Edit::Number(set, value))
                    } else {
                        OptionMessage::Ignore
                    }
                })
                .step(number.step)
                .style(style::range_slider(enabled))
                .width(Fill),
                container(badge).width(64).align_x(Horizontal::Right)
            ]
            .spacing(12)
            .align_y(Alignment::Center)
            .into(),
            NumberStyle::Stepper => row![
                step_button(
                    "−",
                    (enabled && value > number.min).then_some(value - number.step),
                    set
                ),
                container(badge).width(40).align_x(Horizontal::Center),
                step_button(
                    "+",
                    (enabled && value < number.max).then_some(value + number.step),
                    set
                ),
            ]
            .spacing(6)
            .align_y(Alignment::Center)
            .into(),
        };
        with_help(control, number.help.and_then(crate::bundle::help))
    }

    fn pick(&self, pick: Pick, enabled: bool) -> El<'a> {
        let settings = self.settings;
        let caps = self.caps;
        match pick {
            Pick::ExpCurve => picker(
                caps.exp_curves()
                    .iter()
                    .copied()
                    .map(ExpCurveChoice)
                    .collect::<Vec<_>>(),
                settings.selected_exp_curve.map(ExpCurveChoice),
                enabled,
                |choice| Edit::ExpCurve(choice.0),
            ),
            Pick::BaseStatGeneration => picker(
                generation_choices(caps.base_stat_update_generations()),
                Some(GenerationChoice(settings.update_base_stats_to_generation)),
                enabled,
                |choice| Edit::BaseStatGeneration(choice.0),
            ),
            Pick::MoveGeneration => picker(
                generation_choices(caps.move_update_generations()),
                Some(GenerationChoice(settings.update_moves_to_generation)),
                enabled,
                |choice| Edit::MoveGeneration(choice.0),
            ),
            Pick::Starter(slot) => self.starter_pick(slot, enabled),
            Pick::TrainerMode => self.trainer_mode_pick(),
        }
    }

    fn starter_pick(&self, slot: usize, enabled: bool) -> El<'a> {
        let current = self.settings.custom_starters[slot];
        let selected = self.starters.iter().find(|choice| choice.value == current);
        if !enabled {
            let label =
                selected.map_or_else(|| STARTER_PLACEHOLDER.to_owned(), ToString::to_string);
            return container(disabled_field(label))
                .width(STARTER_PICK_WIDTH)
                .into();
        }
        pick_list(self.starters, selected, move |choice: StarterChoice| {
            OptionMessage::Edit(Edit::Starter(slot, choice.value))
        })
        .placeholder(STARTER_PLACEHOLDER)
        .text_size(13)
        .padding([6, 10])
        .width(STARTER_PICK_WIDTH)
        .into()
    }

    fn trainer_mode_pick(&self) -> El<'a> {
        let selected = TrainerModeChoice(self.settings.trainers_mod);
        let options: Vec<TrainerModeChoice> = self
            .caps
            .trainer_modes()
            .into_iter()
            .map(TrainerModeChoice)
            .collect();
        let list = pick_list(options, Some(selected), |choice: TrainerModeChoice| {
            OptionMessage::Edit(Edit::TrainerMode(choice.0))
        })
        .text_size(13)
        .padding([6, 10])
        .width(280);
        with_help(list, selected.help())
    }

    fn action(&self, action: Action, enabled: bool) -> El<'a> {
        match action {
            Action::EditGenerationLimits => button(
                text(generation_limit_summary(self.settings))
                    .size(13)
                    .align_x(Horizontal::Center),
            )
            .padding([6, 12])
            .style(style::secondary_button)
            .on_press_maybe(enabled.then_some(OptionMessage::Action(action)))
            .into(),
        }
    }

    fn radio(&self, radio: &'static Radio) -> El<'a> {
        let choices: Vec<&'static Item> = radio
            .choices
            .iter()
            .filter(|choice| choice.is_visible(self.caps))
            .collect();
        let segmented = choices.len() <= 4
            && choices
                .iter()
                .all(|choice| choice.label.label().chars().count() <= SEGMENT_LABEL_LIMIT);
        let control = if segmented {
            self.segmented(&choices)
        } else {
            Column::with_children(choices.iter().map(|choice| self.choice_row(choice)))
                .spacing(2)
                .into()
        };
        let mut content = Column::new().spacing(6);
        if let Some(title) = radio.title {
            content = content.push(
                text(title.title())
                    .size(12)
                    .font(SEMIBOLD)
                    .style(style::muted_text),
            );
        }
        content = content.push(control);
        let summary = choices
            .iter()
            .find(|choice| choice.is_selected(self.settings))
            .and_then(|choice| choice.help_text())
            .and_then(|help| help.lines().next());
        if let Some(summary) = summary {
            content = content.push(text(summary).size(12).style(style::muted_text));
        }
        nest(content.into(), radio.nested)
    }

    fn segmented(&self, choices: &[&'static Item]) -> El<'a> {
        let segments = choices.iter().map(|choice| {
            let selected = choice.is_selected(self.settings);
            let enabled = choice.is_enabled(self.settings, self.caps);
            let segment = button(
                text(choice.label.label())
                    .size(13)
                    .width(Fill)
                    .align_x(Horizontal::Center),
            )
            .width(Fill)
            .padding([6, 10])
            .style(style::segment(selected, enabled))
            .on_press_maybe(enabled.then(|| select_message(choice)).flatten());
            with_help(segment, choice.help_text())
        });
        container(iced::widget::Row::with_children(segments).spacing(2))
            .padding(3)
            .width(Fill)
            .style(style::segmented_track)
            .into()
    }

    fn choice_row(&self, choice: &'static Item) -> El<'a> {
        let selected = choice.is_selected(self.settings);
        let enabled = choice.is_enabled(self.settings, self.caps);
        let dot =
            container(space().width(16).height(16)).style(style::radio_dot(selected, enabled));
        let entry = button(
            row![dot, text(choice.label.label()).size(14)]
                .spacing(10)
                .align_y(Alignment::Center),
        )
        .width(Fill)
        .padding([6, 8])
        .style(style::radio_row(selected, enabled))
        .on_press_maybe(enabled.then(|| select_message(choice)).flatten());
        with_help(entry, choice.help_text())
    }

    fn tweaks(&self, tweaks: &[MiscTweak], card: &'static Card) -> El<'a> {
        if tweaks.is_empty() {
            return text(card.unavailable_note.map_or("", crate::bundle::label))
                .size(13)
                .style(style::muted_text)
                .into();
        }
        grid(tweaks.iter().map(|tweak| self.tweak_chip(*tweak)))
            .fluid(TWEAK_CHIP_WIDTH)
            .spacing(SPACE_1)
            .height(iced::widget::grid::Sizing::EvenlyDistribute(Length::Shrink))
            .into()
    }

    fn tweak_chip(&self, tweak: MiscTweak) -> El<'a> {
        let selected = tweak.is_set_in(self.settings.current_misc_tweaks);
        let help = options::misc_tweak_help(tweak);
        let description = help.map(first_sentence).unwrap_or_default();
        let indicator = text(if selected { "✓" } else { "+" })
            .size(13)
            .font(SEMIBOLD);
        let chip = button(
            column![
                row![
                    text(options::misc_tweak_label(tweak))
                        .size(14)
                        .font(SEMIBOLD)
                        .width(Fill),
                    indicator
                ]
                .spacing(SPACE_1),
                text(description).size(12).style(style::muted_text),
            ]
            .spacing(4),
        )
        .width(Fill)
        .height(TWEAK_CHIP_HEIGHT)
        .padding([10, 12])
        .style(style::chip(selected))
        .on_press(OptionMessage::Edit(Edit::MiscTweak(tweak, !selected)));
        with_help(chip, help)
    }

    fn disclosure(&self, card: &'static Card) -> Option<El<'a>> {
        let hidden = hidden_labels(card, self.caps);
        if hidden.is_empty() {
            return None;
        }
        let open = self.disclosures.contains(card.id);
        let noun = if hidden.len() == 1 {
            "option"
        } else {
            "options"
        };
        let toggle = button(
            row![
                text(if open { "▾" } else { "▸" }).size(12),
                text(format!("{} {noun} unavailable for this game", hidden.len())).size(12),
            ]
            .spacing(6)
            .align_y(Alignment::Center),
        )
        .padding([4, 8])
        .style(style::quiet_button)
        .on_press(OptionMessage::ToggleDisclosure(card.id));
        let mut content = column![toggle].spacing(4);
        if open {
            content = content.push(
                container(text(hidden.join("  ·  ")).size(12).style(style::muted_text))
                    .padding([0, 8]),
            );
        }
        Some(content.into())
    }

    fn estimated_height(&self, card_match: &CardMatch) -> f32 {
        let card = card_match.card;
        if !card.is_visible(self.caps) {
            return 90.0;
        }
        let rows: f32 = card_match
            .rows
            .iter()
            .filter(|row| row.is_visible(self.caps))
            .map(|row| self.estimated_row_height(row))
            .sum();
        let tweaks = (card_match.tweaks.len() as f32 / 2.0).ceil() * (TWEAK_CHIP_HEIGHT + SPACE_1);
        110.0 + rows + tweaks
    }

    fn estimated_row_height(&self, row: &Row) -> f32 {
        match row {
            Row::Item(Item {
                control:
                    Control::Toggle {
                        attachment: Some(Attachment::Number(_)),
                        ..
                    },
                ..
            }) => 72.0,
            Row::Item(_) => 34.0,
            Row::Radio(radio) => {
                let visible = radio
                    .choices
                    .iter()
                    .filter(|choice| choice.is_visible(self.caps))
                    .count();
                60.0 + visible as f32 * 30.0
            }
            Row::Caption { .. } => 26.0,
        }
    }
}

fn two_columns<'a>(cards: Vec<(f32, El<'a>)>) -> El<'a> {
    let mut left = Column::new().spacing(SPACE_2).width(Length::FillPortion(1));
    let mut right = Column::new().spacing(SPACE_2).width(Length::FillPortion(1));
    let (mut left_height, mut right_height) = (0.0, 0.0);
    for (height, card) in cards {
        if left_height <= right_height {
            left = left.push(card);
            left_height += height;
        } else {
            right = right.push(card);
            right_height += height;
        }
    }
    row![left, right].spacing(SPACE_2).into()
}

fn card_header<'a>(card: &'static Card) -> El<'a> {
    column![
        text(card.title.title()).size(17).font(SEMIBOLD),
        text(card.summary).size(13).style(style::muted_text),
    ]
    .spacing(4)
    .into()
}

fn unavailable_card<'a>(card: &'static Card) -> El<'a> {
    let mut content = column![
        card_header(card),
        text("Unavailable for this game")
            .size(13)
            .style(style::accent_text),
    ]
    .spacing(SPACE_1);
    if let Some(note) = card.unavailable_note {
        content = content.push(
            text(crate::bundle::label(note))
                .size(12)
                .style(style::muted_text),
        );
    }
    container(content)
        .padding(20)
        .width(Fill)
        .style(style::subtle_card)
        .into()
}

fn hidden_labels(card: &'static Card, caps: &Capabilities) -> Vec<&'static str> {
    let mut labels: Vec<&'static str> = card
        .rows
        .iter()
        .flat_map(|row| row.hidden_labels(caps))
        .collect();
    if card.body == CardBody::MiscTweaks {
        labels.extend(
            MiscTweak::ALL
                .iter()
                .filter(|tweak| !caps.has_misc_tweak(**tweak))
                .map(|tweak| options::misc_tweak_label(*tweak)),
        );
    }
    labels
}

fn nest<'a>(content: El<'a>, nested: bool) -> El<'a> {
    if nested {
        container(content)
            .padding(iced::Padding::default().left(NESTED_INDENT))
            .width(Fill)
            .into()
    } else {
        content
    }
}

fn with_help<'a>(content: impl Into<El<'a>>, help: Option<&'static str>) -> El<'a> {
    match help {
        Some(help) => tooltip(
            content,
            container(text(help).size(12))
                .padding(12)
                .max_width(TOOLTIP_WIDTH)
                .style(style::tooltip_bubble),
            tooltip::Position::Top,
        )
        .gap(6)
        .delay(Duration::from_millis(400))
        .snap_within_viewport(true)
        .into(),
        None => content.into(),
    }
}

fn select_message(choice: &Item) -> Option<OptionMessage> {
    match choice.control {
        Control::Choice { select, .. } => Some(OptionMessage::Edit(Edit::Select(select))),
        _ => None,
    }
}

fn picker<'a, T>(
    options: Vec<T>,
    selected: Option<T>,
    enabled: bool,
    edit: impl Fn(T) -> Edit + 'a,
) -> El<'a>
where
    T: ToString + PartialEq + Clone + 'a,
{
    if !enabled {
        return disabled_field(selected.map(|value| value.to_string()).unwrap_or_default());
    }
    pick_list(options, selected, move |choice| {
        OptionMessage::Edit(edit(choice))
    })
    .text_size(13)
    .padding([6, 10])
    .into()
}

fn disabled_field<'a>(label: String) -> El<'a> {
    container(text(label).size(13))
        .padding([6, 10])
        .width(Fill)
        .style(style::disabled_field)
        .into()
}

fn step_button<'a>(
    label: &'static str,
    target: Option<i32>,
    set: crate::options::spec::NumberSetter,
) -> El<'a> {
    button(text(label).size(14).align_x(Horizontal::Center))
        .width(28)
        .padding([2, 0])
        .style(style::secondary_button)
        .on_press_maybe(target.map(|value| OptionMessage::Edit(Edit::Number(set, value))))
        .into()
}

fn generation_choices(generations: Vec<i32>) -> Vec<GenerationChoice> {
    generations.into_iter().map(GenerationChoice).collect()
}

fn format_number(format: NumberFormat, value: i32) -> String {
    match format {
        NumberFormat::Plain => value.to_string(),
        NumberFormat::Percent => format!("{value}%"),
        NumberFormat::SignedPercent if value > 0 => format!("+{value}%"),
        NumberFormat::SignedPercent => format!("{value}%"),
        NumberFormat::Level => format!("Lv. {value}"),
    }
}

fn first_sentence(help: &'static str) -> String {
    let first_line = help.lines().next().unwrap_or_default();
    let sentence = first_line
        .find(". ")
        .map_or(first_line, |end| &first_line[..=end]);
    const LIMIT: usize = 80;
    if sentence.chars().count() <= LIMIT {
        return sentence.to_owned();
    }
    let truncated: String = sentence.chars().take(LIMIT - 1).collect();
    format!("{}…", truncated.trim_end())
}

pub fn generation_limit_summary(settings: &Settings) -> String {
    let Some(restrictions) = settings.current_restrictions else {
        return "Choose generations…".to_owned();
    };
    let allowed = [
        restrictions.allow_gen1,
        restrictions.allow_gen2,
        restrictions.allow_gen3,
        restrictions.allow_gen4,
        restrictions.allow_gen5,
        restrictions.allow_gen6,
        restrictions.allow_gen7,
    ];
    let generations: Vec<String> = allowed
        .iter()
        .enumerate()
        .filter(|(_, allowed)| **allowed)
        .map(|(index, _)| (index + 1).to_string())
        .collect();
    if generations.is_empty() {
        return "No generations".to_owned();
    }
    let relatives = if restrictions.allow_evolutionary_relatives {
        " + relatives"
    } else {
        ""
    };
    format!("Gen {}{relatives}", generations.join(", "))
}
