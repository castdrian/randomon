use randomon_core::{RandomizationError, Result};
use randomon_formats::ctr::GarcArchive;
use randomon_formats::ctr::text::{TextError, read_texts, save_entry};

fn text_error(error: TextError) -> RandomizationError {
    RandomizationError::randomization(error.to_string())
}

fn first_file(garc: &GarcArchive, index: usize) -> Result<&[u8]> {
    garc.file(index)
        .ok_or(RandomizationError::IndexOutOfBounds("textGARC.files"))
}

pub fn read_strings(garc: &GarcArchive, index: usize) -> Result<Vec<String>> {
    read_texts(first_file(garc, index)?).map_err(text_error)
}

pub fn write_strings(garc: &mut GarcArchive, index: usize, strings: &[String]) -> Result<()> {
    first_file(garc, index)?;
    match save_entry(strings) {
        Ok(encoded) => garc
            .set_file(index, encoded)
            .map_err(|error| RandomizationError::Io(error.to_string())),
        Err(TextError::Rejected(_)) => Ok(()),
        Err(error) => Err(text_error(error)),
    }
}
