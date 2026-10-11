use semver::Version;
use serde::Deserialize;

use super::digest::{self, Sha256};

pub const RELEASES_PAGE: &str = "https://github.com/castdrian/randomon/releases/latest";
pub const LATEST_RELEASE_URL: &str =
    "https://api.github.com/repos/castdrian/randomon/releases/latest";

#[derive(Debug, Deserialize)]
struct GithubRelease {
    tag_name: String,
    html_url: String,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    prerelease: bool,
    #[serde(default)]
    assets: Vec<GithubAsset>,
}

#[derive(Debug, Deserialize)]
struct GithubAsset {
    name: String,
    browser_download_url: String,
    size: u64,
    #[serde(default)]
    digest: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Release {
    pub version: Version,
    pub page: String,
    pub assets: Vec<Asset>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Asset {
    pub name: String,
    pub url: String,
    pub size: u64,
    pub sha256: Option<Sha256>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    MacOs,
    Windows,
    Linux,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Target {
    pub platform: Platform,
    pub arch: &'static str,
}

impl Target {
    pub fn current() -> Option<Self> {
        let platform = match std::env::consts::OS {
            "macos" => Platform::MacOs,
            "windows" => Platform::Windows,
            "linux" => Platform::Linux,
            _ => return None,
        };
        Some(Self {
            platform,
            arch: std::env::consts::ARCH,
        })
    }

    pub fn asset_name(&self, version: &Version) -> Option<String> {
        let suffix = match (self.platform, self.arch) {
            (Platform::MacOs, _) => "macos.dmg".to_owned(),
            (Platform::Windows, "x86_64" | "aarch64") => "windows-setup.exe".to_owned(),
            (Platform::Linux, arch @ ("x86_64" | "aarch64")) => format!("linux-{arch}.AppImage"),
            _ => return None,
        };
        Some(format!("randomon-{version}-{suffix}"))
    }
}

pub fn current_version() -> Version {
    Version::parse(env!("CARGO_PKG_VERSION")).unwrap_or_else(|_| Version::new(0, 0, 0))
}

pub fn parse_version(tag: &str) -> Option<Version> {
    let trimmed = tag.trim();
    let bare = trimmed
        .strip_prefix('v')
        .or_else(|| trimmed.strip_prefix('V'))
        .unwrap_or(trimmed);
    Version::parse(bare).ok()
}

pub fn parse_release(json: &str) -> Result<Release, String> {
    let release: GithubRelease = serde_json::from_str(json)
        .map_err(|error| format!("GitHub sent an unexpected reply: {error}"))?;
    if release.draft || release.prerelease {
        return Err("The latest GitHub release isn't a stable release.".to_owned());
    }
    let version = parse_version(&release.tag_name).ok_or_else(|| {
        format!(
            "The latest release tag \u{201C}{}\u{201D} isn't a version number.",
            release.tag_name
        )
    })?;
    let assets = release
        .assets
        .into_iter()
        .map(|asset| Asset {
            sha256: asset
                .digest
                .as_deref()
                .and_then(digest::parse_github_digest),
            name: asset.name,
            url: asset.browser_download_url,
            size: asset.size,
        })
        .collect();
    Ok(Release {
        version,
        page: release.html_url,
        assets,
    })
}

impl Release {
    pub fn is_newer_than(&self, current: &Version) -> bool {
        self.version > *current
    }

    pub fn asset_for(&self, target: Target) -> Option<&Asset> {
        let name = target.asset_name(&self.version)?;
        self.assets.iter().find(|asset| asset.name == name)
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    pub const SAMPLE_RELEASE: &str = r#"{
        "tag_name": "v1.4.0",
        "html_url": "https://github.com/castdrian/randomon/releases/tag/v1.4.0",
        "draft": false,
        "prerelease": false,
        "assets": [
            {
                "name": "randomon-1.4.0-macos.dmg",
                "browser_download_url": "https://github.com/castdrian/randomon/releases/download/v1.4.0/randomon-1.4.0-macos.dmg",
                "size": 31000000,
                "digest": "sha256:ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
            },
            {
                "name": "randomon-1.4.0-windows-setup.exe",
                "browser_download_url": "https://github.com/castdrian/randomon/releases/download/v1.4.0/randomon-1.4.0-windows-setup.exe",
                "size": 12000000,
                "digest": "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
            },
            {
                "name": "randomon-1.4.0-linux-x86_64.AppImage",
                "browser_download_url": "https://github.com/castdrian/randomon/releases/download/v1.4.0/randomon-1.4.0-linux-x86_64.AppImage",
                "size": 20000000,
                "digest": null
            },
            {
                "name": "randomon-1.4.0-linux-aarch64.AppImage",
                "browser_download_url": "https://github.com/castdrian/randomon/releases/download/v1.4.0/randomon-1.4.0-linux-aarch64.AppImage",
                "size": 19000000
            }
        ]
    }"#;

    fn sample() -> Release {
        parse_release(SAMPLE_RELEASE).unwrap()
    }

    fn target(platform: Platform, arch: &'static str) -> Target {
        Target { platform, arch }
    }

    #[test]
    fn parses_the_github_release_reply() {
        let release = sample();
        assert_eq!(release.version, Version::new(1, 4, 0));
        assert_eq!(
            release.page,
            "https://github.com/castdrian/randomon/releases/tag/v1.4.0"
        );
        assert_eq!(release.assets.len(), 4);
        assert!(release.assets[0].sha256.is_some());
        assert!(release.assets[2].sha256.is_none());
        assert!(release.assets[3].sha256.is_none());
    }

    #[test]
    fn rejects_prereleases_and_bad_tags() {
        let prerelease = SAMPLE_RELEASE.replace(r#""prerelease": false"#, r#""prerelease": true"#);
        assert!(parse_release(&prerelease).is_err());
        let bad_tag = SAMPLE_RELEASE.replace("\"v1.4.0\"", "\"latest\"");
        assert!(parse_release(&bad_tag).is_err());
        assert!(parse_release("{\"message\": \"Not Found\"}").is_err());
    }

    #[test]
    fn compares_versions_semantically() {
        let release = sample();
        assert!(release.is_newer_than(&Version::new(1, 3, 9)));
        assert!(release.is_newer_than(&Version::new(1, 3, 10)));
        assert!(!release.is_newer_than(&Version::new(1, 4, 0)));
        assert!(!release.is_newer_than(&Version::new(1, 10, 0)));
        assert!(!release.is_newer_than(&Version::new(2, 0, 0)));
        assert!(Version::parse("1.4.0-beta.1").unwrap() < release.version);
        assert_eq!(parse_version("v1.2.3"), Some(Version::new(1, 2, 3)));
        assert_eq!(parse_version("1.2.3"), Some(Version::new(1, 2, 3)));
        assert_eq!(parse_version("v1.2"), None);
    }

    #[test]
    fn picks_the_asset_for_each_platform() {
        let release = sample();
        let name = |platform, arch| {
            release
                .asset_for(target(platform, arch))
                .map(|asset| asset.name.as_str())
        };
        assert_eq!(
            name(Platform::MacOs, "aarch64"),
            Some("randomon-1.4.0-macos.dmg")
        );
        assert_eq!(
            name(Platform::MacOs, "x86_64"),
            Some("randomon-1.4.0-macos.dmg")
        );
        assert_eq!(
            name(Platform::Windows, "x86_64"),
            Some("randomon-1.4.0-windows-setup.exe")
        );
        assert_eq!(
            name(Platform::Windows, "aarch64"),
            Some("randomon-1.4.0-windows-setup.exe")
        );
        assert_eq!(
            name(Platform::Linux, "x86_64"),
            Some("randomon-1.4.0-linux-x86_64.AppImage")
        );
        assert_eq!(
            name(Platform::Linux, "aarch64"),
            Some("randomon-1.4.0-linux-aarch64.AppImage")
        );
        assert_eq!(name(Platform::Linux, "riscv64"), None);
        assert_eq!(name(Platform::Windows, "x86"), None);
    }

    #[test]
    fn ignores_assets_from_other_versions() {
        let mut release = sample();
        release.version = Version::new(1, 5, 0);
        assert!(
            release
                .asset_for(target(Platform::MacOs, "aarch64"))
                .is_none()
        );
    }
}
