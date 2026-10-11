use randomon_core::Result;
use randomon_formats::nds::NarcArchive;
use randomon_formats::text::pptxt;
use randomon_nds::format_error;

use crate::handler::Gen5RomHandler;

fn narc_file(narc: &NarcArchive, index: i32) -> Result<&[u8]> {
    usize::try_from(index)
        .ok()
        .and_then(|index| narc.files.get(index))
        .map(Vec::as_slice)
        .ok_or(randomon_core::RandomizationError::IndexOutOfBounds(
            "text NARC entry",
        ))
}

impl Gen5RomHandler {
    fn text_narc(&self, is_story_text: bool) -> &NarcArchive {
        if is_story_text {
            &self.story_text_narc
        } else {
            &self.strings_narc
        }
    }

    pub(crate) fn get_strings(&self, is_story_text: bool, index: i32) -> Result<Vec<String>> {
        let raw = narc_file(self.text_narc(is_story_text), index)?;
        pptxt::read_texts(raw).map_err(format_error)
    }

    pub(crate) fn set_strings(
        &mut self,
        is_story_text: bool,
        index: i32,
        strings: &[String],
    ) -> Result<()> {
        let narc = if is_story_text {
            &mut self.story_text_narc
        } else {
            &mut self.strings_narc
        };
        let original = narc_file(narc, index)?;
        let encoded = pptxt::save_entry(original, strings).map_err(format_error)?;
        narc.files[index as usize] = encoded;
        Ok(())
    }

    pub(crate) fn replace_all_strings_in_entry(
        &mut self,
        entry: i32,
        replacements: &std::collections::BTreeMap<String, String>,
    ) -> Result<()> {
        let mut strings = self.get_strings(true, entry)?;
        for string in &mut strings {
            for (old, new) in replacements {
                *string = crate::trades::java_replace_all(string, old, new)?;
            }
        }
        self.set_strings(true, entry, &strings)
    }
}
