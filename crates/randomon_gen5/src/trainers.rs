use randomon_core::{RandomizationError, Result};
use randomon_data::constants::{abilities, gen5, items};
use randomon_data::{Effectiveness, MoveCategory, Trainer, TrainerPokemon, Type, moves_at_level};
use randomon_formats::nds::NarcArchive;
use randomon_nds::{find, read_byte, read_word, write_word};

use crate::handler::Gen5RomHandler;

const DRIFTVEIL_TRAINER_COUNT: usize = 17;
const DRIFTVEIL_LEVEL: i32 = 25;
const NOP_HALFWORD: i32 = 0x46C0;
const CMP_ZERO_HALFWORD: i32 = 0x2800;

fn starts_with_capital(name: &str) -> bool {
    name.encode_utf16()
        .next()
        .is_some_and(|unit| (u16::from(b'A')..=u16::from(b'Z')).contains(&unit))
}

fn string_at(strings: &[String], index: usize) -> Result<&str> {
    strings
        .get(index)
        .map(String::as_str)
        .ok_or(RandomizationError::IndexOutOfBounds("strings"))
}

fn file_at(narc: &NarcArchive, index: usize) -> Result<&Vec<u8>> {
    narc.files
        .get(index)
        .ok_or(RandomizationError::IndexOutOfBounds("NARC files"))
}

fn driftveil_team_size(trainer: usize) -> usize {
    if trainer < 2 { 3 } else { 6 }
}

fn not_supported() -> RandomizationError {
    RandomizationError::randomization("Double Battle Mode not supported for this game")
}

impl Gen5RomHandler {
    fn read_trainer_pokemon(&self, trainer: &mut Trainer, data: &[u8], count: i32) {
        let mut offset = 0;
        for _ in 0..count {
            let difficulty = read_byte(data, offset);
            let species = read_word(data, offset + 4);
            let forme = read_word(data, offset + 6);
            let ability_and_flag = read_byte(data, offset + 1);
            let mut pokemon = TrainerPokemon::new(self.pk(species), read_word(data, offset + 2));
            pokemon.ivs = difficulty * 31 / 255;
            pokemon.ability_slot = (ability_and_flag >> 4) & 0xF;
            pokemon.forced_gender_flag = ability_and_flag & 0xF;
            pokemon.forme = forme;
            pokemon.forme_suffix = gen5::forme_suffix_by_base_forme(species, forme).to_owned();
            offset += 8;
            if trainer.pokemon_have_items() {
                pokemon.held_item = read_word(data, offset);
                offset += 2;
            }
            if trainer.pokemon_have_custom_moves() {
                for (slot, mv) in pokemon.moves.iter_mut().enumerate() {
                    *mv = read_word(data, offset + slot as i32 * 2);
                }
                offset += 8;
            }
            trainer.pokemon.push(pokemon);
        }
    }

    fn driftveil_trainers(&self, classes: &[String], names: &[String]) -> Result<Vec<Trainer>> {
        let driftveil = self.read_narc("DriftveilPokemon")?;
        let mut current_file = 1;
        let mut trainers = Vec::new();
        for trainer_number in 0..DRIFTVEIL_TRAINER_COUNT {
            let mut trainer = Trainer::new();
            trainer.poketype = 3;
            let name_and_class = gen5::BW2_DRIFTVEIL_TRAINER_OFFSETS[trainer_number];
            trainer.full_display_name = Some(format!(
                "{} {}",
                string_at(
                    classes,
                    (gen5::NORMAL_TRAINER_CLASS_LENGTH + name_and_class) as usize
                )?,
                string_at(
                    names,
                    (gen5::NORMAL_TRAINER_NAME_LENGTH + name_and_class) as usize
                )?
            ));
            trainer.requires_unique_held_items = true;
            for _ in 0..driftveil_team_size(trainer_number) {
                let data = file_at(&driftveil, current_file)?;
                let mut pokemon = TrainerPokemon::new(self.pk(read_word(data, 0)), DRIFTVEIL_LEVEL);
                pokemon.ivs = 31;
                pokemon.held_item = read_word(data, 12);
                for (slot, mv) in pokemon.moves.iter_mut().enumerate() {
                    *mv = read_word(data, 2 + slot as i32 * 2);
                }
                trainer.pokemon.push(pokemon);
                current_file += 1;
            }
            trainers.push(trainer);
        }
        Ok(trainers)
    }

    pub(crate) fn trainers(&self) -> Result<Vec<Trainer>> {
        let trainer_data = self.read_narc("TrainerData")?;
        let trainer_pokemon = self.read_narc("TrainerPokemon")?;
        let classes = self.trainer_class_names()?;
        let names = self.trainer_names()?;
        let mut trainers = Vec::new();
        for index in 1..trainer_data.files.len() {
            let data = &trainer_data.files[index];
            let pokes = file_at(&trainer_pokemon, index)?;
            let mut trainer = Trainer::new();
            trainer.poketype = read_byte(data, 0);
            trainer.index = index as i32;
            trainer.trainerclass = read_byte(data, 1);
            let count = read_byte(data, 3);
            trainer.full_display_name = Some(format!(
                "{} {}",
                string_at(&classes, trainer.trainerclass as usize)?,
                string_at(&names, index - 1)?
            ));
            if data[2] == 1 {
                self.original_double_trainers
                    .borrow_mut()
                    .push(index as i32);
            }
            self.read_trainer_pokemon(&mut trainer, pokes, count);
            trainers.push(trainer);
        }
        if self.is_bw2() {
            if self
                .entry
                .file("DriftveilPokemon")
                .is_some_and(|path| !path.is_empty())
            {
                trainers.extend(self.driftveil_trainers(&classes, &names)?);
            }
            let is_black2 = self.entry.rom_code.starts_with("IRE");
            gen5::tag_trainers_bw2(&mut trainers);
            gen5::set_multi_battle_status_bw2(&mut trainers, is_black2);
        } else {
            gen5::tag_trainers_bw(&mut trainers);
            gen5::set_multi_battle_status_bw(&mut trainers);
        }
        Ok(trainers)
    }

    fn reset_or_custom_moves(
        &self,
        pokemon: &TrainerPokemon,
        species: i32,
        movesets: &randomon_core::Movesets,
    ) -> Result<[i32; 4]> {
        if !pokemon.reset_moves {
            return Ok(pokemon.moves);
        }
        let movepool = movesets
            .get(&species)
            .ok_or(RandomizationError::NullPointer("movesets.get"))?;
        Ok(moves_at_level(movepool, pokemon.level, 0))
    }

    fn encode_trainer_pokemon(
        &self,
        trainer: &Trainer,
        movesets: &randomon_core::Movesets,
    ) -> Result<Vec<u8>> {
        let count = trainer.pokemon.len();
        let mut bytes_needed = 8 * count;
        if trainer.pokemon_have_custom_moves() {
            bytes_needed += 8 * count;
        }
        if trainer.pokemon_have_items() {
            bytes_needed += 2 * count;
        }
        let mut data = vec![0u8; bytes_needed];
        let mut offset = 0;
        for pokemon in &trainer.pokemon {
            let difficulty = 255.min(1 + (pokemon.ivs * 255) / 31);
            let ability_and_flag = ((pokemon.ability_slot << 4) | pokemon.forced_gender_flag) as i8;
            write_word(
                &mut data,
                offset,
                difficulty | (i32::from(ability_and_flag) << 8),
            );
            write_word(&mut data, offset + 2, pokemon.level);
            write_word(
                &mut data,
                offset + 4,
                self.core.data.pk(pokemon.pokemon).number,
            );
            write_word(&mut data, offset + 6, pokemon.forme);
            offset += 8;
            if trainer.pokemon_have_items() {
                write_word(&mut data, offset, pokemon.held_item);
                offset += 2;
            }
            if trainer.pokemon_have_custom_moves() {
                let forme = self.alt_forme_of(pokemon.pokemon, pokemon.forme);
                let species = self.core.data.pk(forme).number;
                let moves = self.reset_or_custom_moves(pokemon, species, movesets)?;
                for (slot, &mv) in moves.iter().enumerate() {
                    write_word(&mut data, offset + slot as i32 * 2, mv);
                }
                offset += 8;
            }
        }
        Ok(data)
    }

    pub(crate) fn set_trainers_impl(
        &mut self,
        trainers: &[Trainer],
        double_battle_mode: bool,
    ) -> Result<()> {
        let mut iterator = trainers.iter();
        let mut trainer_data = self.read_narc("TrainerData")?;
        let mut trainer_pokemon = NarcArchive::new();
        let movesets = self.moves_learnt()?;
        trainer_pokemon.files.push(vec![0; 8]);
        for index in 1..trainer_data.files.len() {
            let trainer = iterator
                .next()
                .ok_or(RandomizationError::IndexOutOfBounds("trainer iterator"))?;
            let data = &mut trainer_data.files[index];
            data[0] = trainer.poketype as u8;
            data[3] = trainer.pokemon.len() as u8;
            if double_battle_mode && !trainer.skip_important() && data[2] == 0 {
                data[2] = 1;
                data[12] |= 0x80;
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
        if self.is_bw2()
            && self
                .entry
                .file("DriftveilPokemon")
                .is_some_and(|path| !path.is_empty())
        {
            self.write_driftveil_trainers(&mut iterator, &movesets)?;
        }
        Ok(())
    }

    fn write_driftveil_trainers<'a>(
        &self,
        iterator: &mut impl Iterator<Item = &'a Trainer>,
        movesets: &randomon_core::Movesets,
    ) -> Result<()> {
        let mut driftveil = self.read_narc("DriftveilPokemon")?;
        let mut current_file = 1;
        for trainer_number in 0..DRIFTVEIL_TRAINER_COUNT {
            let trainer = iterator
                .next()
                .ok_or(RandomizationError::IndexOutOfBounds("trainer iterator"))?;
            let mut pokes = trainer.pokemon.iter();
            for _ in 0..driftveil_team_size(trainer_number) {
                let pokemon = pokes.next().ok_or(RandomizationError::IndexOutOfBounds(
                    "trainer pokemon iterator",
                ))?;
                let species = self.core.data.pk(pokemon.pokemon).number;
                let moves = self.reset_or_custom_moves(pokemon, species, movesets)?;
                let data = driftveil
                    .files
                    .get_mut(current_file)
                    .ok_or(RandomizationError::IndexOutOfBounds("driftveil.files"))?;
                write_word(data, 0, species);
                write_word(data, 12, pokemon.held_item);
                for (slot, &mv) in moves.iter().enumerate() {
                    write_word(data, 2 + slot as i32 * 2, mv);
                }
                current_file += 1;
            }
        }
        self.write_narc("DriftveilPokemon", &driftveil)
    }

    fn update_trainer_text_boxes(&self) -> Result<()> {
        let mut text_boxes = self.read_narc("TrainerTextBoxes")?;
        let originals = self.original_double_trainers.borrow().clone();
        let data = text_boxes
            .files
            .get_mut(0)
            .ok_or(RandomizationError::IndexOutOfBounds(
                "trainerTextBoxes.files",
            ))?;
        let mut position = 0;
        while position < data.len() as i32 {
            let trainer_index = read_word(data, position);
            if originals.contains(&trainer_index) {
                match read_word(data, position + 2) {
                    3 => write_word(data, position + 2, 0),
                    5 => write_word(data, position + 2, 2),
                    6 => write_word(data, position + 2, 0x18),
                    _ => {}
                }
            }
            position += 4;
        }
        self.write_narc("TrainerTextBoxes", &text_boxes)
    }

    fn prefixed_offset(data: &[u8], prefix: &str, length_prefix: &str) -> Result<i32> {
        let offset = find(data, prefix);
        if offset > 0 {
            Ok(offset + length_prefix.len() as i32 / 2)
        } else {
            Err(not_supported())
        }
    }

    fn patch_field_overlay_for_doubles(&self) -> Result<()> {
        let number = self.entry.int("FieldOvlNumber");
        let mut overlay = self.read_overlay(number)?;
        let text_box_prefix = self.entry.string("TrainerOverworldTextBoxPrefix");
        let offset = Self::prefixed_offset(&overlay, text_box_prefix, text_box_prefix)?;
        overlay[(offset - 2) as usize] = 0;
        overlay[offset as usize] = 2;
        overlay[(offset + 2) as usize] = 0x18;
        let limit_prefix = self.entry.string("DoubleBattleLimitPrefix");
        let offset = Self::prefixed_offset(&overlay, limit_prefix, text_box_prefix)?;
        write_word(&mut overlay, offset, NOP_HALFWORD);
        write_word(&mut overlay, offset + 2, NOP_HALFWORD);
        let pointer_prefix = self.entry.string("DoubleBattleGetPointerPrefix");
        let beq_to_single_trainer = self.entry.int("BeqToSingleTrainerNumber");
        let offset = Self::prefixed_offset(&overlay, pointer_prefix, text_box_prefix)?;
        let moved_e = read_word(&overlay, offset + 0xE);
        write_word(&mut overlay, offset + 0x10, moved_e);
        let moved_c = read_word(&overlay, offset + 0xC);
        write_word(&mut overlay, offset + 0xE, moved_c);
        let moved_a = read_word(&overlay, offset + 0xA);
        write_word(&mut overlay, offset + 0xC, moved_a);
        write_word(&mut overlay, offset + 0xA, beq_to_single_trainer);
        write_word(&mut overlay, offset + 8, CMP_ZERO_HALFWORD);
        self.write_overlay(number, &overlay)
    }

    fn apply_double_battle_patches(&mut self) -> Result<()> {
        self.update_trainer_text_boxes()?;
        self.patch_field_overlay_for_doubles()?;
        let prefix = self.entry.string("TextBoxChoicePrefix");
        let offset = Self::prefixed_offset(&self.arm9, prefix, prefix)?;
        self.arm9[(offset - 4) as usize] = 2;
        Ok(())
    }

    pub(crate) fn trainer_names(&self) -> Result<Vec<String>> {
        let mut names = self.get_strings(false, self.entry.int("TrainerNamesTextOffset"))?;
        if !names.is_empty() {
            names.remove(0);
        }
        if self.is_bw2() {
            names.extend(self.get_strings(false, self.entry.int("PWTTrainerNamesTextOffset"))?);
        }
        let mugshots = self.get_strings(false, self.entry.int("TrainerMugshotsTextOffset"))?;
        names.extend(
            mugshots
                .into_iter()
                .filter(|name| starts_with_capital(name)),
        );
        Ok(names)
    }

    pub(crate) fn set_trainer_names_impl(&mut self, trainer_names: &[String]) -> Result<()> {
        let mut trainer_names = trainer_names.to_vec();
        let names_index = self.entry.int("TrainerNamesTextOffset");
        let mugshots_index = self.entry.int("TrainerMugshotsTextOffset");
        let names = self.get_strings(false, names_index)?;
        let mut mugshots = self.get_strings(false, mugshots_index)?;
        for index in (0..mugshots.len()).rev() {
            if starts_with_capital(&mugshots[index]) {
                mugshots[index] = trainer_names
                    .pop()
                    .ok_or(RandomizationError::IndexOutOfBounds("trainerNames"))?;
            }
        }
        self.set_strings(false, mugshots_index, &mugshots)?;
        let first = names
            .first()
            .cloned()
            .ok_or(RandomizationError::IndexOutOfBounds("tnames"))?;
        if self.is_bw2() {
            let pwt_index = self.entry.int("PWTTrainerNamesTextOffset");
            let pwt_names = self.get_strings(false, pwt_index)?;
            let mut new_names = vec![first];
            let mut new_pwt_names = Vec::new();
            for index in 1..names.len() + pwt_names.len() {
                let name = trainer_names
                    .get(index - 1)
                    .cloned()
                    .ok_or(RandomizationError::IndexOutOfBounds("trainerNames"))?;
                if index < names.len() {
                    new_names.push(name);
                } else {
                    new_pwt_names.push(name);
                }
            }
            self.set_strings(false, names_index, &new_names)?;
            self.set_strings(false, pwt_index, &new_pwt_names)
        } else {
            let mut new_names = vec![first];
            new_names.extend(trainer_names);
            self.set_strings(false, names_index, &new_names)
        }
    }

    pub(crate) fn trainer_class_names(&self) -> Result<Vec<String>> {
        let mut names = self.get_strings(false, self.entry.int("TrainerClassesTextOffset"))?;
        if self.is_bw2() {
            names.extend(self.get_strings(false, self.entry.int("PWTTrainerClassesTextOffset"))?);
        }
        Ok(names)
    }

    pub(crate) fn set_trainer_class_names_impl(&mut self, class_names: &[String]) -> Result<()> {
        let classes_index = self.entry.int("TrainerClassesTextOffset");
        if !self.is_bw2() {
            return self.set_strings(false, classes_index, class_names);
        }
        let pwt_index = self.entry.int("PWTTrainerClassesTextOffset");
        let class_count = self.get_strings(false, classes_index)?.len();
        let pwt_count = self.get_strings(false, pwt_index)?.len();
        let take = |index: usize| {
            class_names
                .get(index)
                .cloned()
                .ok_or(RandomizationError::IndexOutOfBounds("trainerClassNames"))
        };
        let new_classes = (0..class_count).map(take).collect::<Result<Vec<_>>>()?;
        let new_pwt_classes = (class_count..class_count + pwt_count)
            .map(take)
            .collect::<Result<Vec<_>>>()?;
        self.set_strings(false, classes_index, &new_classes)?;
        self.set_strings(false, pwt_index, &new_pwt_classes)
    }

    pub(crate) fn ability_for_trainer_pokemon(&self, pokemon: &TrainerPokemon) -> i32 {
        if pokemon.ability_slot < 1 || pokemon.ability_slot > 3 {
            return 0;
        }
        let data = &self.core.data;
        let mut current = pokemon.pokemon;
        while let Some(base) = data.pk(current).base_forme {
            current = base;
        }
        let pk = data.pk(current);
        [pk.ability1, pk.ability2, pk.ability3][(pokemon.ability_slot - 1) as usize]
    }

    fn add_move_items(&self, items_list: &mut Vec<i32>, move_number: i32, consumable_only: bool) {
        let Some(mv) = self.core.data.move_by_number(move_number) else {
            return;
        };
        let kind = mv.kind;
        let type_items = || kind.and_then(gen5::type_boosting_items).unwrap_or_default();
        let consumable = || kind.and_then(gen5::consumable_type_boosting_item);
        match mv.category {
            MoveCategory::Physical => {
                items_list.push(items::LIECHI_BERRY);
                items_list.extend(consumable());
                if !consumable_only {
                    items_list.extend(type_items());
                    items_list.push(items::CHOICE_BAND);
                    items_list.push(items::MUSCLE_BAND);
                }
            }
            MoveCategory::Special => {
                items_list.push(items::PETAYA_BERRY);
                items_list.extend(consumable());
                if !consumable_only {
                    items_list.extend(type_items());
                    items_list.push(items::WISE_GLASSES);
                    items_list.push(items::CHOICE_SPECS);
                }
            }
            MoveCategory::Status => {}
        }
        if !consumable_only && let Some(boosting) = gen5::move_boosting_items(move_number) {
            items_list.extend(boosting);
        }
    }

    pub(crate) fn sensible_held_items_for(
        &self,
        pokemon: &TrainerPokemon,
        consumable_only: bool,
        moveset: &[i32; 4],
    ) -> Vec<i32> {
        let mut items_list = gen5::general_purpose_consumable_items();
        let mut frequency_boost_count = 6;
        if !consumable_only {
            frequency_boost_count = 8;
            items_list.extend(gen5::general_purpose_items());
        }
        for &move_number in moveset {
            self.add_move_items(&mut items_list, move_number, consumable_only);
        }
        let pk = self.core.data.pk(pokemon.pokemon);
        let primary = pk.primary_type.unwrap_or(Type::Normal);
        let by_type =
            Effectiveness::against(primary, pk.secondary_type, 5, self.effectiveness_updated)
                .unwrap_or_default();
        let effectiveness_of = |kind: Type| {
            by_type
                .iter()
                .find(|(attacker, _)| *attacker == kind)
                .map(|(_, effect)| *effect)
        };
        for &(kind, effect) in &by_type {
            let berry = gen5::weakness_reducing_berry(kind);
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
        if effectiveness_of(Type::Normal) == Some(Effectiveness::Neutral) {
            items_list.push(items::CHILAN_BERRY);
        }
        let ability = self.ability_for_trainer_pokemon(pokemon);
        if ability == abilities::LEVITATE {
            items_list.retain(|&item| item != items::SHUCA_BERRY);
        } else if matches!(
            effectiveness_of(Type::Ground),
            Some(Effectiveness::Double) | Some(Effectiveness::Quadruple)
        ) {
            items_list.push(items::AIR_BALLOON);
        }
        if !consumable_only {
            if let Some(boosting) = gen5::ability_boosting_items(ability) {
                items_list.extend(boosting);
            }
            if pk.primary_type == Some(Type::Poison) || pk.secondary_type == Some(Type::Poison) {
                items_list.push(items::BLACK_SLUDGE);
            }
            if let Some(species_items) = gen5::species_boosting_items(pk.number) {
                for _ in 0..frequency_boost_count {
                    items_list.extend(species_items.iter().copied());
                }
            }
            if !self.core.data.evolutions_from(pokemon.pokemon).is_empty() && pokemon.level >= 20 {
                items_list.push(items::EVIOLITE);
            }
        }
        items_list
    }
}
