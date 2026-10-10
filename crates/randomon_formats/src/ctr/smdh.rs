use crate::bytes::read_long;
use crate::error::{FormatError, Result};
use crate::java::strings::{decode_utf16le, encode_utf16le, java_trim};

const CONTEXT: &str = "SMDH";
const SMDH_MAGIC: i32 = 0x48444D53;
const TITLE_LENGTH: usize = 0x200;
const SHORT_DESCRIPTION_LENGTH: usize = 0x80;
const LONG_DESCRIPTION_LENGTH: usize = 0x100;
const PUBLISHER_LENGTH: usize = 0x80;
const TITLE_COUNT: usize = 12;
const SHORT_DESCRIPTION_BASE: usize = 0x08;
const LONG_DESCRIPTION_BASE: usize = 0x88;
const PUBLISHER_BASE: usize = 0x188;

#[derive(Debug, Clone, PartialEq, Eq)]
struct Titles {
    short_descriptions: Vec<String>,
    long_descriptions: Vec<String>,
    publishers: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Smdh {
    data: Vec<u8>,
    titles: Option<Titles>,
}

impl Smdh {
    pub fn parse(data: Vec<u8>) -> Result<Self> {
        if data.len() < 4 {
            return Err(FormatError::out_of_bounds(CONTEXT));
        }
        let titles = if read_long(&data, 0) == SMDH_MAGIC {
            Some(read_titles(&data)?)
        } else {
            None
        };
        Ok(Smdh { data, titles })
    }

    pub fn bytes(&self) -> &[u8] {
        &self.data
    }

    pub fn is_valid(&self) -> bool {
        self.titles.is_some()
    }

    pub fn short_descriptions(&self) -> Option<&[String]> {
        self.titles
            .as_ref()
            .map(|titles| titles.short_descriptions.as_slice())
    }

    pub fn long_descriptions(&self) -> Option<&[String]> {
        self.titles
            .as_ref()
            .map(|titles| titles.long_descriptions.as_slice())
    }

    pub fn publishers(&self) -> Option<&[String]> {
        self.titles
            .as_ref()
            .map(|titles| titles.publishers.as_slice())
    }

    pub fn set_all_descriptions(&mut self, description: &str) -> Result<()> {
        if encode_utf16le(description).len() > SHORT_DESCRIPTION_LENGTH {
            return Ok(());
        }
        let titles = self.titles_mut()?;
        titles.short_descriptions = vec![description.to_owned(); TITLE_COUNT];
        titles.long_descriptions = vec![description.to_owned(); TITLE_COUNT];
        self.write_titles()
    }

    pub fn set_all_publishers(&mut self, publisher: &str) -> Result<()> {
        if encode_utf16le(publisher).len() > PUBLISHER_LENGTH {
            return Ok(());
        }
        self.titles_mut()?.publishers = vec![publisher.to_owned(); TITLE_COUNT];
        self.write_titles()
    }

    fn titles_mut(&mut self) -> Result<&mut Titles> {
        self.titles.as_mut().ok_or_else(|| {
            FormatError::invalid(CONTEXT, "descriptions of an invalid SMDH are null")
        })
    }

    fn write_titles(&mut self) -> Result<()> {
        let Some(titles) = &self.titles else {
            return Err(FormatError::invalid(
                CONTEXT,
                "descriptions of an invalid SMDH are null",
            ));
        };
        for index in 0..TITLE_COUNT {
            let base = TITLE_LENGTH * index;
            write_field(
                &mut self.data,
                SHORT_DESCRIPTION_BASE + base,
                SHORT_DESCRIPTION_LENGTH,
                &titles.short_descriptions[index],
            )?;
            write_field(
                &mut self.data,
                LONG_DESCRIPTION_BASE + base,
                LONG_DESCRIPTION_LENGTH,
                &titles.long_descriptions[index],
            )?;
            write_field(
                &mut self.data,
                PUBLISHER_BASE + base,
                PUBLISHER_LENGTH,
                &titles.publishers[index],
            )?;
        }
        Ok(())
    }
}

fn read_titles(data: &[u8]) -> Result<Titles> {
    let mut titles = Titles {
        short_descriptions: Vec::new(),
        long_descriptions: Vec::new(),
        publishers: Vec::new(),
    };
    for index in 0..TITLE_COUNT {
        let base = TITLE_LENGTH * index;
        titles.short_descriptions.push(read_field(
            data,
            SHORT_DESCRIPTION_BASE + base,
            SHORT_DESCRIPTION_LENGTH,
        )?);
        titles.long_descriptions.push(read_field(
            data,
            LONG_DESCRIPTION_BASE + base,
            LONG_DESCRIPTION_LENGTH,
        )?);
        titles
            .publishers
            .push(read_field(data, PUBLISHER_BASE + base, PUBLISHER_LENGTH)?);
    }
    Ok(titles)
}

fn read_field(data: &[u8], offset: usize, length: usize) -> Result<String> {
    let bytes = data
        .get(offset..offset + length)
        .ok_or(FormatError::out_of_bounds(CONTEXT))?;
    Ok(java_trim(&decode_utf16le(bytes)).to_owned())
}

fn write_field(data: &mut [u8], offset: usize, length: usize, text: &str) -> Result<()> {
    let field = data
        .get_mut(offset..offset + length)
        .ok_or(FormatError::out_of_bounds(CONTEXT))?;
    field.fill(0);
    let encoded = encode_utf16le(text);
    field
        .get_mut(..encoded.len())
        .ok_or(FormatError::out_of_bounds(CONTEXT))?
        .copy_from_slice(&encoded);
    Ok(())
}
