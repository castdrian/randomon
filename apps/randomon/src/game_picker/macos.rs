use std::path::PathBuf;

use objc2::MainThreadMarker;
use objc2_app_kit::{NSApplication, NSModalResponseOK, NSOpenPanel};
use objc2_foundation::{NSArray, NSString};

pub fn pick_file_or_folder(title: &str, extensions: &[&str]) -> Option<PathBuf> {
    let title = title.to_owned();
    let extensions: Vec<String> = extensions.iter().map(|&value| value.to_owned()).collect();
    dispatch2::run_on_main(move |mtm| run_panel(mtm, &title, &extensions))
}

fn run_panel(mtm: MainThreadMarker, title: &str, extensions: &[String]) -> Option<PathBuf> {
    let key_window = NSApplication::sharedApplication(mtm).keyWindow();
    let panel = NSOpenPanel::openPanel(mtm);
    panel.setCanChooseFiles(true);
    panel.setCanChooseDirectories(true);
    panel.setAllowsMultipleSelection(false);
    panel.setMessage(Some(&NSString::from_str(title)));
    allow_extensions(&panel, extensions);
    let picked = (panel.runModal() == NSModalResponseOK)
        .then(|| panel.URL())
        .flatten()
        .and_then(|url| url.path())
        .map(|path| PathBuf::from(path.to_string()));
    if let Some(window) = key_window {
        window.makeKeyAndOrderFront(None);
    }
    picked
}

#[allow(deprecated)]
fn allow_extensions(panel: &NSOpenPanel, extensions: &[String]) {
    let types: Vec<_> = extensions
        .iter()
        .map(|extension| NSString::from_str(extension))
        .collect();
    panel.setAllowedFileTypes(Some(&NSArray::from_retained_slice(&types)));
}
