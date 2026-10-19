use std::collections::BTreeMap;
use std::path::Path;

use crate::error::Result;
use crate::formats::text::TextFile;
use crate::io::RomfsLayers;
use crate::paths;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TextRole {
    SpeciesNames,
    MoveNames,
    MoveDescriptions,
    ItemNames,
    ItemDescriptions,
    AbilityNames,
    AbilityDescriptions,
    TrainerNames,
    TrainerClasses,
    TypeNames,
    NatureNames,
    FormNames,
}

impl TextRole {
    pub const ALL: [TextRole; 12] = [
        TextRole::SpeciesNames,
        TextRole::MoveNames,
        TextRole::MoveDescriptions,
        TextRole::ItemNames,
        TextRole::ItemDescriptions,
        TextRole::AbilityNames,
        TextRole::AbilityDescriptions,
        TextRole::TrainerNames,
        TextRole::TrainerClasses,
        TextRole::TypeNames,
        TextRole::NatureNames,
        TextRole::FormNames,
    ];

    pub fn file_name(self) -> &'static str {
        match self {
            TextRole::SpeciesNames => "monsname.dat",
            TextRole::MoveNames => "wazaname.dat",
            TextRole::MoveDescriptions => "wazainfo.dat",
            TextRole::ItemNames => "itemname.dat",
            TextRole::ItemDescriptions => "iteminfo.dat",
            TextRole::AbilityNames => "tokusei.dat",
            TextRole::AbilityDescriptions => "tokuseiinfo.dat",
            TextRole::TrainerNames => "trname.dat",
            TextRole::TrainerClasses => "trtype.dat",
            TextRole::TypeNames => "typename.dat",
            TextRole::NatureNames => "seikaku.dat",
            TextRole::FormNames => "zkn_form.dat",
        }
    }

    pub fn path(self, language: &str) -> String {
        paths::common_text_file(language, self.file_name())
    }
}

pub fn load_text(romfs: &Path, language: &str, role: TextRole) -> Result<TextFile> {
    load_layered_text(&RomfsLayers::new(romfs), language, role)
}

pub fn load_layered_text(layers: &RomfsLayers, language: &str, role: TextRole) -> Result<TextFile> {
    let relative = role.path(language);
    TextFile::parse(&layers.read(&relative)?)
        .map_err(|error| error.in_file(layers.path_of(&relative)))
}

pub fn load_texts(layers: &RomfsLayers, language: &str) -> Result<BTreeMap<TextRole, TextFile>> {
    TextRole::ALL
        .into_iter()
        .map(|role| Ok((role, load_layered_text(layers, language, role)?)))
        .collect()
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NameLists {
    pub species: Vec<String>,
    pub moves: Vec<String>,
    pub items: Vec<String>,
    pub abilities: Vec<String>,
    pub trainer_names: Vec<String>,
    pub trainer_classes: Vec<String>,
    pub types: Vec<String>,
    pub natures: Vec<String>,
}

impl NameLists {
    pub fn from_texts(texts: &BTreeMap<TextRole, TextFile>) -> Self {
        let strings = |role: TextRole| texts.get(&role).map(TextFile::strings).unwrap_or_default();
        NameLists {
            species: strings(TextRole::SpeciesNames),
            moves: strings(TextRole::MoveNames),
            items: strings(TextRole::ItemNames),
            abilities: strings(TextRole::AbilityNames),
            trainer_names: strings(TextRole::TrainerNames),
            trainer_classes: strings(TextRole::TrainerClasses),
            types: strings(TextRole::TypeNames),
            natures: strings(TextRole::NatureNames),
        }
    }

    pub fn load(romfs: &Path, language: &str) -> Result<Self> {
        let roles = [
            TextRole::SpeciesNames,
            TextRole::MoveNames,
            TextRole::ItemNames,
            TextRole::AbilityNames,
            TextRole::TrainerNames,
            TextRole::TrainerClasses,
            TextRole::TypeNames,
            TextRole::NatureNames,
        ];
        let texts = roles
            .into_iter()
            .map(|role| Ok((role, load_text(romfs, language, role)?)))
            .collect::<Result<BTreeMap<_, _>>>()?;
        Ok(NameLists::from_texts(&texts))
    }

    pub fn load_english(romfs: &Path) -> Result<Self> {
        NameLists::load(romfs, paths::DEFAULT_LANGUAGE)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roles_map_to_common_text_files() {
        assert_eq!(
            TextRole::SpeciesNames.path("English"),
            "bin/message/English/common/monsname.dat"
        );
        assert_eq!(TextRole::AbilityNames.file_name(), "tokusei.dat");
    }

    #[test]
    fn name_lists_come_from_loaded_texts() {
        let mut texts = BTreeMap::new();
        texts.insert(
            TextRole::SpeciesNames,
            TextFile::from_lines(&["Egg", "Bulbasaur"]).expect("text"),
        );
        let names = NameLists::from_texts(&texts);
        assert_eq!(names.species, vec!["Egg", "Bulbasaur"]);
        assert!(names.moves.is_empty());
    }

    #[test]
    fn missing_text_is_an_error() {
        assert!(load_text(Path::new("/nonexistent"), "English", TextRole::MoveNames).is_err());
    }
}
