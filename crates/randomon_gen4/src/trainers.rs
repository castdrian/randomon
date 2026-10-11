use randomon_core::{Movesets, RandomizationError, Result};
use randomon_data::constants::{abilities, gen4, items, species};
use randomon_data::{Effectiveness, MoveCategory, Trainer, TrainerPokemon, Type, moves_at_level};
use randomon_formats::nds::NarcArchive;

use crate::bytes::{after_prefix, file, file_mut, i8_at, set_u8, set_u16, u8_at, u16_at};
use crate::handler::Gen4RomHandler;

const DOUBLE_BATTLE_FLAG_OFFSET: i32 = 16;
const SINGLE_TRAINER_DOUBLE_FLAG: i32 = 3;
const TRAINER_CLASS_OFFSET: i32 = 1;
const TRAINER_COUNT_OFFSET: i32 = 3;

fn not_supported() -> RandomizationError {
    RandomizationError::randomization("Double Battle Mode not supported for this game")
}

fn string_at(strings: &[String], index: i32) -> &str {
    usize::try_from(index)
        .ok()
        .and_then(|index| strings.get(index))
        .map_or("null", String::as_str)
}

impl Gen4RomHandler {
    fn has_extra_trainer_padding(&self) -> bool {
        !self.entry.is_dp()
    }

    fn read_trainer_pokemon(&self, trainer: &mut Trainer, data: &[u8], count: i32) -> Result<()> {
        let mut offset = 0;
        for _ in 0..count {
            let difficulty = u8_at(data, offset)?;
            let level = u8_at(data, offset + 2)?;
            let species = u8_at(data, offset + 4)? + ((u8_at(data, offset + 5)? & 0x01) << 8);
            let forme = i8_at(data, offset + 5)? >> 2;
            let mut ability_slot = (u8_at(data, offset + 1)? >> 4) & 0xF;
            if ability_slot == 0 {
                ability_slot = 1;
            }
            let mut pokemon = TrainerPokemon::new(self.pokemon_by_number(species)?, level);
            pokemon.ivs = difficulty * 31 / 255;
            pokemon.ability_slot = ability_slot;
            pokemon.forme = forme;
            pokemon.forme_suffix = gen4::forme_suffix_by_base_forme(species, forme).to_owned();
            offset += 6;
            if trainer.pokemon_have_items() {
                pokemon.held_item = u16_at(data, offset)?;
                offset += 2;
            }
            if trainer.pokemon_have_custom_moves() {
                for (slot, mv) in pokemon.moves.iter_mut().enumerate() {
                    *mv = u16_at(data, offset + slot as i32 * 2)?;
                }
                offset += 8;
            }
            if self.has_extra_trainer_padding() {
                offset += 2;
            }
            trainer.pokemon.push(pokemon);
        }
        Ok(())
    }

    pub(crate) fn trainers(&self) -> Result<Vec<Trainer>> {
        let trainer_data = self.read_narc("TrainerData")?;
        let trainer_pokemon = self.read_narc("TrainerPokemon")?;
        let classes = self.trainer_class_names()?;
        let names = self.trainer_names()?;
        let mut trainers = Vec::new();
        for index in 1..trainer_data.files.len() as i32 {
            let data = file(&trainer_data, index)?;
            let pokes = file(&trainer_pokemon, index)?;
            let mut trainer = Trainer::new();
            trainer.poketype = u8_at(data, 0)?;
            trainer.trainerclass = u8_at(data, TRAINER_CLASS_OFFSET)?;
            trainer.index = index;
            let count = u8_at(data, TRAINER_COUNT_OFFSET)?;
            trainer.full_display_name = Some(format!(
                "{} {}",
                string_at(&classes, trainer.trainerclass),
                string_at(&names, index - 1)
            ));
            self.read_trainer_pokemon(&mut trainer, pokes, count)?;
            trainers.push(trainer);
        }
        match self.rom_type() {
            gen4::TYPE_DP => {
                gen4::tag_trainers_dp(&mut trainers);
                gen4::set_multi_battle_status_dp(&mut trainers);
            }
            gen4::TYPE_PLAT => {
                gen4::tag_trainers_pt(&mut trainers);
                gen4::set_multi_battle_status_pt(&mut trainers);
            }
            _ => {
                gen4::tag_trainers_hgss(&mut trainers);
                gen4::set_multi_battle_status_hgss(&mut trainers);
            }
        }
        Ok(trainers)
    }

    fn trainer_moves(&self, pokemon: &TrainerPokemon, movesets: &Movesets) -> [i32; 4] {
        if !pokemon.reset_moves {
            return pokemon.moves;
        }
        let forme = self.alt_forme_of(pokemon.pokemon, pokemon.forme);
        let movepool = movesets
            .get(&self.number_of(forme))
            .map_or(&[][..], Vec::as_slice);
        moves_at_level(movepool, pokemon.level, 0)
    }

    fn encode_trainer_pokemon(&self, trainer: &Trainer, movesets: &Movesets) -> Result<Vec<u8>> {
        let count = trainer.pokemon.len();
        let mut bytes_needed = 6 * count;
        if self.has_extra_trainer_padding() {
            bytes_needed += 2 * count;
        }
        if trainer.pokemon_have_custom_moves() {
            bytes_needed += 8 * count;
        }
        if trainer.pokemon_have_items() {
            bytes_needed += 2 * count;
        }
        let mut data = vec![0u8; bytes_needed];
        let mut offset = 0;
        for pokemon in &trainer.pokemon {
            let ability = if pokemon.ability_slot == 1 {
                0
            } else {
                pokemon.ability_slot << 4
            };
            let difficulty = 255.min(1 + (pokemon.ivs * 255) / 31);
            set_u16(&mut data, offset, difficulty | ability << 8)?;
            set_u16(&mut data, offset + 2, pokemon.level)?;
            set_u16(&mut data, offset + 4, self.number_of(pokemon.pokemon))?;
            let species_high = u8_at(&data, offset + 5)?;
            set_u8(&mut data, offset + 5, species_high | (pokemon.forme << 2))?;
            offset += 6;
            if trainer.pokemon_have_items() {
                set_u16(&mut data, offset, pokemon.held_item)?;
                offset += 2;
            }
            if trainer.pokemon_have_custom_moves() {
                for (slot, &mv) in self.trainer_moves(pokemon, movesets).iter().enumerate() {
                    set_u16(&mut data, offset + slot as i32 * 2, mv)?;
                }
                offset += 8;
            }
            if self.has_extra_trainer_padding() {
                offset += 2;
            }
        }
        Ok(data)
    }

    fn fix_ability_slot_values_for_hgss(trainers: &mut [Trainer]) {
        for trainer in trainers {
            if let Some(last) = trainer.pokemon.last().map(|pokemon| pokemon.ability_slot) {
                for pokemon in &mut trainer.pokemon {
                    pokemon.ability_slot = last;
                }
            }
        }
    }

    pub(crate) fn set_trainers_impl(
        &mut self,
        trainers: &[Trainer],
        double_battle_mode: bool,
    ) -> Result<()> {
        let mut trainers = trainers.to_vec();
        if self.entry.is_hgss() {
            Self::fix_ability_slot_values_for_hgss(&mut trainers);
        }
        let mut iterator = trainers.iter();
        let mut trainer_data = self.read_narc("TrainerData")?;
        let mut trainer_pokemon = NarcArchive::new();
        let movesets = self.moves_learnt()?;
        trainer_pokemon.files.push(vec![0; 8]);
        for index in 1..trainer_data.files.len() as i32 {
            let Some(trainer) = iterator.next() else {
                let original = self.read_narc("TrainerPokemon")?;
                trainer_pokemon
                    .files
                    .push(file(&original, index).cloned().unwrap_or_default());
                continue;
            };
            let data = file_mut(&mut trainer_data, index)?;
            set_u8(data, 0, trainer.poketype)?;
            set_u8(data, TRAINER_COUNT_OFFSET, trainer.pokemon.len() as i32)?;
            if double_battle_mode && !trainer.skip_important() {
                let excluded_partner =
                    !self.entry.is_hgss() && gen4::PARTNER_TRAINER_INDICES.contains(&trainer.index);
                if u8_at(data, DOUBLE_BATTLE_FLAG_OFFSET)? == 0 && !excluded_partner {
                    set_u8(data, DOUBLE_BATTLE_FLAG_OFFSET, SINGLE_TRAINER_DOUBLE_FLAG)?;
                }
            }
            trainer_pokemon
                .files
                .push(self.encode_trainer_pokemon(trainer, &movesets)?);
        }
        self.write_narc("TrainerData", &trainer_data)?;
        self.write_narc("TrainerPokemon", &trainer_pokemon)?;
        if double_battle_mode {
            self.apply_double_battle_patches()?;
        }
        Ok(())
    }

    fn apply_double_battle_patches(&mut self) -> Result<()> {
        let mut arm9 = self.arm9.clone();
        let fix_prefix = gen4::double_battle_fix_prefix(self.rom_type());
        let offset = after_prefix(&arm9, fix_prefix).ok_or_else(not_supported)?;
        set_u8(&mut arm9, offset, 0xE0)?;
        let prefix = |key: &str| self.entry.string(key);
        let offset = after_prefix(&arm9, prefix("DoubleBattleFlagReturnPrefix"))
            .ok_or_else(not_supported)?;
        set_u16(&mut arm9, offset, 0xBD08)?;
        let offset =
            after_prefix(&arm9, prefix("DoubleBattleWalkingPrefix1")).ok_or_else(not_supported)?;
        set_u8(&mut arm9, offset, 0x2)?;
        set_u8(&mut arm9, offset + 3, 0xD0)?;
        let offset =
            after_prefix(&arm9, prefix("DoubleBattleWalkingPrefix2")).ok_or_else(not_supported)?;
        set_u8(&mut arm9, offset, 0x2)?;
        let offset =
            after_prefix(&arm9, prefix("DoubleBattleTextBoxPrefix")).ok_or_else(not_supported)?;
        set_u16(&mut arm9, offset, 0x46C0)?;
        set_u16(&mut arm9, offset + 2, 0x2802)?;
        set_u8(&mut arm9, offset + 5, 0xD0)?;
        let mut sub_seq = self.read_narc("BattleSkillSubSeq")?;
        let end_file = file_mut(&mut sub_seq, self.entry.int("TrainerEndFileNumber"))?;
        set_u8(end_file, self.entry.int("TrainerEndTextBoxOffset"), 0)?;
        self.write_narc("BattleSkillSubSeq", &sub_seq)?;
        self.arm9 = arm9;
        Ok(())
    }

    pub(crate) fn trainer_names(&self) -> Result<Vec<String>> {
        let mut names = self.get_strings(self.entry.int("TrainerNamesTextOffset"))?;
        if !names.is_empty() {
            names.remove(0);
        }
        Ok(names
            .into_iter()
            .map(|name| name.replace("\\and", "&"))
            .collect())
    }

    pub(crate) fn set_trainer_names_impl(&mut self, trainer_names: &[String]) -> Result<()> {
        let index = self.entry.int("TrainerNamesTextOffset");
        let (old_names, compressed) = self.get_strings_and_flag(index)?;
        let mut new_names: Vec<String> = trainer_names
            .iter()
            .map(|name| name.replace('&', "\\and"))
            .collect();
        new_names.insert(0, old_names.first().cloned().unwrap_or_default());
        self.set_strings_compressed(index, &new_names, compressed)
    }

    pub(crate) fn trainer_class_names(&self) -> Result<Vec<String>> {
        self.get_strings(self.entry.int("TrainerClassesTextOffset"))
    }

    pub(crate) fn set_trainer_class_names_impl(&mut self, names: &[String]) -> Result<()> {
        self.set_strings(self.entry.int("TrainerClassesTextOffset"), names)
    }

    pub(crate) fn ability_for_trainer_pokemon(&self, pokemon: &TrainerPokemon) -> i32 {
        let data = &self.core.data;
        let mut current = pokemon.pokemon;
        while let Some(base) = data.pk(current).base_forme {
            current = base;
        }
        let pk = data.pk(current);
        if self.entry.is_hgss() && pokemon.ability_slot == 2 {
            pk.ability2
        } else {
            pk.ability1
        }
    }

    pub(crate) fn banned_formes_for_trainers(&self) -> Vec<randomon_data::PokemonId> {
        if self.entry.is_dp() {
            return Vec::new();
        }
        let giratina = self.pk(species::GIRATINA);
        let origin = self.alt_forme_of(giratina, 1);
        if origin == giratina {
            Vec::new()
        } else {
            vec![origin]
        }
    }

    fn add_move_items(&self, items_list: &mut Vec<i32>, move_number: i32, consumable_only: bool) {
        let Some(mv) = self.core.data.move_by_number(move_number) else {
            return;
        };
        let type_items = || {
            mv.kind
                .and_then(gen4::type_boosting_items)
                .unwrap_or_default()
        };
        match mv.category {
            MoveCategory::Physical => {
                items_list.push(items::LIECHI_BERRY);
                if !consumable_only {
                    items_list.extend(type_items());
                    items_list.push(items::CHOICE_BAND);
                    items_list.push(items::MUSCLE_BAND);
                }
            }
            MoveCategory::Special => {
                items_list.push(items::PETAYA_BERRY);
                if !consumable_only {
                    items_list.extend(type_items());
                    items_list.push(items::WISE_GLASSES);
                    items_list.push(items::CHOICE_SPECS);
                }
            }
            MoveCategory::Status => {}
        }
        if !consumable_only && let Some(boosting) = gen4::move_boosting_items(move_number) {
            items_list.extend(boosting);
        }
    }

    pub(crate) fn sensible_held_items_for(
        &self,
        pokemon: &TrainerPokemon,
        consumable_only: bool,
        moveset: &[i32; 4],
    ) -> Vec<i32> {
        let mut items_list = gen4::general_purpose_consumable_items();
        let mut frequency_boost_count = 6;
        if !consumable_only {
            frequency_boost_count = 8;
            items_list.extend(gen4::general_purpose_items());
        }
        for &move_number in moveset {
            self.add_move_items(&mut items_list, move_number, consumable_only);
        }
        let pk = self.core.data.pk(pokemon.pokemon);
        let by_type = pk
            .primary_type
            .and_then(|primary| {
                Effectiveness::against(primary, pk.secondary_type, 4, self.effectiveness_updated)
            })
            .unwrap_or_default();
        for &(kind, effect) in &by_type {
            let berry = gen4::weakness_reducing_berry(kind);
            match effect {
                Effectiveness::Double => items_list.extend(berry),
                Effectiveness::Quadruple => {
                    for _ in 0..frequency_boost_count {
                        items_list.extend(berry);
                    }
                }
                _ => {}
            }
        }
        let normal = by_type
            .iter()
            .find(|(attacker, _)| *attacker == Type::Normal)
            .map(|(_, effect)| *effect);
        if normal == Some(Effectiveness::Neutral) {
            items_list.push(items::CHILAN_BERRY);
        }
        let ability = self.ability_for_trainer_pokemon(pokemon);
        if ability == abilities::LEVITATE {
            items_list.retain(|&item| item != items::SHUCA_BERRY);
        }
        if !consumable_only {
            if let Some(boosting) = gen4::ability_boosting_items(ability) {
                items_list.extend(boosting);
            }
            if pk.primary_type == Some(Type::Poison) || pk.secondary_type == Some(Type::Poison) {
                items_list.push(items::BLACK_SLUDGE);
            }
            if let Some(species_items) = gen4::species_boosting_items(pk.number) {
                for _ in 0..frequency_boost_count {
                    items_list.extend(species_items.iter().copied());
                }
            }
        }
        items_list
    }
}
