use iced::widget::{button, container, slider, text, toggler};
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
    let (background, text_color) = match status {
        button::Status::Hovered | button::Status::Pressed => (
            palette.background.strong.color,
            palette.background.base.text,
        ),
        button::Status::Disabled => (
            Color {
                a: 0.5,
                ..palette.background.weak.color
            },
            muted_color(theme),
        ),
        button::Status::Active => (palette.background.weak.color, palette.background.base.text),
    };
    rounded(button::Style {
        background: Some(Background::Color(background)),
        text_color,
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

pub const SPACE_1: f32 = 8.0;
pub const SPACE_2: f32 = 16.0;
pub const SPACE_3: f32 = 24.0;
pub const NESTED_INDENT: f32 = 32.0;

pub fn muted_color(theme: &Theme) -> Color {
    Color {
        a: 0.62,
        ..theme.extended_palette().background.base.text
    }
}

pub fn muted_text(theme: &Theme) -> text::Style {
    text::Style {
        color: Some(muted_color(theme)),
    }
}

pub fn accent_text(theme: &Theme) -> text::Style {
    text::Style {
        color: Some(theme.extended_palette().primary.base.color),
    }
}

pub fn subtle_card(theme: &Theme) -> container::Style {
    let palette = theme.extended_palette();
    container::Style {
        background: Some(Background::Color(palette.background.base.color)),
        border: Border {
            color: palette.background.weak.color,
            width: 1.0,
            radius: RADIUS.into(),
        },
        ..container::Style::default()
    }
}

pub fn tooltip_bubble(theme: &Theme) -> container::Style {
    let palette = theme.extended_palette();
    container::Style {
        text_color: Some(palette.background.base.text),
        background: Some(Background::Color(palette.background.weakest.color)),
        border: Border {
            color: palette.background.strong.color,
            width: 1.0,
            radius: 8.0.into(),
        },
        shadow: Shadow {
            color: Color::from_rgba(0.0, 0.0, 0.0, 0.18),
            offset: Vector::new(0.0, 4.0),
            blur_radius: 16.0,
        },
        ..container::Style::default()
    }
}

pub fn segmented_track(theme: &Theme) -> container::Style {
    let palette = theme.extended_palette();
    container::Style {
        background: Some(Background::Color(palette.background.weak.color)),
        border: Border {
            radius: 10.0.into(),
            ..Border::default()
        },
        ..container::Style::default()
    }
}

pub fn segment(selected: bool, enabled: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |theme, status| {
        let palette = theme.extended_palette();
        let hovered = enabled && matches!(status, button::Status::Hovered);
        let background = if selected {
            Some(Background::Color(palette.background.weakest.color))
        } else if hovered {
            Some(Background::Color(palette.background.strong.color))
        } else {
            None
        };
        let text_color = match (selected, enabled) {
            (true, _) => palette.primary.base.color,
            (false, true) => palette.background.base.text,
            (false, false) => muted_color(theme),
        };
        button::Style {
            background,
            text_color,
            border: Border {
                radius: 8.0.into(),
                ..Border::default()
            },
            shadow: if selected {
                Shadow {
                    color: Color::from_rgba(0.0, 0.0, 0.0, 0.12),
                    offset: Vector::new(0.0, 1.0),
                    blur_radius: 3.0,
                }
            } else {
                Shadow::default()
            },
            ..button::Style::default()
        }
    }
}

pub fn radio_row(
    selected: bool,
    enabled: bool,
) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |theme, status| {
        let palette = theme.extended_palette();
        let hovered = enabled && matches!(status, button::Status::Hovered);
        let background = if selected {
            Some(Background::Color(Color {
                a: 0.10,
                ..palette.primary.base.color
            }))
        } else if hovered {
            Some(Background::Color(palette.background.weak.color))
        } else {
            None
        };
        button::Style {
            background,
            text_color: if enabled {
                palette.background.base.text
            } else {
                muted_color(theme)
            },
            border: Border {
                radius: 8.0.into(),
                ..Border::default()
            },
            ..button::Style::default()
        }
    }
}

pub fn radio_dot(selected: bool, enabled: bool) -> impl Fn(&Theme) -> container::Style {
    move |theme| {
        let palette = theme.extended_palette();
        let accent = if enabled {
            palette.primary.base.color
        } else {
            palette.background.strong.color
        };
        container::Style {
            background: Some(Background::Color(if selected {
                accent
            } else {
                Color::TRANSPARENT
            })),
            border: Border {
                color: if selected {
                    accent
                } else {
                    palette.background.strong.color
                },
                width: if selected { 4.5 } else { 1.5 },
                radius: 999.0.into(),
            },
            ..container::Style::default()
        }
    }
}

pub fn chip(selected: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |theme, status| {
        let palette = theme.extended_palette();
        let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
        let (background, border_color, text_color) = if selected {
            (
                Color {
                    a: if hovered { 0.22 } else { 0.14 },
                    ..palette.primary.base.color
                },
                palette.primary.base.color,
                palette.primary.base.color,
            )
        } else {
            (
                if hovered {
                    palette.background.weak.color
                } else {
                    palette.background.weakest.color
                },
                palette.background.strong.color,
                palette.background.base.text,
            )
        };
        button::Style {
            background: Some(Background::Color(background)),
            text_color,
            border: Border {
                color: border_color,
                width: 1.0,
                radius: 10.0.into(),
            },
            ..button::Style::default()
        }
    }
}

pub fn toggle_switch(theme: &Theme, status: toggler::Status) -> toggler::Style {
    let base = toggler::default(theme, status);
    toggler::Style {
        text_color: matches!(status, toggler::Status::Disabled { .. }).then(|| muted_color(theme)),
        ..base
    }
}

pub fn range_slider(enabled: bool) -> impl Fn(&Theme, slider::Status) -> slider::Style {
    move |theme, status| {
        let palette = theme.extended_palette();
        let base = slider::default(theme, status);
        if enabled {
            return base.with_circular_handle(7.0);
        }
        let mut style = base.with_circular_handle(6.0);
        style.rail.backgrounds = (
            palette.background.strong.color.into(),
            palette.background.weak.color.into(),
        );
        style.handle.background = palette.background.strong.color.into();
        style
    }
}

pub fn value_badge(enabled: bool) -> impl Fn(&Theme) -> container::Style {
    move |theme| {
        let palette = theme.extended_palette();
        container::Style {
            text_color: Some(if enabled {
                palette.primary.base.color
            } else {
                muted_color(theme)
            }),
            background: Some(Background::Color(if enabled {
                Color {
                    a: 0.12,
                    ..palette.primary.base.color
                }
            } else {
                palette.background.weak.color
            })),
            border: Border {
                radius: 6.0.into(),
                ..Border::default()
            },
            ..container::Style::default()
        }
    }
}

pub fn disabled_field(theme: &Theme) -> container::Style {
    let palette = theme.extended_palette();
    container::Style {
        text_color: Some(muted_color(theme)),
        background: Some(Background::Color(palette.background.weak.color)),
        border: Border {
            color: palette.background.weak.color,
            width: 1.0,
            radius: 6.0.into(),
        },
        ..container::Style::default()
    }
}

pub fn quiet_button(theme: &Theme, status: button::Status) -> button::Style {
    let palette = theme.extended_palette();
    let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
    button::Style {
        background: hovered.then_some(Background::Color(palette.background.weak.color)),
        text_color: muted_color(theme),
        border: Border {
            radius: 6.0.into(),
            ..Border::default()
        },
        ..button::Style::default()
    }
}

pub fn count_badge(theme: &Theme) -> container::Style {
    let palette = theme.extended_palette();
    container::Style {
        text_color: Some(palette.primary.base.text),
        background: Some(Background::Color(palette.primary.base.color)),
        border: Border {
            radius: 999.0.into(),
            ..Border::default()
        },
        ..container::Style::default()
    }
}

pub fn backdrop(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(Color::from_rgba(0.0, 0.0, 0.0, 0.45))),
        ..container::Style::default()
    }
}

pub fn dialog(theme: &Theme) -> container::Style {
    let palette = theme.extended_palette();
    container::Style {
        background: Some(Background::Color(palette.background.weakest.color)),
        border: Border {
            color: palette.background.weak.color,
            width: 1.0,
            radius: 16.0.into(),
        },
        shadow: Shadow {
            color: Color::from_rgba(0.0, 0.0, 0.0, 0.3),
            offset: Vector::new(0.0, 12.0),
            blur_radius: 40.0,
        },
        ..container::Style::default()
    }
}

pub fn warning_note(theme: &Theme) -> container::Style {
    let palette = theme.extended_palette();
    container::Style {
        text_color: Some(palette.warning.strong.color),
        background: Some(Background::Color(Color {
            a: 0.12,
            ..palette.warning.base.color
        })),
        border: Border {
            radius: 8.0.into(),
            ..Border::default()
        },
        ..container::Style::default()
    }
}

pub fn danger_button(theme: &Theme, status: button::Status) -> button::Style {
    rounded(button::Style {
        text_color: Color::WHITE,
        ..button::danger(theme, status)
    })
}

pub fn read_only_field(theme: &Theme) -> container::Style {
    let palette = theme.extended_palette();
    container::Style {
        background: Some(Background::Color(palette.background.base.color)),
        border: Border {
            color: palette.background.strong.color,
            width: 1.0,
            radius: 8.0.into(),
        },
        ..container::Style::default()
    }
}

pub fn success_note(theme: &Theme) -> container::Style {
    tinted_note(theme.extended_palette().success.base.color)
}

pub fn danger_note(theme: &Theme) -> container::Style {
    tinted_note(theme.extended_palette().danger.base.color)
}

fn tinted_note(color: Color) -> container::Style {
    container::Style {
        text_color: Some(color),
        background: Some(Background::Color(Color { a: 0.12, ..color })),
        border: Border {
            radius: 8.0.into(),
            ..Border::default()
        },
        ..container::Style::default()
    }
}

pub fn progress_track(theme: &Theme) -> iced::widget::progress_bar::Style {
    let palette = theme.extended_palette();
    iced::widget::progress_bar::Style {
        background: Background::Color(palette.background.weak.color),
        bar: Background::Color(palette.primary.base.color),
        border: Border {
            radius: 999.0.into(),
            ..Border::default()
        },
    }
}

pub fn calm_error_note(theme: &Theme) -> container::Style {
    let palette = theme.extended_palette();
    container::Style {
        text_color: Some(palette.background.base.text),
        background: Some(Background::Color(Color {
            a: 0.08,
            ..palette.danger.base.color
        })),
        border: Border {
            color: Color {
                a: 0.3,
                ..palette.danger.base.color
            },
            width: 1.0,
            radius: 8.0.into(),
        },
        ..container::Style::default()
    }
}

pub fn success_badge(theme: &Theme) -> container::Style {
    round_badge(theme.extended_palette().success.base.color)
}

pub fn danger_badge(theme: &Theme) -> container::Style {
    round_badge(theme.extended_palette().danger.base.color)
}

fn round_badge(color: Color) -> container::Style {
    container::Style {
        text_color: Some(color),
        background: Some(Background::Color(Color { a: 0.14, ..color })),
        border: Border {
            radius: 999.0.into(),
            ..Border::default()
        },
        ..container::Style::default()
    }
}

pub fn update_chip(theme: &Theme) -> container::Style {
    let palette = theme.extended_palette();
    container::Style {
        text_color: Some(palette.primary.base.color),
        background: Some(Background::Color(Color {
            a: 0.12,
            ..palette.primary.base.color
        })),
        border: Border {
            color: Color {
                a: 0.35,
                ..palette.primary.base.color
            },
            width: 1.0,
            radius: 8.0.into(),
        },
        ..container::Style::default()
    }
}

pub fn chip_remove_button(theme: &Theme, status: button::Status) -> button::Style {
    let palette = theme.extended_palette();
    let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
    button::Style {
        background: hovered.then_some(Background::Color(Color {
            a: 0.2,
            ..palette.primary.base.color
        })),
        text_color: palette.primary.base.color,
        border: Border {
            radius: 6.0.into(),
            ..Border::default()
        },
        ..button::Style::default()
    }
}
