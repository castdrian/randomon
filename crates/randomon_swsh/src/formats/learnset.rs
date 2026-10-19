use crate::bytes::bytes_at;
use crate::error::{Result, invalid};

pub const RECORD_SIZE: usize = 0x104;
pub const MAX_MOVES: usize = 64;
const PAIR_SIZE: usize = 4;
const CONTEXT: &str = "wazaoboe_total.bin";

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct LevelUpMove {
    pub move_id: u16,
    pub level: u16,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Learnset {
    pub moves: Vec<LevelUpMove>,
}

fn is_terminator(pair: &[u8; PAIR_SIZE]) -> bool {
    pair[3] == 0xFF
}

impl Learnset {
    pub fn parse(record: &[u8]) -> Result<Self> {
        if record.len() != RECORD_SIZE {
            return Err(invalid(
                CONTEXT,
                format!("learnset record of {} bytes", record.len()),
            ));
        }
        let pairs = bytes_at(record, 0, RECORD_SIZE, CONTEXT)?
            .as_chunks::<PAIR_SIZE>()
            .0
            .iter();
        let moves = pairs
            .take_while(|pair| !is_terminator(pair))
            .map(|pair| LevelUpMove {
                move_id: u16::from_le_bytes([pair[0], pair[1]]),
                level: u16::from_le_bytes([pair[2], pair[3]]),
            })
            .collect();
        Ok(Learnset { moves })
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        if self.moves.len() > MAX_MOVES {
            return Err(invalid(
                CONTEXT,
                format!(
                    "{} level-up moves exceed the limit of {MAX_MOVES}",
                    self.moves.len()
                ),
            ));
        }
        if let Some(entry) = self.moves.iter().find(|entry| entry.level >> 8 == 0xFF) {
            return Err(invalid(
                CONTEXT,
                format!("level {} would read as a terminator", entry.level),
            ));
        }
        let mut record = vec![0xFF; RECORD_SIZE];
        for (pair, entry) in record
            .as_chunks_mut::<PAIR_SIZE>()
            .0
            .iter_mut()
            .zip(&self.moves)
        {
            pair[..2].copy_from_slice(&entry.move_id.to_le_bytes());
            pair[2..].copy_from_slice(&entry.level.to_le_bytes());
        }
        Ok(record)
    }

    pub fn moves_at_or_below(&self, level: u16) -> impl Iterator<Item = u16> + '_ {
        self.moves
            .iter()
            .filter(move |entry| entry.level <= level)
            .map(|entry| entry.move_id)
    }
}

pub fn parse_all(data: &[u8]) -> Result<Vec<Learnset>> {
    if data.is_empty() || !data.len().is_multiple_of(RECORD_SIZE) {
        return Err(invalid(
            CONTEXT,
            format!("{} bytes is not a whole number of records", data.len()),
        ));
    }
    data.as_chunks::<RECORD_SIZE>()
        .0
        .iter()
        .map(|record| Learnset::parse(record))
        .collect()
}

pub fn write_all(learnsets: &[Learnset]) -> Result<Vec<u8>> {
    let mut output = Vec::with_capacity(learnsets.len() * RECORD_SIZE);
    for learnset in learnsets {
        output.extend_from_slice(&learnset.to_bytes()?);
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Learnset {
        Learnset {
            moves: vec![
                LevelUpMove {
                    move_id: 33,
                    level: 1,
                },
                LevelUpMove {
                    move_id: 45,
                    level: 1,
                },
                LevelUpMove {
                    move_id: 22,
                    level: 9,
                },
            ],
        }
    }

    #[test]
    fn learnset_round_trips_with_ff_padding() {
        let bytes = sample().to_bytes().expect("write");
        assert_eq!(bytes.len(), RECORD_SIZE);
        assert!(bytes[12..].iter().all(|&byte| byte == 0xFF));
        assert_eq!(Learnset::parse(&bytes), Ok(sample()));
        assert_eq!(
            sample().moves_at_or_below(5).collect::<Vec<_>>(),
            vec![33, 45]
        );
    }

    #[test]
    fn table_round_trips() {
        let table = vec![sample(), Learnset::default()];
        let bytes = write_all(&table).expect("write");
        assert_eq!(parse_all(&bytes), Ok(table));
        assert!(parse_all(&bytes[1..]).is_err());
        assert!(parse_all(&[]).is_err());
    }

    #[test]
    fn oversized_learnsets_are_rejected() {
        let learnset = Learnset {
            moves: vec![
                LevelUpMove {
                    move_id: 1,
                    level: 1
                };
                MAX_MOVES + 1
            ],
        };
        assert!(learnset.to_bytes().is_err());
        let full = Learnset {
            moves: vec![
                LevelUpMove {
                    move_id: 1,
                    level: 1
                };
                MAX_MOVES
            ],
        };
        assert_eq!(Learnset::parse(&full.to_bytes().expect("write")), Ok(full));
        let terminator_level = Learnset {
            moves: vec![LevelUpMove {
                move_id: 1,
                level: 0xFF00,
            }],
        };
        assert!(terminator_level.to_bytes().is_err());
    }

    #[test]
    fn records_of_the_wrong_size_are_rejected() {
        assert!(Learnset::parse(&[0xFF; 8]).is_err());
    }
}
