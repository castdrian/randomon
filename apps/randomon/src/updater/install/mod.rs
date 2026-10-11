mod linux;
mod macos;
mod windows;

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Installer {
    MacBundle(PathBuf),
    WindowsSetup(PathBuf),
    AppImage(PathBuf),
}

impl Installer {
    pub fn detect() -> Option<Self> {
        let executable = std::env::current_exe().ok()?;
        if cfg!(target_os = "macos") {
            macos::bundle_of(&executable).map(Self::MacBundle)
        } else if cfg!(target_os = "windows") {
            windows::install_directory(&executable).map(Self::WindowsSetup)
        } else if cfg!(target_os = "linux") {
            linux::running_appimage().map(Self::AppImage)
        } else {
            None
        }
    }

    pub fn download_path(&self, asset_name: &str) -> PathBuf {
        match self {
            Self::AppImage(appimage) => linux::staging_path(appimage, asset_name),
            Self::MacBundle(_) | Self::WindowsSetup(_) => scratch_directory().join(asset_name),
        }
    }

    fn install(&self, downloaded: &Path) -> Result<(), String> {
        match self {
            Self::MacBundle(bundle) => macos::install(bundle, downloaded, &scratch_directory()),
            Self::WindowsSetup(directory) => windows::install(directory, downloaded),
            Self::AppImage(appimage) => linux::install(appimage, downloaded),
        }
    }
}

pub async fn install(installer: Installer, downloaded: PathBuf) -> Result<(), String> {
    tokio::task::spawn_blocking(move || installer.install(&downloaded))
        .await
        .unwrap_or_else(|error| Err(format!("The installation stopped unexpectedly: {error}")))
}

fn scratch_directory() -> PathBuf {
    std::env::temp_dir().join("randomon-update")
}

fn run_tool(program: &str, arguments: &[&std::ffi::OsStr]) -> Result<String, String> {
    let output = Command::new(program)
        .args(arguments)
        .stdin(Stdio::null())
        .output()
        .map_err(|error| format!("Couldn't run {program}: {error}"))?;
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    if output.status.success() {
        Ok(stdout + &stderr)
    } else {
        Err(format!("{program} failed: {}", stderr.trim()))
    }
}

#[cfg(unix)]
fn relaunch_when_exited(launch: &str, target: &Path) -> Result<(), String> {
    use std::os::unix::process::CommandExt;
    let script =
        format!("while kill -0 \"$1\" 2>/dev/null; do sleep 0.2; done; exec {launch} \"$2\"");
    Command::new("/bin/sh")
        .arg("-c")
        .arg(script)
        .arg("sh")
        .arg(std::process::id().to_string())
        .arg(target)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .process_group(0)
        .spawn()
        .map(drop)
        .map_err(|error| format!("Installed, but couldn't restart randomon: {error}"))
}

#[cfg(not(unix))]
fn relaunch_when_exited(_launch: &str, _target: &Path) -> Result<(), String> {
    Err("Restarting is not supported on this platform.".to_owned())
}

fn permission_hint(error: &std::io::Error, location: &Path) -> String {
    if error.kind() == std::io::ErrorKind::PermissionDenied {
        format!(
            "randomon isn't allowed to change {}. Download the update from the release page instead, or move randomon to a folder you own.",
            location.display()
        )
    } else {
        format!("Couldn't replace {}: {error}", location.display())
    }
}
