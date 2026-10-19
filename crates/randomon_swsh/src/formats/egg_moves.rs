use crate::bytes::u16_at;
use crate::error::{Result, invalid};

const CONTEXT: &str = "tamagowaza file";

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EggMoves {
    pub form_table_index: u16,
    pub moves: Vec<u16>,
}

impl EggMoves {
    pub fn parse(data: &[u8]) -> Result<Self> {
        let form_table_index = u16_at(data, 0, CONTEXT)?;
        let count = usize::from(u16_at(data, 2, CONTEXT)?);
        if data.len() != 4 + count * 2 {
            return Err(invalid(
                CONTEXT,
                format!(
                    "{count} moves need {} bytes, file has {}",
                    4 + count * 2,
                    data.len()
                ),
            ));
        }
        let moves = (0..count)
            .map(|index| u16_at(data, 4 + index * 2, CONTEXT))
            .collect::<Result<Vec<_>>>()?;
        Ok(EggMoves {
            form_table_index,
            moves,
        })
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        let count =
            u16::try_from(self.moves.len()).map_err(|_| invalid(CONTEXT, "too many egg moves"))?;
        let mut data = Vec::with_capacity(4 + self.moves.len() * 2);
        data.extend_from_slice(&self.form_table_index.to_le_bytes());
        data.extend_from_slice(&count.to_le_bytes());
        for move_id in &self.moves {
            data.extend_from_slice(&move_id.to_le_bytes());
        }
        Ok(data)
    }

    pub fn alternate_form_entry(&self, form: u16) -> Option<usize> {
        (form != 0 && self.form_table_index != 0)
            .then(|| usize::from(self.form_table_index) + usize::from(form) - 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn egg_moves_round_trip() {
        let bytes = [0x01, 0x00, 0x02, 0x00, 0x82, 0x00, 0x50, 0x00];
        let parsed = EggMoves::parse(&bytes).expect("parse");
        assert_eq!(
            parsed,
            EggMoves {
                form_table_index: 1,
                moves: vec![130, 80]
            }
        );
        assert_eq!(parsed.to_bytes().expect("write"), bytes);
        assert_eq!(
            EggMoves::parse(&[0, 0, 0, 0]).expect("parse").moves.len(),
            0
        );
    }

    #[test]
    fn mismatched_counts_are_rejected() {
        assert!(EggMoves::parse(&[0, 0, 2, 0, 1, 0]).is_err());
        assert!(EggMoves::parse(&[0, 0]).is_err());
    }

    #[test]
    fn alternate_forms_index_past_the_form_table_start() {
        let moves = EggMoves {
            form_table_index: 914,
            moves: Vec::new(),
        };
        assert_eq!(moves.alternate_form_entry(0), None);
        assert_eq!(moves.alternate_form_entry(2), Some(915));
    }
}
