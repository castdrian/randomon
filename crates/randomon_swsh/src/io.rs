use std::path::{Path, PathBuf};

use crate::error::{Result, SwshError};

pub(crate) fn read_file(path: &Path) -> Result<Vec<u8>> {
    std::fs::read(path).map_err(|error| match error.kind() {
        std::io::ErrorKind::NotFound => SwshError::MissingFile(path.to_path_buf()),
        _ => SwshError::Io(path.to_path_buf(), error.to_string()),
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RomfsLayers {
    roots: Vec<PathBuf>,
}

impl RomfsLayers {
    pub fn new(base: impl Into<PathBuf>) -> Self {
        RomfsLayers {
            roots: vec![base.into()],
        }
    }

    pub fn with_overlay(mut self, overlay: impl Into<PathBuf>) -> Self {
        self.roots.push(overlay.into());
        self
    }

    pub fn from_roots(roots: &[PathBuf]) -> Option<Self> {
        let (base, overlays) = roots.split_first()?;
        Some(
            overlays
                .iter()
                .fold(RomfsLayers::new(base), |layers, overlay| {
                    layers.with_overlay(overlay)
                }),
        )
    }

    pub fn base(&self) -> &Path {
        &self.roots[0]
    }

    pub fn roots(&self) -> &[PathBuf] {
        &self.roots
    }

    pub fn resolve(&self, relative: &str) -> Option<PathBuf> {
        self.roots
            .iter()
            .rev()
            .map(|root| root.join(relative))
            .find(|path| path.is_file())
    }

    pub fn path_of(&self, relative: &str) -> PathBuf {
        self.resolve(relative)
            .unwrap_or_else(|| self.base().join(relative))
    }

    pub fn read(&self, relative: &str) -> Result<Vec<u8>> {
        read_file(&self.path_of(relative))
    }

    pub fn count_indexed_files(&self, file_for: impl Fn(usize) -> String) -> usize {
        (0..)
            .take_while(|&index| self.resolve(&file_for(index)).is_some())
            .count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn later_layers_override_earlier_ones() {
        let root =
            std::env::temp_dir().join(format!("randomon-swsh-layers-{}", std::process::id()));
        let base = root.join("base");
        let overlay = root.join("overlay");
        for directory in [&base, &overlay] {
            std::fs::create_dir_all(directory.join("bin")).expect("folder");
        }
        std::fs::write(base.join("bin/a.bin"), b"base").expect("write");
        std::fs::write(base.join("bin/b.bin"), b"base").expect("write");
        std::fs::write(overlay.join("bin/a.bin"), b"overlay").expect("write");
        let layers = RomfsLayers::new(&base).with_overlay(&overlay);
        assert_eq!(layers.read("bin/a.bin"), Ok(b"overlay".to_vec()));
        assert_eq!(layers.read("bin/b.bin"), Ok(b"base".to_vec()));
        assert!(matches!(
            layers.read("bin/c.bin"),
            Err(SwshError::MissingFile(_))
        ));
        assert_eq!(
            layers.count_indexed_files(|index| format!("bin/{}.bin", ["a", "b", "c"][index])),
            2
        );
        let _ = std::fs::remove_dir_all(&root);
    }
}
