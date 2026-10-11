use iced::widget::{Column, button, column, container, progress_bar, row, space, text};
use iced::{Alignment, Element, Fill};

use super::network::Progress;
use super::{Check, Offer, Stage, Updater, UpdaterMessage, style};
use crate::rom;
use crate::ui::options_view::SEMIBOLD;
use crate::ui::style as app_style;

type Item<'a> = Element<'a, UpdaterMessage>;

impl Updater {
    pub fn version_line(&self) -> Item<'_> {
        let version = text(format!("v{}", self.current))
            .size(12)
            .style(app_style::muted_text);
        let status: Item<'_> = match &self.check {
            Check::Checking { manual: true } => muted("Checking\u{2026}"),
            Check::UpToDate => muted("Up to date"),
            Check::Checking { manual: false } | Check::Idle | Check::Failed(_) => {
                link("Check for updates", UpdaterMessage::Check)
            }
        };
        let line = row![
            version,
            text("\u{00B7}").size(12).style(app_style::muted_text),
            status
        ]
        .spacing(6)
        .align_y(Alignment::Center);
        match &self.check {
            Check::Failed(error) => column![line, text(error).size(11).style(text::danger)]
                .spacing(4)
                .into(),
            _ => line.into(),
        }
    }

    pub fn banner(&self) -> Option<Item<'_>> {
        let offer = self.offer.as_ref()?;
        let failed = matches!(offer.stage, Stage::Failed(_));
        let body = match &offer.stage {
            Stage::Ready => self.ready(offer),
            Stage::Downloading(progress) => downloading(offer, *progress),
            Stage::Installing => working(offer, "Installing", "randomon restarts when it's done."),
            Stage::Installed => installed(offer),
            Stage::Failed(error) => failed_body(error),
        };
        Some(
            container(body)
                .padding(12)
                .width(Fill)
                .style(if failed {
                    style::failed_banner
                } else {
                    style::banner
                })
                .into(),
        )
    }

    fn ready<'a>(&self, offer: &'a Offer) -> Item<'a> {
        let action = if self.installable().is_some() {
            "Update"
        } else {
            "Download"
        };
        column![
            header("Update available", true),
            text(format!("randomon {} is available", offer.release.version))
                .size(14)
                .font(SEMIBOLD),
            row![
                secondary("What's new", UpdaterMessage::OpenReleasePage),
                primary(action, UpdaterMessage::Update),
            ]
            .spacing(6),
            link("Skip this version", UpdaterMessage::Skip),
        ]
        .spacing(8)
        .into()
    }
}

fn header<'a>(title: &'a str, dismissible: bool) -> Item<'a> {
    let mut line = row![
        container(space().width(6).height(6)).style(style::dot),
        text(title)
            .size(11)
            .font(SEMIBOLD)
            .style(app_style::accent_text),
        space::horizontal(),
    ]
    .spacing(6)
    .align_y(Alignment::Center);
    if dismissible {
        line = line.push(
            button(text("\u{2715}").size(10))
                .padding([1, 5])
                .style(app_style::quiet_button)
                .on_press(UpdaterMessage::Dismiss),
        );
    }
    line.into()
}

fn downloading(offer: &Offer, progress: Progress) -> Item<'_> {
    let percent = (progress.fraction() * 100.0).round();
    column![
        row![
            text(format!("Downloading {}", offer.release.version))
                .size(13)
                .font(SEMIBOLD),
            space::horizontal(),
            text(format!("{percent}%"))
                .size(12)
                .style(app_style::muted_text),
        ]
        .align_y(Alignment::Center),
        progress_bar(0.0..=1.0, progress.fraction())
            .girth(6)
            .style(style::progress),
        muted_owned(format!(
            "{} of {}",
            rom::human_size(progress.received),
            rom::human_size(progress.total)
        )),
    ]
    .spacing(8)
    .into()
}

fn working<'a>(offer: &'a Offer, verb: &str, detail: &'a str) -> Item<'a> {
    column![
        header("Updating", false),
        text(format!("{verb} randomon {}\u{2026}", offer.release.version))
            .size(13)
            .font(SEMIBOLD),
        muted(detail),
    ]
    .spacing(6)
    .into()
}

fn installed(offer: &Offer) -> Item<'_> {
    column![
        header("Ready", false),
        text(format!("randomon {} is installed", offer.release.version))
            .size(13)
            .font(SEMIBOLD),
        muted("Restart once randomizing has finished."),
        primary("Restart now", UpdaterMessage::Restart),
    ]
    .spacing(8)
    .into()
}

fn failed_body(error: &str) -> Item<'_> {
    let title = row![
        text("Update failed")
            .size(13)
            .font(SEMIBOLD)
            .style(text::danger),
        space::horizontal(),
        button(text("\u{2715}").size(10))
            .padding([1, 5])
            .style(app_style::quiet_button)
            .on_press(UpdaterMessage::Dismiss),
    ]
    .align_y(Alignment::Center);
    Column::new()
        .push(title)
        .push(text(error).size(12))
        .push(
            row![
                secondary("Release page", UpdaterMessage::OpenReleasePage),
                primary("Retry", UpdaterMessage::Update),
            ]
            .spacing(6),
        )
        .spacing(8)
        .into()
}

fn muted(label: &str) -> Item<'_> {
    text(label).size(12).style(app_style::muted_text).into()
}

fn muted_owned<'a>(label: String) -> Item<'a> {
    text(label).size(11).style(app_style::muted_text).into()
}

fn link(label: &str, message: UpdaterMessage) -> Item<'_> {
    button(text(label).size(12))
        .padding(0)
        .style(style::link)
        .on_press(message)
        .into()
}

fn primary(label: &str, message: UpdaterMessage) -> Item<'_> {
    button(text(label).size(12))
        .padding([5, 12])
        .style(app_style::primary_button)
        .on_press(message)
        .into()
}

fn secondary(label: &str, message: UpdaterMessage) -> Item<'_> {
    button(text(label).size(12))
        .padding([5, 12])
        .style(app_style::secondary_button)
        .on_press(message)
        .into()
}
