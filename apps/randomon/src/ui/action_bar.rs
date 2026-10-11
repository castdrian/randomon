use iced::widget::{Row, button, column, container, row, space, text, text_input, tooltip};
use iced::{Alignment, Element, Fill, Length};

use crate::ui::style::{self, SPACE_1, SPACE_2, SPACE_3};

pub const COMPACT_WIDTH: f32 = 1180.0;

pub fn bar<'a, Message: 'a>(
    status: Option<Element<'a, Message>>,
    controls: Vec<Element<'a, Message>>,
    compact: bool,
) -> Element<'a, Message> {
    let controls = Row::with_children(controls)
        .spacing(SPACE_2)
        .align_y(Alignment::Center);
    let content: Element<'a, Message> = match (status, compact) {
        (Some(status), true) => column![
            row![space::horizontal(), controls],
            container(status).width(Fill)
        ]
        .spacing(SPACE_1)
        .into(),
        (status, _) => row![
            container(status.unwrap_or_else(|| space().into())).width(Fill),
            controls
        ]
        .spacing(SPACE_2)
        .align_y(Alignment::Center)
        .into(),
    };
    container(content)
        .padding([SPACE_2, SPACE_3])
        .width(Fill)
        .style(style::action_bar)
        .into()
}

pub fn settings_button<'a, Message: Clone + 'a>(on_press: Message) -> Element<'a, Message> {
    hint(
        button(text("Settings"))
            .padding([10, 16])
            .style(style::secondary_button)
            .on_press(on_press),
        "Copy, paste, import or export these settings",
    )
}

pub fn seed_controls<'a, Message: Clone + 'a>(
    seed: &str,
    on_change: impl Fn(String) -> Message + 'a,
    on_roll: Message,
) -> Element<'a, Message> {
    row![
        text("Seed").size(13).style(style::muted_text),
        text_input("Seed", seed)
            .on_input(on_change)
            .padding(10)
            .width(Length::Fixed(170.0)),
        hint(
            button(text("🎲"))
                .padding([10, 14])
                .style(style::secondary_button)
                .on_press(on_roll),
            "Roll a new random seed",
        ),
    ]
    .spacing(SPACE_1)
    .align_y(Alignment::Center)
    .into()
}

fn hint<'a, Message: 'a>(
    content: impl Into<Element<'a, Message>>,
    label: &'a str,
) -> Element<'a, Message> {
    tooltip(
        content,
        container(text(label).size(12))
            .padding([6, 10])
            .style(style::tooltip_bubble),
        tooltip::Position::Top,
    )
    .gap(6)
    .into()
}
