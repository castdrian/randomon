use randomon_core::Result;
use randomon_ctr::bytes::{find, find_after_prefix};

use crate::constants;
use crate::handler::Gen7RomHandler;

impl Gen7RomHandler {
    pub(crate) fn apply_fastest_text(&mut self) {
        let patches: [(&str, [u8; 4]); 2] = [
            (
                constants::FASTEST_TEXT_PREFIXES[0],
                [0x03, 0x40, 0xA0, 0xE3],
            ),
            (
                constants::FASTEST_TEXT_PREFIXES[1],
                [0x03, 0x50, 0xA0, 0xE3],
            ),
        ];
        for (prefix, instruction) in patches {
            if let Some(offset) = find_after_prefix(&self.code, prefix) {
                self.code[offset..offset + 4].copy_from_slice(&instruction);
            }
        }
    }

    pub(crate) fn positive_call_rates(&mut self) {
        for pokemon in self.pokes.clone().into_iter().flatten() {
            let pokemon = self.core.data.pk_mut(pokemon);
            if pokemon.call_rate <= 0 {
                pokemon.call_rate = 5;
            }
        }
    }

    pub(crate) fn patch_forme_reversion(&mut self) -> Result<()> {
        let prefix = constants::save_load_forme_reversion_prefix(self.rom_type);
        if let Some(offset) = find_after_prefix(&self.code, prefix) {
            let offset = offset + 8;
            self.code[offset..offset + 4].copy_from_slice(&[0, 0, 0, 0]);
            if self.is_usum() {
                self.code[offset + 8..offset + 12].copy_from_slice(&[0, 0, 0, 0]);
            }
        }
        let battle_path = self.file("Battle");
        let mut battle_cro = self.read_file(battle_path)?;
        if let Some(offset) =
            find_after_prefix(&battle_cro, constants::AFTER_BATTLE_FORME_REVERSION_PREFIX)
        {
            for relative in [0, 60, 92, 148] {
                battle_cro[offset + relative..offset + relative + 4].copy_from_slice(&[0, 0, 0, 0]);
            }
            self.write_file(battle_path, &battle_cro)?;
        }
        Ok(())
    }

    pub(crate) fn enable_guaranteed_catching(&self) -> Result<()> {
        let battle_path = self.file("Battle");
        let mut battle_cro = self.read_file(battle_path)?;
        let offset = find(&battle_cro, constants::PERFECT_ODDS_BRANCH_LOCATOR);
        if offset > 0 {
            let offset = offset as usize;
            battle_cro[offset..offset + 4].copy_from_slice(&[0, 0, 0, 0]);
            self.write_file(battle_path, &battle_cro)?;
        }
        Ok(())
    }
}
