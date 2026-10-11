use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

use iced::futures::{SinkExt, Stream};

use super::digest;
use super::release::{self, Asset, Release};

const USER_AGENT: &str = concat!("randomon/", env!("CARGO_PKG_VERSION"));
const PROGRESS_STEP: u64 = 256 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Progress {
    pub received: u64,
    pub total: u64,
}

impl Progress {
    pub fn fraction(self) -> f32 {
        if self.total == 0 {
            0.0
        } else {
            (self.received as f64 / self.total as f64).clamp(0.0, 1.0) as f32
        }
    }
}

#[derive(Debug, Clone)]
pub enum DownloadEvent {
    Progress(Progress),
    Finished(Result<PathBuf, String>),
}

fn agent(body_timeout: Duration) -> ureq::Agent {
    let config = ureq::Agent::config_builder()
        .user_agent(USER_AGENT)
        .timeout_connect(Some(Duration::from_secs(10)))
        .timeout_recv_response(Some(Duration::from_secs(20)))
        .timeout_recv_body(Some(body_timeout));
    #[cfg(target_os = "windows")]
    let config = config.tls_config(
        ureq::tls::TlsConfig::builder()
            .provider(ureq::tls::TlsProvider::NativeTls)
            .build(),
    );
    config.build().into()
}

fn describe(error: &ureq::Error) -> String {
    match error {
        ureq::Error::StatusCode(403 | 429) => {
            "GitHub is limiting update checks right now. Try again in a little while.".to_owned()
        }
        ureq::Error::StatusCode(404) => "No randomon release is published yet.".to_owned(),
        ureq::Error::StatusCode(code) => format!("GitHub answered with an error ({code})."),
        ureq::Error::Timeout(_) => "GitHub took too long to answer.".to_owned(),
        ureq::Error::HostNotFound | ureq::Error::ConnectionFailed | ureq::Error::Io(_) => {
            "Couldn't reach GitHub. Check your internet connection.".to_owned()
        }
        other => format!("Couldn't reach GitHub: {other}"),
    }
}

fn fetch_latest_release() -> Result<Release, String> {
    let body = agent(Duration::from_secs(20))
        .get(release::LATEST_RELEASE_URL)
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .call()
        .map_err(|error| describe(&error))?
        .into_body()
        .read_to_string()
        .map_err(|error| describe(&error))?;
    release::parse_release(&body)
}

pub async fn latest_release() -> Result<Release, String> {
    tokio::task::spawn_blocking(fetch_latest_release)
        .await
        .unwrap_or_else(|error| Err(format!("The update check stopped unexpectedly: {error}")))
}

pub fn copy_with_progress(
    mut reader: impl Read,
    mut writer: impl Write,
    total: u64,
    mut report: impl FnMut(Progress),
) -> std::io::Result<u64> {
    let mut buffer = vec![0u8; 64 * 1024];
    let mut received = 0u64;
    let mut reported = 0u64;
    loop {
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            writer.flush()?;
            report(Progress { received, total });
            return Ok(received);
        }
        writer.write_all(&buffer[..read])?;
        received += read as u64;
        if received - reported >= PROGRESS_STEP {
            reported = received;
            report(Progress { received, total });
        }
    }
}

pub fn save_verified(
    reader: impl Read,
    asset: &Asset,
    destination: &Path,
    report: impl FnMut(Progress),
) -> Result<(), String> {
    let expected = asset
        .sha256
        .ok_or("This release doesn't list a checksum for its download, so randomon won't install it automatically.")?;
    let result = write_and_verify(reader, asset, destination, expected, report);
    if result.is_err() {
        let _ = std::fs::remove_file(destination);
    }
    result
}

fn write_and_verify(
    reader: impl Read,
    asset: &Asset,
    destination: &Path,
    expected: digest::Sha256,
    report: impl FnMut(Progress),
) -> Result<(), String> {
    let saving = |error: std::io::Error| format!("Couldn't save the download: {error}");
    if let Some(directory) = destination.parent() {
        std::fs::create_dir_all(directory).map_err(saving)?;
    }
    let file = std::fs::File::create(destination).map_err(saving)?;
    let received = copy_with_progress(reader, file, asset.size, report)
        .map_err(|error| format!("The download was interrupted: {error}"))?;
    if asset.size != 0 && received != asset.size {
        return Err(format!(
            "The download is incomplete: got {received} of {} bytes.",
            asset.size
        ));
    }
    let actual = digest::of_file(destination).map_err(saving)?;
    digest::verify(actual, expected)
}

fn download_blocking(
    asset: &Asset,
    destination: &Path,
    report: impl FnMut(Progress),
) -> Result<(), String> {
    let response = agent(Duration::from_secs(60 * 60))
        .get(&asset.url)
        .call()
        .map_err(|error| describe(&error))?;
    let reader = response.into_body().into_reader();
    save_verified(reader, asset, destination, report)
}

pub fn download(asset: Asset, destination: PathBuf) -> impl Stream<Item = DownloadEvent> {
    iced::stream::channel(32, async move |mut output| {
        let (sender, mut receiver) = tokio::sync::mpsc::channel(32);
        let worker = tokio::task::spawn_blocking(move || {
            download_blocking(&asset, &destination, |progress| {
                let _ = sender.blocking_send(progress);
            })
            .map(|()| destination)
        });
        while let Some(progress) = receiver.recv().await {
            let _ = output.send(DownloadEvent::Progress(progress)).await;
        }
        let result = worker
            .await
            .unwrap_or_else(|error| Err(format!("The download stopped unexpectedly: {error}")));
        let _ = output.send(DownloadEvent::Finished(result)).await;
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn asset(content: &[u8], digest: Option<&str>) -> Asset {
        Asset {
            name: "randomon-9.9.9-macos.dmg".to_owned(),
            url: "https://example.invalid/randomon.dmg".to_owned(),
            size: content.len() as u64,
            sha256: digest.and_then(digest::parse_github_digest),
        }
    }

    fn scratch(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "randomon-network-{}-{name}/download.bin",
            std::process::id()
        ))
    }

    const ABC: &str = "sha256:ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";

    #[test]
    fn reports_progress_through_the_end() {
        let data = vec![7u8; 600 * 1024];
        let mut seen = Vec::new();
        let mut sink = Vec::new();
        let copied =
            copy_with_progress(&data[..], &mut sink, data.len() as u64, |p| seen.push(p)).unwrap();
        assert_eq!(copied, data.len() as u64);
        assert_eq!(sink, data);
        assert!(seen.len() >= 3);
        assert_eq!(seen.last().unwrap().fraction(), 1.0);
        assert!(
            seen.windows(2)
                .all(|pair| pair[0].received <= pair[1].received)
        );
    }

    #[test]
    fn keeps_downloads_that_match_the_digest() {
        let path = scratch("good");
        save_verified(&b"abc"[..], &asset(b"abc", Some(ABC)), &path, |_| {}).unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"abc");
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn deletes_downloads_that_dont_match() {
        let path = scratch("bad");
        let error =
            save_verified(&b"abd"[..], &asset(b"abd", Some(ABC)), &path, |_| {}).unwrap_err();
        assert!(error.contains("corrupted"));
        assert!(!path.exists());
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn refuses_downloads_without_a_digest_or_with_the_wrong_size() {
        let path = scratch("missing");
        assert!(save_verified(&b"abc"[..], &asset(b"abc", None), &path, |_| {}).is_err());
        let mut short = asset(b"abc", Some(ABC));
        short.size = 10;
        let error = save_verified(&b"abc"[..], &short, &path, |_| {}).unwrap_err();
        assert!(error.contains("incomplete"));
        assert!(!path.exists());
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }
}
