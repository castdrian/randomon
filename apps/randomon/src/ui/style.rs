use iced::widget::{button, container};
use iced::{Background, Border, Color, Shadow, Theme, Vector};

pub const RADIUS: f32 = 12.0;

pub fn sidebar(theme: &Theme) -> container::Style {
    let palette = theme.extended_palette();
    container::Style {
        background: Some(Background::Color(palette.background.weakest.color)),
        border: Border {
            color: palette.background.weak.color,
            width: 1.0,
            radius: 0.0.into(),
        },
        ..container::Style::default()
    }
}

pub fn card(theme: &Theme) -> container::Style {
    let palette = theme.extended_palette();
    container::Style {
        background: Some(Background::Color(palette.background.weakest.color)),
        border: Border {
            color: palette.background.weak.color,
            width: 1.0,
            radius: RADIUS.into(),
        },
        shadow: Shadow {
            color: Color::from_rgba(0.0, 0.0, 0.0, 0.08),
            offset: Vector::new(0.0, 2.0),
            blur_radius: 12.0,
        },
        ..container::Style::default()
    }
}

pub fn drop_zone(theme: &Theme) -> container::Style {
    let palette = theme.extended_palette();
    container::Style {
        background: Some(Background::Color(palette.background.weakest.color)),
        border: Border {
            color: palette.primary.weak.color,
            width: 1.5,
            radius: RADIUS.into(),
        },
        ..container::Style::default()
    }
}

pub fn action_bar(theme: &Theme) -> container::Style {
    let palette = theme.extended_palette();
    container::Style {
        background: Some(Background::Color(palette.background.weakest.color)),
        border: Border {
            color: palette.background.weak.color,
            width: 1.0,
            radius: 0.0.into(),
        },
        ..container::Style::default()
    }
}

pub fn pill(theme: &Theme) -> container::Style {
    let palette = theme.extended_palette();
    container::Style {
        text_color: Some(palette.primary.base.color),
        background: Some(Background::Color(Color {
            a: 0.12,
            ..palette.primary.base.color
        })),
        border: Border {
            radius: 999.0.into(),
            ..Border::default()
        },
        ..container::Style::default()
    }
}

pub fn nav_item(selected: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |theme, status| {
        let palette = theme.extended_palette();
        let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
        let background = if selected {
            Some(Background::Color(Color {
                a: 0.14,
                ..palette.primary.base.color
            }))
        } else if hovered {
            Some(Background::Color(palette.background.weak.color))
        } else {
            None
        };
        button::Style {
            background,
            text_color: if selected {
                palette.primary.base.color
            } else {
                palette.background.base.text
            },
            border: Border {
                radius: 8.0.into(),
                ..Border::default()
            },
            ..button::Style::default()
        }
    }
}

pub fn support_button(theme: &Theme, status: button::Status) -> button::Style {
    let palette = theme.extended_palette();
    let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
    button::Style {
        background: Some(Background::Color(Color {
            a: if hovered { 0.22 } else { 0.12 },
            ..palette.primary.base.color
        })),
        text_color: palette.primary.base.color,
        border: Border {
            radius: 8.0.into(),
            ..Border::default()
        },
        ..button::Style::default()
    }
}

pub fn primary_button(theme: &Theme, status: button::Status) -> button::Style {
    rounded(button::primary(theme, status))
}

pub fn secondary_button(theme: &Theme, status: button::Status) -> button::Style {
    let palette = theme.extended_palette();
    let background = match status {
        button::Status::Hovered | button::Status::Pressed => palette.background.strong.color,
        _ => palette.background.weak.color,
    };
    rounded(button::Style {
        background: Some(Background::Color(background)),
        text_color: palette.background.base.text,
        ..button::Style::default()
    })
}

fn rounded(style: button::Style) -> button::Style {
    button::Style {
        border: Border {
            radius: 8.0.into(),
            ..style.border
        },
        ..style
    }
}
