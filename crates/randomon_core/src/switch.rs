use crate::handler::OutputFile;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SwitchLayout {
    #[default]
    Atmosphere,
    Eden,
    Ryubing,
    Plain,
}

impl SwitchLayout {
    pub const ALL: [SwitchLayout; 4] = [
        SwitchLayout::Atmosphere,
        SwitchLayout::Eden,
        SwitchLayout::Ryubing,
        SwitchLayout::Plain,
    ];

    pub fn parse(text: &str) -> Option<Self> {
        match text.to_ascii_lowercase().as_str() {
            "atmosphere" | "switch" => Some(Self::Atmosphere),
            "eden" | "citron" | "sudachi" | "suyu" | "yuzu" => Some(Self::Eden),
            "ryubing" | "ryujinx" => Some(Self::Ryubing),
            "plain" | "emulator" => Some(Self::Plain),
            _ => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Atmosphere => "Atmosphere (Switch)",
            Self::Eden => "Eden, Citron and other yuzu-based emulators",
            Self::Ryubing => "Ryubing (Ryujinx)",
            Self::Plain => "Plain romfs/exefs folders",
        }
    }

    pub fn install_hint(self) -> &'static str {
        match self {
            Self::Atmosphere => "Copy the atmosphere folder to the root of your Switch's SD card.",
            Self::Eden => {
                "Copy the title ID folder into your emulator's load folder (right-click the game, then Open Mod Data Location)."
            }
            Self::Ryubing => {
                "Copy the mods folder into your Ryubing data folder (File, then Open Ryujinx Folder)."
            }
            Self::Plain => "Point your emulator's mod loader at the romfs and exefs folders.",
        }
    }

    pub fn short_label(self) -> &'static str {
        match self {
            Self::Atmosphere => "Atmosphere",
            Self::Eden => "Eden / yuzu",
            Self::Ryubing => "Ryubing",
            Self::Plain => "Plain folders",
        }
    }

    fn roots(self, title_id: &str, exefs_patch_folder: &str) -> (String, String) {
        match self {
            Self::Atmosphere => (
                format!("atmosphere/contents/{title_id}/romfs"),
                format!("atmosphere/exefs_patches/{exefs_patch_folder}"),
            ),
            Self::Eden => (format!("{title_id}/romfs"), format!("{title_id}/exefs")),
            Self::Ryubing => (
                format!("mods/contents/{title_id}/romfs"),
                format!("mods/contents/{title_id}/exefs"),
            ),
            Self::Plain => ("romfs".to_owned(), "exefs".to_owned()),
        }
    }
}

impl std::fmt::Display for SwitchLayout {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.short_label())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SwitchMod {
    pub title_ids: Vec<String>,
    pub exefs_patch_folder: String,
    pub romfs_files: Vec<(String, Vec<u8>)>,
    pub exefs_patches: Vec<(String, Vec<u8>)>,
}

impl SwitchMod {
    pub fn files(&self, layout: SwitchLayout) -> Vec<OutputFile> {
        let titles: Vec<&String> = match layout {
            SwitchLayout::Plain => self.title_ids.iter().take(1).collect(),
            _ => self.title_ids.iter().collect(),
        };
        let mut files: Vec<OutputFile> = Vec::new();
        for title_id in titles {
            let (romfs_root, exefs_root) = layout.roots(title_id, &self.exefs_patch_folder);
            for (relative, bytes) in &self.romfs_files {
                files.push(OutputFile {
                    path: format!("{romfs_root}/{}", relative.trim_start_matches('/')),
                    bytes: bytes.clone(),
                });
            }
            for (build_id, bytes) in &self.exefs_patches {
                let path = format!("{exefs_root}/{build_id}.ips");
                if !files.iter().any(|file| file.path == path) {
                    files.push(OutputFile {
                        path,
                        bytes: bytes.clone(),
                    });
                }
            }
        }
        files
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Ips32Patch {
    records: Vec<(u32, Vec<u8>)>,
}

impl Ips32Patch {
    pub fn add(&mut self, offset: u32, bytes: &[u8]) {
        self.records.push((offset, bytes.to_vec()));
    }

    pub fn records(&self) -> &[(u32, Vec<u8>)] {
        &self.records
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = b"IPS32".to_vec();
        for (offset, bytes) in &self.records {
            out.extend_from_slice(&offset.to_be_bytes());
            out.extend_from_slice(&(bytes.len() as u16).to_be_bytes());
            out.extend_from_slice(bytes);
        }
        out.extend_from_slice(b"EEOF");
        out
    }

    pub fn parse(data: &[u8]) -> Option<Self> {
        let mut rest = data.strip_prefix(b"IPS32")?;
        let mut patch = Self::default();
        loop {
            if rest == b"EEOF" {
                return Some(patch);
            }
            if rest.len() < 6 {
                return None;
            }
            let offset = u32::from_be_bytes([rest[0], rest[1], rest[2], rest[3]]);
            let length = usize::from(u16::from_be_bytes([rest[4], rest[5]]));
            let bytes = rest.get(6..6 + length)?;
            patch.add(offset, bytes);
            rest = &rest[6 + length..];
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> SwitchMod {
        SwitchMod {
            title_ids: vec!["0100000011D90000".to_owned(), "010018E011D92000".to_owned()],
            exefs_patch_folder: "randomon_bdsp".to_owned(),
            romfs_files: vec![("Data/a.bundle".to_owned(), vec![1])],
            exefs_patches: vec![("ABCD".to_owned(), vec![2])],
        }
    }

    fn paths(layout: SwitchLayout) -> Vec<String> {
        sample()
            .files(layout)
            .into_iter()
            .map(|file| file.path)
            .collect()
    }

    #[test]
    fn atmosphere_shares_one_exefs_patch_folder_between_titles() {
        assert_eq!(
            paths(SwitchLayout::Atmosphere),
            [
                "atmosphere/contents/0100000011D90000/romfs/Data/a.bundle",
                "atmosphere/exefs_patches/randomon_bdsp/ABCD.ips",
                "atmosphere/contents/010018E011D92000/romfs/Data/a.bundle",
            ]
        );
    }

    #[test]
    fn emulator_layouts_put_romfs_and_exefs_under_the_title_id() {
        assert_eq!(
            paths(SwitchLayout::Eden)[..2],
            [
                "0100000011D90000/romfs/Data/a.bundle",
                "0100000011D90000/exefs/ABCD.ips",
            ]
        );
        assert_eq!(
            paths(SwitchLayout::Ryubing)[..2],
            [
                "mods/contents/0100000011D90000/romfs/Data/a.bundle",
                "mods/contents/0100000011D90000/exefs/ABCD.ips",
            ]
        );
    }

    #[test]
    fn plain_folders_only_cover_one_title() {
        assert_eq!(
            paths(SwitchLayout::Plain),
            ["romfs/Data/a.bundle", "exefs/ABCD.ips"]
        );
    }

    #[test]
    fn layouts_parse_from_emulator_names() {
        assert_eq!(SwitchLayout::parse("ryujinx"), Some(SwitchLayout::Ryubing));
        assert_eq!(SwitchLayout::parse("Yuzu"), Some(SwitchLayout::Eden));
        assert_eq!(SwitchLayout::parse("nonsense"), None);
    }

    #[test]
    fn ips32_patches_round_trip() {
        let mut patch = Ips32Patch::default();
        patch.add(0x100, &[1, 2, 3, 4]);
        let parsed = Ips32Patch::parse(&patch.to_bytes()).expect("valid patch");
        assert_eq!(parsed, patch);
    }
}
