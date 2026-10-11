use iced::widget::{image, svg};
use iced::{Color, Element, Length};

pub const GITHUB_SPONSORS_URL: &str = "https://github.com/sponsors/castdrian";
pub const KO_FI_URL: &str = "https://ko-fi.com/castdrian";
pub const DISCORD_URL: &str = "https://discord.gg/NdaBaxFKnn";
pub const REPOSITORY_URL: &str = "https://github.com/castdrian/randomon";

pub fn open_link(url: &str) {
    if let Err(error) = open::that_detached(url) {
        log::warn!("failed to open {url}: {error}");
    }
}

const KO_FI_SYMBOL: &[u8] = include_bytes!("../assets/brand/kofi_symbol.png");
const GITHUB_SPONSORS_SYMBOL: &[u8] = include_bytes!("../assets/brand/github-sponsors.svg");
const GITHUB_SPONSORS_PINK: Color = Color::from_rgb8(0xEA, 0x4A, 0xAA);
const ICON_SIZE: f32 = 18.0;

pub fn ko_fi_icon<'a, Message: 'a>() -> Element<'a, Message> {
    image(image::Handle::from_bytes(KO_FI_SYMBOL))
        .width(Length::Fixed(ICON_SIZE * 1.25))
        .height(Length::Fixed(ICON_SIZE))
        .into()
}

pub fn github_sponsors_icon<'a, Message: 'a>() -> Element<'a, Message> {
    svg(svg::Handle::from_memory(GITHUB_SPONSORS_SYMBOL))
        .width(Length::Fixed(ICON_SIZE))
        .height(Length::Fixed(ICON_SIZE))
        .style(|_theme, _status| svg::Style {
            color: Some(GITHUB_SPONSORS_PINK),
        })
        .into()
}
