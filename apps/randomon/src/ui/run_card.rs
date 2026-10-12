use iced::widget::{
    Column, button, center, column, container, opaque, progress_bar, row, space, text,
};
use iced::{Alignment, Element, Fill, Font};

use crate::run::{CardAction, Finished, REVEAL_LABEL, RunProgress, RunState};
use crate::ui::dialogs;
use crate::ui::options_view::SEMIBOLD;
use crate::ui::style::{self, SPACE_1, SPACE_2, SPACE_3};

const CARD_WIDTH: f32 = 480.0;
const BADGE_SIZE: f32 = 36.0;

pub fn overlay(state: &RunState) -> Option<Element<'_, CardAction>> {
    match state {
        RunState::Idle => None,
        RunState::Running(progress) => Some(blocking(progress_card(progress))),
        RunState::Done(finished) => {
            Some(dialogs::modal(success_card(finished), CardAction::Dismiss))
        }
        RunState::Failed(message) => Some(dialogs::modal(error_card(message), CardAction::Dismiss)),
    }
}

fn blocking<'a>(card: Element<'a, CardAction>) -> Element<'a, CardAction> {
    opaque(
        container(center(card))
            .width(Fill)
            .height(Fill)
            .style(style::backdrop),
    )
}

fn progress_card(progress: &RunProgress) -> Element<'_, CardAction> {
    let detail = progress.detail.as_deref().unwrap_or("\u{00A0}");
    let content = column![
        text(progress.label).size(20).font(SEMIBOLD),
        row![
            progress_bar(0.0..=1.0, progress.fraction)
                .girth(8)
                .style(style::progress_track),
            container(
                text(format!("{}%", progress.percent()))
                    .size(13)
                    .font(SEMIBOLD)
                    .style(style::accent_text)
            )
            .align_right(44),
        ]
        .spacing(SPACE_1)
        .align_y(Alignment::Center),
        container(
            text(detail)
                .size(12)
                .font(Font::MONOSPACE)
                .style(style::muted_text)
                .wrapping(text::Wrapping::None)
        )
        .width(Fill)
        .clip(true),
    ]
    .spacing(SPACE_2);
    dialogs::frame(content, CARD_WIDTH)
}

fn badge<'a>(
    glyph: &'a str,
    tint: fn(&iced::Theme) -> container::Style,
) -> Element<'a, CardAction> {
    container(text(glyph).size(18).font(SEMIBOLD))
        .center_x(BADGE_SIZE)
        .center_y(BADGE_SIZE)
        .style(tint)
        .into()
}

fn header<'a>(
    glyph: &'a str,
    tint: fn(&iced::Theme) -> container::Style,
    title: &'a str,
) -> Element<'a, CardAction> {
    row![badge(glyph, tint), text(title).size(20).font(SEMIBOLD)]
        .spacing(SPACE_2)
        .align_y(Alignment::Center)
        .into()
}

fn success_card(finished: &Finished) -> Element<'_, CardAction> {
    let mut output = column![
        container(
            text(finished.output_name())
                .size(14)
                .font(SEMIBOLD)
                .wrapping(text::Wrapping::WordOrGlyph)
        )
        .padding([10, 12])
        .width(Fill)
        .style(style::read_only_field),
    ]
    .spacing(6);
    if let Some(hint) = &finished.install_hint {
        output = output.push(text(hint).size(12).style(style::muted_text));
    }
    let mut content: Column<'_, CardAction> =
        column![header("✓", style::success_badge, finished.headline), output].spacing(SPACE_2);
    if let Some(code) = &finished.race_code {
        content = content.push(race_code(code));
    }
    if let Some(log) = &finished.log {
        content = content.push(spoiler_log(log));
    }
    content = content.push(footer(
        button(text(REVEAL_LABEL))
            .padding([8, 16])
            .style(style::secondary_button)
            .on_press(CardAction::Reveal(finished.output.clone())),
        "Randomize again",
    ));
    dialogs::frame(content, CARD_WIDTH)
}

fn spoiler_log(log: &std::path::Path) -> Element<'_, CardAction> {
    row![
        column![
            text("Spoiler log").size(14).font(SEMIBOLD),
            text("Everything that changed. Skip it to avoid spoilers.")
                .size(12)
                .style(style::muted_text),
        ]
        .spacing(2)
        .width(Fill),
        button(text("Open spoiler log"))
            .padding([8, 16])
            .style(style::secondary_button)
            .on_press(CardAction::OpenLog(log.to_path_buf())),
    ]
    .spacing(SPACE_2)
    .align_y(Alignment::Center)
    .into()
}

fn race_code(code: &str) -> Element<'_, CardAction> {
    column![
        row![
            text("Race code").size(14).font(SEMIBOLD),
            space::horizontal(),
            container(text(code).size(14).font(Font::MONOSPACE))
                .padding([4, 10])
                .style(style::pill),
        ]
        .align_y(Alignment::Center),
        text("Racers compare this code to make sure they're playing identical games.")
            .size(12)
            .style(style::muted_text),
    ]
    .spacing(4)
    .into()
}

fn error_card(message: &str) -> Element<'_, CardAction> {
    let content = column![
        header("!", style::danger_badge, "Randomization didn't finish"),
        container(text(message).size(13))
            .padding([10, 12])
            .width(Fill)
            .style(style::calm_error_note),
        text("Nothing was changed in your original game. Adjust the settings or try another seed.")
            .size(12)
            .style(style::muted_text),
        footer(space(), "Try again"),
    ]
    .spacing(SPACE_2);
    dialogs::frame(content, CARD_WIDTH)
}

fn footer<'a>(
    leading: impl Into<Element<'a, CardAction>>,
    again: &'a str,
) -> Element<'a, CardAction> {
    container(
        row![
            leading.into(),
            space::horizontal(),
            button(text(again))
                .padding([8, 16])
                .style(style::secondary_button)
                .on_press(CardAction::RandomizeAgain),
            button(text("Done"))
                .padding([8, 20])
                .style(style::primary_button)
                .on_press(CardAction::Dismiss),
        ]
        .spacing(SPACE_1)
        .align_y(Alignment::Center),
    )
    .padding(iced::Padding::ZERO.top(SPACE_3 - SPACE_2))
    .into()
}
