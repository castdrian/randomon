use iced::theme::{Mode, Palette};
use iced::{Color, Theme};

pub fn for_mode(mode: Mode) -> Theme {
    match mode {
        Mode::Dark => dark(),
        Mode::Light | Mode::None => light(),
    }
}

fn dark() -> Theme {
    Theme::custom(
        "Randomon Dark",
        Palette {
            background: Color::from_rgb8(0x10, 0x12, 0x18),
            text: Color::from_rgb8(0xE8, 0xEA, 0xF0),
            primary: Color::from_rgb8(0x5B, 0x8C, 0xFF),
            success: Color::from_rgb8(0x3D, 0xD6, 0x8C),
            warning: Color::from_rgb8(0xF5, 0xB5, 0x44),
            danger: Color::from_rgb8(0xFF, 0x4D, 0x4F),
        },
    )
}

fn light() -> Theme {
    Theme::custom(
        "Randomon Light",
        Palette {
            background: Color::from_rgb8(0xF6, 0xF7, 0xFB),
            text: Color::from_rgb8(0x14, 0x16, 0x1C),
            primary: Color::from_rgb8(0x2F, 0x6B, 0xEA),
            success: Color::from_rgb8(0x1F, 0x9D, 0x5C),
            warning: Color::from_rgb8(0xC9, 0x84, 0x10),
            danger: Color::from_rgb8(0xD9, 0x2D, 0x2F),
        },
    )
}
