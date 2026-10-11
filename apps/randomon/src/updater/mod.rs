mod digest;
mod install;
mod network;
mod release;
mod skip;
mod style;
mod view;

use iced::Task;
use semver::Version;

use crate::config::ConfigFile;
use crate::support;
use install::Installer;
use network::{DownloadEvent, Progress};
use release::{Asset, Release, Target};

#[derive(Debug, Clone)]
pub enum UpdaterMessage {
    Check,
    Checked {
        manual: bool,
        result: Result<Release, String>,
    },
    OpenReleasePage,
    Update,
    Skip,
    Dismiss,
    Download(DownloadEvent),
    Installed(Result<(), String>),
    Restart,
}

#[derive(Debug, Clone, PartialEq)]
enum Check {
    Idle,
    Checking { manual: bool },
    UpToDate,
    Failed(String),
}

#[derive(Debug, Clone, PartialEq)]
enum Stage {
    Ready,
    Downloading(Progress),
    Installing,
    Installed,
    Failed(String),
}

#[derive(Debug, Clone, PartialEq)]
struct Offer {
    release: Release,
    stage: Stage,
}

pub struct Updater {
    current: Version,
    config: Option<ConfigFile>,
    installer: Option<Installer>,
    target: Option<Target>,
    check: Check,
    offer: Option<Offer>,
}

impl Updater {
    pub fn new() -> Self {
        Self {
            current: release::current_version(),
            config: ConfigFile::standard(),
            installer: Installer::detect(),
            target: Target::current(),
            check: Check::Idle,
            offer: None,
        }
    }

    pub fn on_launch(&mut self) -> Task<UpdaterMessage> {
        self.start_check(false)
    }

    pub fn preview(&mut self, version: &str, stage: Option<&str>) {
        let Some(version) = release::parse_version(version) else {
            return;
        };
        let asset_name = self
            .target
            .and_then(|target| target.asset_name(&version))
            .unwrap_or_default();
        let release = Release {
            page: format!("https://github.com/castdrian/randomon/releases/tag/v{version}"),
            assets: vec![Asset {
                url: String::new(),
                size: 31_457_280,
                sha256: None,
                name: asset_name,
            }],
            version,
        };
        let stage = match stage {
            Some("downloading") => Stage::Downloading(Progress {
                received: 13_212_057,
                total: 31_457_280,
            }),
            Some("installing") => Stage::Installing,
            Some("installed") => Stage::Installed,
            Some("failed") => Stage::Failed(
                "The download is corrupted: its SHA-256 doesn't match the one GitHub lists."
                    .to_owned(),
            ),
            _ => Stage::Ready,
        };
        match stage {
            Stage::Ready if self.current >= release.version => self.check = Check::UpToDate,
            stage => self.offer = Some(Offer { release, stage }),
        }
    }

    pub fn update(&mut self, message: UpdaterMessage, busy: bool) -> Task<UpdaterMessage> {
        match message {
            UpdaterMessage::Check => return self.start_check(true),
            UpdaterMessage::Checked { manual, result } => self.finish_check(manual, result),
            UpdaterMessage::OpenReleasePage => self.open_release_page(),
            UpdaterMessage::Update => return self.start_update(),
            UpdaterMessage::Skip => self.skip(),
            UpdaterMessage::Dismiss => {
                if self.offer.as_ref().is_some_and(Offer::can_dismiss) {
                    self.offer = None;
                }
            }
            UpdaterMessage::Download(event) => return self.handle_download(event),
            UpdaterMessage::Installed(Ok(())) => {
                self.set_stage(Stage::Installed);
                if !busy {
                    return iced::exit();
                }
            }
            UpdaterMessage::Installed(Err(error)) => self.set_stage(Stage::Failed(error)),
            UpdaterMessage::Restart => return iced::exit(),
        }
        Task::none()
    }

    fn is_working(&self) -> bool {
        matches!(self.check, Check::Checking { .. })
            || self.offer.as_ref().is_some_and(Offer::is_working)
    }

    fn start_check(&mut self, manual: bool) -> Task<UpdaterMessage> {
        if self.is_working() {
            return Task::none();
        }
        self.check = Check::Checking { manual };
        Task::perform(network::latest_release(), move |result| {
            UpdaterMessage::Checked { manual, result }
        })
    }

    fn finish_check(&mut self, manual: bool, result: Result<Release, String>) {
        self.check = Check::Idle;
        match result {
            Ok(release) if release.is_newer_than(&self.current) => {
                if !manual && self.skipped() == Some(release.version.clone()) {
                    return;
                }
                self.offer = Some(Offer {
                    release,
                    stage: Stage::Ready,
                });
            }
            Ok(_) if manual => self.check = Check::UpToDate,
            Ok(_) => {}
            Err(error) if manual => self.check = Check::Failed(error),
            Err(error) => log::info!("update check failed: {error}"),
        }
    }

    fn skipped(&self) -> Option<Version> {
        self.config.as_ref().and_then(skip::skipped_version)
    }

    fn skip(&mut self) {
        let Some(offer) = self.offer.take() else {
            return;
        };
        let saved = match &self.config {
            Some(config) => skip::skip_version(config, &offer.release.version),
            None => Err("randomon couldn't find a folder for its settings.".to_owned()),
        };
        if let Err(error) = saved {
            self.check = Check::Failed(error);
        }
    }

    fn open_release_page(&self) {
        let page = self
            .offer
            .as_ref()
            .map_or(release::RELEASES_PAGE, |offer| offer.release.page.as_str());
        support::open_link(page);
    }

    fn installable(&self) -> Option<(Installer, Asset)> {
        let offer = self.offer.as_ref()?;
        let installer = self.installer.clone()?;
        let asset = offer.release.asset_for(self.target?)?;
        asset.sha256.is_some().then(|| (installer, asset.clone()))
    }

    fn start_update(&mut self) -> Task<UpdaterMessage> {
        if self.offer.as_ref().is_none_or(Offer::is_working) {
            return Task::none();
        }
        let Some((installer, asset)) = self.installable() else {
            self.open_release_page();
            return Task::none();
        };
        self.set_stage(Stage::Downloading(Progress {
            received: 0,
            total: asset.size,
        }));
        let destination = installer.download_path(&asset.name);
        Task::run(
            network::download(asset, destination),
            UpdaterMessage::Download,
        )
    }

    fn handle_download(&mut self, event: DownloadEvent) -> Task<UpdaterMessage> {
        match event {
            DownloadEvent::Progress(progress) => self.set_stage(Stage::Downloading(progress)),
            DownloadEvent::Finished(Err(error)) => self.set_stage(Stage::Failed(error)),
            DownloadEvent::Finished(Ok(path)) => {
                let Some(installer) = self.installer.clone() else {
                    return Task::none();
                };
                self.set_stage(Stage::Installing);
                return Task::perform(install::install(installer, path), UpdaterMessage::Installed);
            }
        }
        Task::none()
    }

    fn set_stage(&mut self, stage: Stage) {
        if let Some(offer) = &mut self.offer {
            offer.stage = stage;
        }
    }
}

impl Offer {
    fn is_working(&self) -> bool {
        matches!(
            self.stage,
            Stage::Downloading(_) | Stage::Installing | Stage::Installed
        )
    }

    fn can_dismiss(&self) -> bool {
        !self.is_working()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn updater(current: &str) -> Updater {
        Updater {
            current: Version::parse(current).unwrap(),
            config: None,
            installer: None,
            target: Some(Target {
                platform: release::Platform::MacOs,
                arch: "aarch64",
            }),
            check: Check::Idle,
            offer: None,
        }
    }

    fn mocked_release() -> Release {
        release::parse_release(release::tests::SAMPLE_RELEASE).unwrap()
    }

    #[test]
    fn offers_newer_releases_from_a_mocked_reply() {
        let mut updater = updater("1.3.2");
        updater.finish_check(false, Ok(mocked_release()));
        let offer = updater.offer.as_ref().unwrap();
        assert_eq!(offer.release.version, Version::new(1, 4, 0));
        assert_eq!(offer.stage, Stage::Ready);
    }

    #[test]
    fn stays_quiet_when_current_or_offline_at_launch() {
        let mut updater = updater("1.4.0");
        updater.finish_check(false, Ok(mocked_release()));
        assert!(updater.offer.is_none());
        assert_eq!(updater.check, Check::Idle);
        updater.finish_check(false, Err("offline".to_owned()));
        assert_eq!(updater.check, Check::Idle);
    }

    #[test]
    fn reports_manual_check_results() {
        let mut updater = updater("1.4.0");
        updater.finish_check(true, Ok(mocked_release()));
        assert_eq!(updater.check, Check::UpToDate);
        updater.finish_check(true, Err("offline".to_owned()));
        assert_eq!(updater.check, Check::Failed("offline".to_owned()));
    }

    #[test]
    fn skipped_versions_stay_hidden_until_checked_manually() {
        let directory =
            std::env::temp_dir().join(format!("randomon-updater-skip-{}", std::process::id()));
        let mut updater = updater("1.3.0");
        updater.config = Some(ConfigFile::at(directory.join("config.txt")));
        updater.finish_check(false, Ok(mocked_release()));
        updater.skip();
        assert!(updater.offer.is_none());
        updater.finish_check(false, Ok(mocked_release()));
        assert!(updater.offer.is_none());
        updater.finish_check(true, Ok(mocked_release()));
        assert!(updater.offer.is_some());
        let _ = std::fs::remove_dir_all(&directory);
    }

    #[test]
    fn only_installs_verifiable_assets_for_a_supported_install() {
        let mut updater = updater("1.3.0");
        updater.finish_check(false, Ok(mocked_release()));
        assert!(updater.installable().is_none());
        updater.installer = Some(Installer::MacBundle("/Applications/randomon.app".into()));
        let (_, asset) = updater.installable().unwrap();
        assert_eq!(asset.name, "randomon-1.4.0-macos.dmg");
        updater.target = Some(Target {
            platform: release::Platform::Linux,
            arch: "aarch64",
        });
        assert!(updater.installable().is_none());
    }

    #[test]
    fn download_failures_are_shown_and_retryable() {
        let mut updater = updater("1.3.0");
        updater.finish_check(false, Ok(mocked_release()));
        let _ = updater.handle_download(DownloadEvent::Finished(Err("corrupted".to_owned())));
        let offer = updater.offer.as_ref().unwrap();
        assert_eq!(offer.stage, Stage::Failed("corrupted".to_owned()));
        assert!(offer.can_dismiss());
    }
}
