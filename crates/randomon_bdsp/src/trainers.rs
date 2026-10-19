use std::collections::BTreeMap;

use randomon_core::Movesets;
use randomon_data::{Trainer, TrainerPokemon, moves_at_level};
use randomon_unity::Value;

use crate::constants;
use crate::error::Result;
use crate::handler::BdspRomHandler;
use crate::tables::{Table, int, set_int};

const IV_FIELDS: [&str; 6] = [
    "TalentHp",
    "TalentAtk",
    "TalentDef",
    "TalentAgi",
    "TalentSpAtk",
    "TalentSpDef",
];
const SHINY_BIT: i32 = 1 << 30;
const DOUBLE_BATTLE: i32 = 1;
const HAS_ITEMS_AND_MOVES: i32 = 3;

fn slot_field(slot: usize, name: &str) -> String {
    format!("P{}{name}", slot + 1)
}

fn slot_int(row: &Value, slot: usize, name: &str) -> Result<i32> {
    int(row, &slot_field(slot, name))
}

fn set_slot_int(row: &mut Value, slot: usize, name: &str, value: i32) -> Result<()> {
    set_int(row, &slot_field(slot, name), value)
}

fn label(row: &Value, field: &str) -> String {
    row.field(field)
        .ok()
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned()
}

impl BdspRomHandler {
    fn class_labels(&self) -> Result<Vec<String>> {
        Ok(self
            .tables
            .rows(Table::Trainer, "TrainerType")?
            .iter()
            .map(|row| label(row, "LabelTrType"))
            .collect())
    }

    pub(crate) fn class_names(&self) -> Result<Vec<String>> {
        let texts = self.messages.trainer_classes.by_label()?;
        Ok(self
            .class_labels()?
            .iter()
            .map(|label| texts.get(label).cloned().unwrap_or_default())
            .collect())
    }

    pub(crate) fn set_class_names(&mut self, names: &[String]) -> Result<()> {
        for (label, name) in self.class_labels()?.iter().zip(names) {
            if !label.is_empty() && self.messages.trainer_classes.has_label(label) {
                self.messages.trainer_classes.set_label_text(label, name)?;
            }
        }
        Ok(())
    }

    fn name_labels(&self) -> Result<Vec<String>> {
        Ok(self
            .tables
            .rows(Table::Trainer, "TrainerData")?
            .iter()
            .skip(1)
            .map(|row| label(row, "NameLabel"))
            .collect())
    }

    pub(crate) fn trainer_name_list(&self) -> Result<Vec<String>> {
        let texts = self.messages.trainer_names.by_label()?;
        Ok(self
            .name_labels()?
            .iter()
            .map(|label| texts.get(label).cloned().unwrap_or_default())
            .collect())
    }

    pub(crate) fn set_trainer_name_list(&mut self, names: &[String]) -> Result<()> {
        let mut written: BTreeMap<String, ()> = BTreeMap::new();
        for (label, name) in self.name_labels()?.iter().zip(names) {
            if label.is_empty() || written.contains_key(label) {
                continue;
            }
            if self.messages.trainer_names.has_label(label) {
                self.messages.trainer_names.set_label_text(label, name)?;
                written.insert(label.clone(), ());
            }
        }
        Ok(())
    }

    fn ability_slot_of(&self, pokemon: &TrainerPokemon, ability: i32) -> i32 {
        let entry = self.core.data.pk(pokemon.pokemon);
        if ability != 0 && ability == entry.ability1 {
            1
        } else if ability != 0 && ability == entry.ability2 {
            2
        } else if ability != 0 && ability == entry.ability3 {
            3
        } else {
            1
        }
    }

    pub(crate) fn ability_for_trainer_pokemon(&self, pokemon: &TrainerPokemon) -> i32 {
        let entry = self.core.data.pk(pokemon.pokemon);
        match pokemon.ability_slot {
            2 if entry.ability2 != 0 => entry.ability2,
            3 if entry.ability3 != 0 => entry.ability3,
            _ => entry.ability1,
        }
    }

    fn read_trainer_pokemon(&self, row: &Value, slot: usize) -> Result<Option<TrainerPokemon>> {
        let species_number = slot_int(row, slot, "MonsNo")?;
        let level = slot_int(row, slot, "Level")?;
        if !(1..=constants::SPECIES_COUNT).contains(&species_number) || level < 1 {
            return Ok(None);
        }
        let forme = slot_int(row, slot, "FormNo")?;
        let mut pokemon = TrainerPokemon::new(self.pk(species_number), level);
        pokemon.forme = forme;
        pokemon.forme_suffix = constants::forme_suffix(species_number, forme).to_owned();
        pokemon.held_item = slot_int(row, slot, "Item")?;
        pokemon.nature = slot_int(row, slot, "Seikaku")? as i8;
        for (index, field) in IV_FIELDS.iter().enumerate() {
            pokemon.ivs |= (slot_int(row, slot, field)? & 0x1F) << (index * 5);
        }
        if slot_int(row, slot, "IsRare")? != 0 {
            pokemon.ivs |= SHINY_BIT;
        }
        pokemon.hp_evs = slot_int(row, slot, "EffortHp")? as i8;
        pokemon.atk_evs = slot_int(row, slot, "EffortAtk")? as i8;
        pokemon.def_evs = slot_int(row, slot, "EffortDef")? as i8;
        pokemon.spatk_evs = slot_int(row, slot, "EffortSpAtk")? as i8;
        pokemon.spdef_evs = slot_int(row, slot, "EffortSpDef")? as i8;
        pokemon.speed_evs = slot_int(row, slot, "EffortAgi")? as i8;
        for move_slot in 0..4 {
            pokemon.moves[move_slot] = slot_int(row, slot, &format!("Waza{}", move_slot + 1))?;
        }
        let shown = self.pokemon_with_forme(species_number, forme);
        let ability = slot_int(row, slot, "Tokusei")?;
        let probe = TrainerPokemon::new(shown, level);
        pokemon.ability_slot = self.ability_slot_of(&probe, ability);
        Ok(Some(pokemon))
    }

    pub(crate) fn read_trainers(&self) -> Result<Vec<Trainer>> {
        let data_rows = self.tables.rows(Table::Trainer, "TrainerData")?;
        let poke_rows = self.tables.rows(Table::Trainer, "TrainerPoke")?;
        let class_names = self.class_names()?;
        let names = self.trainer_name_list()?;
        let mut trainers = Vec::new();
        for (index, (data, party)) in data_rows.iter().zip(poke_rows).enumerate().skip(1) {
            let mut trainer = Trainer::new();
            trainer.index = index as i32;
            trainer.offset = index as i32;
            trainer.poketype = HAS_ITEMS_AND_MOVES;
            trainer.trainerclass = int(data, "TypeID")?.max(0);
            let class_name = class_names
                .get(trainer.trainerclass as usize)
                .cloned()
                .unwrap_or_default();
            let name = names.get(index - 1).cloned().unwrap_or_default();
            trainer.name = Some(name.clone());
            trainer.full_display_name = Some(format!("{class_name} {name}").trim().to_owned());
            for slot in 0..constants::PARTY_SIZE {
                if let Some(pokemon) = self.read_trainer_pokemon(party, slot)? {
                    trainer.pokemon.push(pokemon);
                }
            }
            trainers.push(trainer);
        }
        constants::tag_trainers(&mut trainers);
        constants::set_forced_rival_starter_positions(&mut trainers);
        Ok(trainers)
    }

    fn write_trainer_pokemon(
        &self,
        row: &mut Value,
        slot: usize,
        pokemon: &TrainerPokemon,
        movesets: &Movesets,
    ) -> Result<()> {
        let species_number = self.core.data.base_number(pokemon.pokemon);
        let shown = self.alt_forme_of(pokemon.pokemon, pokemon.forme);
        set_slot_int(row, slot, "MonsNo", species_number)?;
        set_slot_int(row, slot, "FormNo", pokemon.forme.max(0))?;
        set_slot_int(row, slot, "Level", pokemon.level.clamp(1, 100))?;
        set_slot_int(row, slot, "Item", pokemon.held_item)?;
        set_slot_int(row, slot, "Seikaku", i32::from(pokemon.nature as u8))?;
        set_slot_int(row, slot, "IsRare", i32::from(pokemon.ivs & SHINY_BIT != 0))?;
        for (index, field) in IV_FIELDS.iter().enumerate() {
            set_slot_int(row, slot, field, (pokemon.ivs >> (index * 5)) & 0x1F)?;
        }
        for (field, value) in [
            ("EffortHp", pokemon.hp_evs),
            ("EffortAtk", pokemon.atk_evs),
            ("EffortDef", pokemon.def_evs),
            ("EffortSpAtk", pokemon.spatk_evs),
            ("EffortSpDef", pokemon.spdef_evs),
            ("EffortAgi", pokemon.speed_evs),
        ] {
            set_slot_int(row, slot, field, i32::from(value as u8))?;
        }
        let mut probe = pokemon.clone();
        probe.pokemon = shown;
        set_slot_int(
            row,
            slot,
            "Tokusei",
            self.ability_for_trainer_pokemon(&probe),
        )?;
        let moves = if pokemon.reset_moves || pokemon.moves.iter().all(|&mv| mv == 0) {
            movesets.get(&shown.number()).map_or([0; 4], |movepool| {
                moves_at_level(movepool, pokemon.level, 0)
            })
        } else {
            pokemon.moves
        };
        for (move_slot, &move_number) in moves.iter().enumerate() {
            set_slot_int(row, slot, &format!("Waza{}", move_slot + 1), move_number)?;
        }
        Ok(())
    }

    fn clear_trainer_slot(row: &mut Value, slot: usize) -> Result<()> {
        for field in ["MonsNo", "FormNo", "Level", "Item", "Tokusei", "IsRare"] {
            set_slot_int(row, slot, field, 0)?;
        }
        for move_slot in 1..=4 {
            set_slot_int(row, slot, &format!("Waza{move_slot}"), 0)?;
        }
        Ok(())
    }

    pub(crate) fn write_trainers(
        &mut self,
        trainers: &[Trainer],
        double_battle_mode: bool,
    ) -> Result<()> {
        let movesets = self.read_movesets()?;
        let mut data_rows = self.tables.rows(Table::Trainer, "TrainerData")?.to_vec();
        let mut poke_rows = self.tables.rows(Table::Trainer, "TrainerPoke")?.to_vec();
        for trainer in trainers {
            let index = trainer.index as usize;
            let (Some(data), Some(party)) = (data_rows.get_mut(index), poke_rows.get_mut(index))
            else {
                continue;
            };
            for slot in 0..constants::PARTY_SIZE {
                match trainer.pokemon.get(slot) {
                    Some(pokemon) => self.write_trainer_pokemon(party, slot, pokemon, &movesets)?,
                    None => Self::clear_trainer_slot(party, slot)?,
                }
            }
            if double_battle_mode && !trainer.skip_important() && trainer.pokemon.len() >= 2 {
                set_int(data, "FightType", DOUBLE_BATTLE)?;
            }
        }
        *self.tables.rows_mut(Table::Trainer, "TrainerData")? = data_rows;
        *self.tables.rows_mut(Table::Trainer, "TrainerPoke")? = poke_rows;
        Ok(())
    }
}
