use std::path::{Path, PathBuf};

use crate::config::environment_directory;

const UNINSTALLER: &str = "uninstall.exe";

pub fn install_directory(executable: &Path) -> Option<PathBuf> {
    let installed_here = executable
        .parent()
        .filter(|directory| directory.join(UNINSTALLER).is_file());
    installed_here
        .map(Path::to_path_buf)
        .or_else(|| environment_directory("LOCALAPPDATA").map(default_directory))
}

pub fn default_directory(local_app_data: PathBuf) -> PathBuf {
    local_app_data.join("Programs").join("randomon")
}

fn powershell_literal(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

pub fn finish_script(process_id: u32, installer: &Path, directory: &Path) -> String {
    let installer = powershell_literal(&installer.display().to_string());
    let arguments = powershell_literal(&format!("/S /D={}", directory.display()));
    let executable = powershell_literal(&directory.join("randomon.exe").display().to_string());
    format!(
        "Wait-Process -Id {process_id} -ErrorAction SilentlyContinue; \
         $setup = Start-Process -FilePath {installer} -ArgumentList {arguments} -Wait -PassThru; \
         Remove-Item -LiteralPath {installer} -ErrorAction SilentlyContinue; \
         if ($setup.ExitCode -eq 0) {{ Start-Process -FilePath {executable} }}"
    )
}

pub fn install(directory: &Path, installer: &Path) -> Result<(), String> {
    let script = finish_script(std::process::id(), installer, directory);
    spawn_powershell(&script).map_err(|error| format!("Couldn't start the installer: {error}"))
}

#[cfg(windows)]
fn spawn_powershell(script: &str) -> std::io::Result<()> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
    std::process::Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-WindowStyle",
            "Hidden",
            "-Command",
            script,
        ])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .creation_flags(CREATE_NO_WINDOW | CREATE_NEW_PROCESS_GROUP)
        .spawn()
        .map(drop)
}

#[cfg(not(windows))]
fn spawn_powershell(_script: &str) -> std::io::Result<()> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "the Windows installer only runs on Windows",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_to_the_per_user_install() {
        assert_eq!(
            default_directory(PathBuf::from("C:/Users/Ash/AppData/Local")),
            PathBuf::from("C:/Users/Ash/AppData/Local/Programs/randomon")
        );
    }

    #[test]
    fn script_waits_installs_silently_and_relaunches() {
        let script = finish_script(
            4242,
            Path::new("C:/Temp/randomon-update/randomon-2.0.0-windows-setup.exe"),
            Path::new("C:/Users/O'Brien/AppData/Local/Programs/randomon"),
        );
        assert!(script.starts_with("Wait-Process -Id 4242"));
        assert!(script.contains("'/S /D=C:/Users/O''Brien/AppData/Local/Programs/randomon'"));
        assert!(script.contains("-Wait -PassThru"));
        assert!(script.contains("randomon/randomon.exe'"));
    }
}
