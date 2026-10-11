use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use super::{permission_hint, relaunch_when_exited, run_tool};

pub fn bundle_of(executable: &Path) -> Option<PathBuf> {
    executable
        .ancestors()
        .find(|ancestor| {
            ancestor
                .extension()
                .is_some_and(|extension| extension == "app")
        })
        .map(Path::to_path_buf)
}

pub fn install(bundle: &Path, dmg: &Path, scratch: &Path) -> Result<(), String> {
    if bundle.to_string_lossy().contains("/AppTranslocation/") {
        return Err(
            "macOS is running randomon from a temporary location. Move randomon to your Applications folder, open it from there, and try again."
                .to_owned(),
        );
    }
    let mount = scratch.join(format!("mount-{}", std::process::id()));
    std::fs::create_dir_all(&mount)
        .map_err(|error| format!("Couldn't prepare the update: {error}"))?;
    run_tool(
        "/usr/bin/hdiutil",
        &[
            OsStr::new("attach"),
            dmg.as_os_str(),
            OsStr::new("-nobrowse"),
            OsStr::new("-readonly"),
            OsStr::new("-noautoopen"),
            OsStr::new("-mountpoint"),
            mount.as_os_str(),
        ],
    )
    .map_err(|error| format!("Couldn't open the downloaded disk image. {error}"))?;
    let result = install_from(bundle, &mount);
    detach(&mount);
    let _ = std::fs::remove_dir(&mount);
    let _ = std::fs::remove_file(dmg);
    result?;
    relaunch_when_exited("/usr/bin/open", bundle)
}

fn detach(mount: &Path) {
    let quiet = [
        OsStr::new("detach"),
        mount.as_os_str(),
        OsStr::new("-quiet"),
    ];
    if run_tool("/usr/bin/hdiutil", &quiet).is_err() {
        let _ = run_tool(
            "/usr/bin/hdiutil",
            &[
                OsStr::new("detach"),
                mount.as_os_str(),
                OsStr::new("-force"),
            ],
        );
    }
}

fn install_from(bundle: &Path, mount: &Path) -> Result<(), String> {
    let new_bundle = find_bundle(mount)?;
    verify_signature(&new_bundle)?;
    let running_team = team_identifier(bundle)
        .map_err(|error| format!("Couldn't check this copy of randomon's signature. {error}"))?;
    let new_team = team_identifier(&new_bundle)?;
    if running_team != new_team {
        return Err(format!(
            "The update is signed by a different developer ({new_team}, expected {running_team}), so randomon refused to install it."
        ));
    }
    replace_bundle(bundle, &new_bundle)
}

fn find_bundle(mount: &Path) -> Result<PathBuf, String> {
    std::fs::read_dir(mount)
        .map_err(|error| format!("Couldn't read the disk image: {error}"))?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .find(|path| path.extension().is_some_and(|extension| extension == "app"))
        .ok_or_else(|| "The disk image doesn't contain randomon.app.".to_owned())
}

fn verify_signature(bundle: &Path) -> Result<(), String> {
    run_tool(
        "/usr/bin/codesign",
        &[
            OsStr::new("--verify"),
            OsStr::new("--deep"),
            OsStr::new("--strict"),
            bundle.as_os_str(),
        ],
    )
    .map(drop)
    .map_err(|error| {
        format!(
            "The update's code signature isn't valid, so randomon refused to install it. {error}"
        )
    })
}

fn team_identifier(bundle: &Path) -> Result<String, String> {
    let details = run_tool(
        "/usr/bin/codesign",
        &[
            OsStr::new("-d"),
            OsStr::new("--verbose=2"),
            bundle.as_os_str(),
        ],
    )?;
    parse_team_identifier(&details)
        .ok_or_else(|| format!("{} isn't signed by a developer team.", bundle.display()))
}

pub fn parse_team_identifier(details: &str) -> Option<String> {
    details.lines().find_map(|line| {
        let team = line.trim().strip_prefix("TeamIdentifier=")?.trim();
        (!team.is_empty() && team != "not set").then(|| team.to_owned())
    })
}

fn sibling(bundle: &Path, suffix: &str) -> PathBuf {
    let name = bundle
        .file_name()
        .map_or_else(|| "randomon.app".into(), |name| name.to_string_lossy());
    bundle.with_file_name(format!(".{name}.{suffix}"))
}

fn replace_bundle(bundle: &Path, new_bundle: &Path) -> Result<(), String> {
    let location = bundle.parent().unwrap_or(bundle);
    let staged = sibling(bundle, "update");
    let retired = sibling(bundle, "old");
    let _ = std::fs::remove_dir_all(&staged);
    let _ = std::fs::remove_dir_all(&retired);
    std::fs::create_dir(&staged).map_err(|error| permission_hint(&error, location))?;
    if let Err(error) = run_tool(
        "/usr/bin/ditto",
        &[new_bundle.as_os_str(), staged.as_os_str()],
    ) {
        let _ = std::fs::remove_dir_all(&staged);
        return Err(format!("Couldn't copy the update. {error}"));
    }
    swap_into_place(bundle, &staged, &retired).inspect_err(|_| {
        let _ = std::fs::remove_dir_all(&staged);
    })?;
    let _ = std::fs::remove_dir_all(&retired);
    Ok(())
}

pub fn swap_into_place(target: &Path, staged: &Path, retired: &Path) -> Result<(), String> {
    let location = target.parent().unwrap_or(target);
    std::fs::rename(target, retired).map_err(|error| permission_hint(&error, location))?;
    if let Err(error) = std::fs::rename(staged, target) {
        let _ = std::fs::rename(retired, target);
        return Err(permission_hint(&error, location));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_the_enclosing_bundle() {
        assert_eq!(
            bundle_of(Path::new(
                "/Applications/randomon.app/Contents/MacOS/randomon"
            )),
            Some(PathBuf::from("/Applications/randomon.app"))
        );
        assert_eq!(
            bundle_of(Path::new("/Users/me/target/debug/randomon")),
            None
        );
    }

    #[test]
    fn reads_the_team_identifier() {
        let details = "Executable=/Applications/randomon.app/Contents/MacOS/randomon\nIdentifier=dev.adrian.randomon\nAuthority=Developer ID Application: Adrian (ABCDE12345)\nTeamIdentifier=ABCDE12345\n";
        assert_eq!(
            parse_team_identifier(details).as_deref(),
            Some("ABCDE12345")
        );
        assert_eq!(parse_team_identifier("TeamIdentifier=not set\n"), None);
        assert_eq!(parse_team_identifier("Signature=adhoc\n"), None);
    }

    #[test]
    fn swaps_bundles_and_keeps_the_old_one_aside() {
        let root = std::env::temp_dir().join(format!("randomon-swap-{}", std::process::id()));
        let target = root.join("randomon.app");
        let staged = root.join(".randomon.app.update");
        let retired = root.join(".randomon.app.old");
        std::fs::create_dir_all(&target).unwrap();
        std::fs::create_dir_all(&staged).unwrap();
        std::fs::write(target.join("version"), "old").unwrap();
        std::fs::write(staged.join("version"), "new").unwrap();
        swap_into_place(&target, &staged, &retired).unwrap();
        assert_eq!(
            std::fs::read_to_string(target.join("version")).unwrap(),
            "new"
        );
        assert_eq!(
            std::fs::read_to_string(retired.join("version")).unwrap(),
            "old"
        );
        assert!(!staged.exists());
        let _ = std::fs::remove_dir_all(&root);
    }
}
