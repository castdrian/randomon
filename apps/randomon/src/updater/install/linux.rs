use std::path::{Path, PathBuf};

use super::{permission_hint, relaunch_when_exited};
use crate::config::environment_directory;

pub fn running_appimage() -> Option<PathBuf> {
    environment_directory("APPIMAGE").filter(|path| path.is_file())
}

pub fn staging_path(appimage: &Path, asset_name: &str) -> PathBuf {
    appimage.with_file_name(format!(".{asset_name}.download"))
}

pub fn install(appimage: &Path, downloaded: &Path) -> Result<(), String> {
    let location = appimage.parent().unwrap_or(appimage);
    let replaced = make_executable(downloaded)
        .and_then(|()| std::fs::rename(downloaded, appimage))
        .map_err(|error| permission_hint(&error, location));
    if replaced.is_err() {
        let _ = std::fs::remove_file(downloaded);
    }
    replaced?;
    relaunch_when_exited("", appimage)
}

#[cfg(unix)]
fn make_executable(path: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))
}

#[cfg(not(unix))]
fn make_executable(_path: &Path) -> std::io::Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stages_next_to_the_appimage() {
        assert_eq!(
            staging_path(
                Path::new("/home/me/Apps/randomon.AppImage"),
                "randomon-2.0.0-linux-x86_64.AppImage"
            ),
            PathBuf::from("/home/me/Apps/.randomon-2.0.0-linux-x86_64.AppImage.download")
        );
    }

    #[cfg(unix)]
    #[test]
    fn replaces_the_appimage_with_an_executable_file() {
        use std::os::unix::fs::PermissionsExt;
        let root = std::env::temp_dir().join(format!("randomon-appimage-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let appimage = root.join("randomon.AppImage");
        let downloaded = staging_path(&appimage, "randomon-2.0.0-linux-x86_64.AppImage");
        std::fs::write(&appimage, "old").unwrap();
        std::fs::write(&downloaded, "new").unwrap();
        make_executable(&downloaded).unwrap();
        std::fs::rename(&downloaded, &appimage).unwrap();
        assert_eq!(std::fs::read_to_string(&appimage).unwrap(), "new");
        let mode = std::fs::metadata(&appimage).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o755);
        assert!(!downloaded.exists());
        let _ = std::fs::remove_dir_all(&root);
    }
}
