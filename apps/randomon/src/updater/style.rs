use iced::widget::{button, container, progress_bar};
use iced::{Background, Border, Color, Theme};

fn tint(color: Color, alpha: f32) -> Color {
    Color { a: alpha, ..color }
}

pub fn banner(theme: &Theme) -> container::Style {
    let primary = theme.extended_palette().primary.base.color;
    container::Style {
        background: Some(Background::Color(tint(primary, 0.08))),
        border: Border {
            color: tint(primary, 0.28),
            width: 1.0,
            radius: 10.0.into(),
        },
        ..container::Style::default()
    }
}

pub fn failed_banner(theme: &Theme) -> container::Style {
    let danger = theme.extended_palette().danger.base.color;
    container::Style {
        background: Some(Background::Color(tint(danger, 0.08))),
        border: Border {
            color: tint(danger, 0.3),
            width: 1.0,
            radius: 10.0.into(),
        },
        ..container::Style::default()
    }
}

pub fn dot(theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(
            theme.extended_palette().primary.base.color,
        )),
        border: Border {
            radius: 999.0.into(),
            ..Border::default()
        },
        ..container::Style::default()
    }
}

pub fn progress(theme: &Theme) -> progress_bar::Style {
    let primary = theme.extended_palette().primary.base.color;
    progress_bar::Style {
        background: Background::Color(tint(primary, 0.16)),
        bar: Background::Color(primary),
        border: Border {
            radius: 999.0.into(),
            ..Border::default()
        },
    }
}

pub fn link(theme: &Theme, status: button::Status) -> button::Style {
    let palette = theme.extended_palette();
    let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
    let base = crate::ui::style::muted_color(theme);
    button::Style {
        text_color: if hovered {
            palette.primary.base.color
        } else {
            base
        },
        ..button::Style::default()
    }
}
