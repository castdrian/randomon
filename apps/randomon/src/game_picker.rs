#[cfg(target_os = "macos")]
mod macos;

use std::path::PathBuf;

use crate::rom;

const TITLE: &str = "Open game or romfs folder";

#[cfg(target_os = "macos")]
pub async fn pick_game() -> Option<PathBuf> {
    tokio::task::spawn_blocking(|| macos::pick_file_or_folder(TITLE, rom::SUPPORTED_EXTENSIONS))
        .await
        .ok()
        .flatten()
}

#[cfg(not(target_os = "macos"))]
pub async fn pick_game() -> Option<PathBuf> {
    rfd::AsyncFileDialog::new()
        .set_title(TITLE)
        .add_filter("Pokémon ROMs", rom::SUPPORTED_EXTENSIONS)
        .add_filter("Any file inside a romfs folder", &["*"])
        .pick_file()
        .await
        .map(|handle| handle.path().to_path_buf())
}
