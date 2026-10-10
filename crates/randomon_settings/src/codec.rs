use crate::enums::*;
use crate::error::SettingsError;
use crate::gen_restrictions::GenRestrictions;
use crate::java_text::{decode_us_ascii, encode_us_ascii};
use crate::settings::{LENGTH_OF_SETTINGS_DATA, Settings};

const CHECKSUM_TRAILER_LENGTH: usize = 8;

fn flags(bits: &[bool]) -> i32 {
    bits.iter()
        .enumerate()
        .fold(0, |byte, (index, &set)| byte | (i32::from(set) << index))
}

fn high_bit_if(set: bool) -> i32 {
    if set { 0x80 } else { 0 }
}

fn offset_level_modifier(modified: bool, modifier: i32) -> i32 {
    high_bit_if(modified) | modifier.wrapping_add(50)
}

fn crc32(bytes: &[u8]) -> i32 {
    crc32fast::hash(bytes) as i32
}

struct ByteSink {
    bytes: Vec<u8>,
}

impl ByteSink {
    fn write(&mut self, value: i32) {
        self.bytes.push(value as u8);
    }

    fn write_u16_le(&mut self, value: i32) {
        self.write(value & 0xFF);
        self.write((value >> 8) & 0xFF);
    }

    fn write_i32_be(&mut self, value: i32) {
        self.bytes.extend_from_slice(&value.to_be_bytes());
    }
}

pub fn encode(settings: &Settings, custom_names_checksum: i32) -> Result<Vec<u8>, SettingsError> {
    let mut sink = ByteSink {
        bytes: Vec::with_capacity(LENGTH_OF_SETTINGS_DATA + 64),
    };
    write_pokemon_bytes(&mut sink, settings);
    write_move_and_trainer_bytes(&mut sink, settings);
    write_wild_static_and_tm_bytes(&mut sink, settings);
    write_tutor_trade_and_item_bytes(&mut sink, settings);
    write_restriction_and_tweak_bytes(&mut sink, settings);
    write_late_bytes(&mut sink, settings)?;
    let rom_name = settings
        .rom_name
        .as_deref()
        .ok_or(SettingsError::MissingRomName)?;
    let rom_name_bytes = encode_us_ascii(rom_name);
    sink.write(rom_name_bytes.len() as i32);
    sink.bytes.extend_from_slice(&rom_name_bytes);
    let checksum = crc32(&sink.bytes);
    sink.write_i32_be(checksum);
    sink.write_i32_be(custom_names_checksum);
    Ok(sink.bytes)
}

fn write_pokemon_bytes(sink: &mut ByteSink, s: &Settings) {
    sink.write(flags(&[
        s.change_impossible_evolutions,
        s.update_moves,
        s.update_moves_legacy,
        s.randomize_trainer_names,
        s.randomize_trainer_class_names,
        s.make_evolutions_easier,
        s.remove_time_based_evolutions,
    ]));
    sink.write(flags(&[
        s.base_stats_follow_evolutions,
        s.base_statistics_mod == BaseStatisticsMod::Random,
        s.base_statistics_mod == BaseStatisticsMod::Shuffle,
        s.base_statistics_mod == BaseStatisticsMod::Unchanged,
        s.standardize_exp_curves,
        s.update_base_stats,
        s.base_stats_follow_mega_evolutions,
        s.assign_evo_stats_randomly,
    ]));
    sink.write(flags(&[
        s.types_mod == TypesMod::RandomFollowEvolutions,
        s.types_mod == TypesMod::CompletelyRandom,
        s.types_mod == TypesMod::Unchanged,
        s.race_mode,
        s.block_broken_moves,
        s.limit_pokemon,
        s.types_follow_mega_evolutions,
        s.dual_type_only,
    ]));
    sink.write(flags(&[
        s.abilities_mod == AbilitiesMod::Unchanged,
        s.abilities_mod == AbilitiesMod::Randomize,
        s.allow_wonder_guard,
        s.abilities_follow_evolutions,
        s.ban_trapping_abilities,
        s.ban_negative_abilities,
        s.ban_bad_abilities,
        s.abilities_follow_mega_evolutions,
    ]));
    sink.write(flags(&[
        s.starters_mod == StartersMod::Custom,
        s.starters_mod == StartersMod::CompletelyRandom,
        s.starters_mod == StartersMod::Unchanged,
        s.starters_mod == StartersMod::RandomWithTwoEvolutions,
        s.randomize_starters_held_items,
        s.ban_bad_random_starter_held_items,
        s.allow_starter_alt_formes,
    ]));
    for starter in s.custom_starters {
        sink.write_u16_le(starter.wrapping_sub(1));
    }
}

fn write_move_and_trainer_bytes(sink: &mut ByteSink, s: &Settings) {
    sink.write(
        flags(&[
            s.movesets_mod == MovesetsMod::CompletelyRandom,
            s.movesets_mod == MovesetsMod::RandomPreferSameType,
            s.movesets_mod == MovesetsMod::Unchanged,
            s.movesets_mod == MovesetsMod::MetronomeOnly,
            s.start_with_guaranteed_moves,
            s.reorder_damaging_moves,
        ]) | (s.guaranteed_move_count.wrapping_sub(2) << 6),
    );
    sink.write(high_bit_if(s.movesets_force_good_damaging) | s.movesets_good_damaging_percent);
    sink.write(flags(&[
        s.trainers_mod == TrainersMod::Unchanged,
        s.trainers_mod == TrainersMod::Random,
        s.trainers_mod == TrainersMod::Distributed,
        s.trainers_mod == TrainersMod::MainPlaythrough,
        s.trainers_mod == TrainersMod::TypeThemed,
        s.trainers_mod == TrainersMod::TypeThemedElite4Gyms,
    ]));
    sink.write(high_bit_if(s.trainers_force_fully_evolved) | s.trainers_force_fully_evolved_level);
}

fn write_wild_static_and_tm_bytes(sink: &mut ByteSink, s: &Settings) {
    let restriction = s.wild_pokemon_restriction_mod;
    sink.write(flags(&[
        restriction == WildPokemonRestrictionMod::CatchEmAll,
        s.wild_pokemon_mod == WildPokemonMod::AreaMapping,
        restriction == WildPokemonRestrictionMod::None,
        restriction == WildPokemonRestrictionMod::TypeThemeAreas,
        s.wild_pokemon_mod == WildPokemonMod::GlobalMapping,
        s.wild_pokemon_mod == WildPokemonMod::Random,
        s.wild_pokemon_mod == WildPokemonMod::Unchanged,
        s.use_time_based_encounters,
    ]));
    sink.write(flags(&[
        s.use_minimum_catch_rate,
        s.block_wild_legendaries,
        restriction == WildPokemonRestrictionMod::SimilarStrength,
        s.randomize_wild_pokemon_held_items,
        s.ban_bad_random_wild_pokemon_held_items,
        false,
        false,
        s.balance_shaking_grass,
    ]));
    sink.write(flags(&[
        s.static_pokemon_mod == StaticPokemonMod::Unchanged,
        s.static_pokemon_mod == StaticPokemonMod::RandomMatching,
        s.static_pokemon_mod == StaticPokemonMod::CompletelyRandom,
        s.static_pokemon_mod == StaticPokemonMod::SimilarStrength,
        s.limit_main_game_legendaries,
        s.limit600,
        s.allow_static_alt_formes,
        s.swap_static_mega_evos,
    ]));
    let compatibility = s.tms_hms_compatibility_mod;
    sink.write(flags(&[
        compatibility == TmsHmsCompatibilityMod::CompletelyRandom,
        compatibility == TmsHmsCompatibilityMod::RandomPreferType,
        compatibility == TmsHmsCompatibilityMod::Unchanged,
        s.tms_mod == TmsMod::Random,
        s.tms_mod == TmsMod::Unchanged,
        s.tm_level_up_move_sanity,
        s.keep_field_move_tms,
        compatibility == TmsHmsCompatibilityMod::Full,
    ]));
    sink.write(flags(&[
        s.full_hm_compat,
        s.tms_follow_evolutions,
        s.tutor_follow_evolutions,
    ]));
    sink.write(high_bit_if(s.tms_force_good_damaging) | s.tms_good_damaging_percent);
}

fn write_tutor_trade_and_item_bytes(sink: &mut ByteSink, s: &Settings) {
    let compatibility = s.move_tutors_compatibility_mod;
    sink.write(flags(&[
        compatibility == MoveTutorsCompatibilityMod::CompletelyRandom,
        compatibility == MoveTutorsCompatibilityMod::RandomPreferType,
        compatibility == MoveTutorsCompatibilityMod::Unchanged,
        s.move_tutor_moves_mod == MoveTutorMovesMod::Random,
        s.move_tutor_moves_mod == MoveTutorMovesMod::Unchanged,
        s.tutor_level_up_move_sanity,
        s.keep_field_move_tutors,
        compatibility == MoveTutorsCompatibilityMod::Full,
    ]));
    sink.write(high_bit_if(s.tutors_force_good_damaging) | s.tutors_good_damaging_percent);
    sink.write(flags(&[
        s.in_game_trades_mod == InGameTradesMod::RandomizeGivenAndRequested,
        s.in_game_trades_mod == InGameTradesMod::RandomizeGiven,
        s.randomize_in_game_trades_items,
        s.randomize_in_game_trades_ivs,
        s.randomize_in_game_trades_nicknames,
        s.randomize_in_game_trades_ots,
        s.in_game_trades_mod == InGameTradesMod::Unchanged,
    ]));
    sink.write(flags(&[
        s.field_items_mod == FieldItemsMod::Random,
        s.field_items_mod == FieldItemsMod::Shuffle,
        s.field_items_mod == FieldItemsMod::Unchanged,
        s.ban_bad_random_field_items,
        s.field_items_mod == FieldItemsMod::RandomEven,
    ]));
    sink.write(flags(&[
        s.randomize_move_powers,
        s.randomize_move_accuracies,
        s.randomize_move_pps,
        s.randomize_move_types,
        s.randomize_move_category,
        s.correct_static_music,
    ]));
    sink.write(flags(&[
        s.evolutions_mod == EvolutionsMod::Unchanged,
        s.evolutions_mod == EvolutionsMod::Random,
        s.evos_similar_strength,
        s.evos_same_typing,
        s.evos_max_three_stages,
        s.evos_force_change,
        s.evos_allow_alt_formes,
        s.evolutions_mod == EvolutionsMod::RandomEveryLevel,
    ]));
    sink.write(flags(&[
        s.trainers_use_pokemon_of_similar_strength,
        s.rival_carries_starter_throughout,
        s.trainers_match_typing_distribution,
        s.trainers_block_legendaries,
        s.trainers_block_early_wonder_guard,
        s.swap_trainer_mega_evos,
        s.shiny_chance,
        s.better_trainer_movesets,
    ]));
}

fn write_restriction_and_tweak_bytes(sink: &mut ByteSink, s: &Settings) {
    sink.write_i32_be(s.current_restrictions.map_or(0, GenRestrictions::to_state));
    sink.write_i32_be(s.current_misc_tweaks);
    sink.write(offset_level_modifier(
        s.trainers_level_modified,
        s.trainers_level_modifier,
    ));
    sink.write(flags(&[
        s.shop_items_mod == ShopItemsMod::Random,
        s.shop_items_mod == ShopItemsMod::Shuffle,
        s.shop_items_mod == ShopItemsMod::Unchanged,
        s.ban_bad_random_shop_items,
        s.ban_regular_shop_items,
        s.ban_op_shop_items,
        s.balance_shop_prices,
        s.guarantee_evolution_items,
    ]));
    sink.write(offset_level_modifier(
        s.wild_levels_modified,
        s.wild_level_modifier,
    ));
    sink.write(flags(&[
        s.exp_curve_mod == ExpCurveMod::Legendaries,
        s.exp_curve_mod == ExpCurveMod::StrongLegendaries,
        s.exp_curve_mod == ExpCurveMod::All,
        s.block_broken_moveset_moves,
        s.block_broken_tm_moves,
        s.block_broken_tutor_moves,
        s.allow_trainer_alternate_formes,
        s.allow_wild_alt_formes,
    ]));
}

fn write_late_bytes(sink: &mut ByteSink, s: &Settings) -> Result<(), SettingsError> {
    sink.write(
        i32::from(s.double_battle_mode)
            | (s.additional_boss_trainer_pokemon << 1)
            | (s.additional_important_trainer_pokemon << 4)
            | high_bit_if(s.weigh_duplicate_abilities_together),
    );
    sink.write(
        s.additional_regular_trainer_pokemon
            | flags(&[
                false,
                false,
                false,
                s.aura_mod == AuraMod::Unchanged,
                s.aura_mod == AuraMod::Random,
                s.aura_mod == AuraMod::SameStrength,
                s.evolution_moves_for_all,
                s.guarantee_x_items,
            ]),
    );
    sink.write(flags(&[
        s.totem_pokemon_mod == TotemPokemonMod::Unchanged,
        s.totem_pokemon_mod == TotemPokemonMod::Random,
        s.totem_pokemon_mod == TotemPokemonMod::SimilarStrength,
        s.ally_pokemon_mod == AllyPokemonMod::Unchanged,
        s.ally_pokemon_mod == AllyPokemonMod::Random,
        s.ally_pokemon_mod == AllyPokemonMod::SimilarStrength,
        s.randomize_totem_held_items,
        s.allow_totem_alt_formes,
    ]));
    sink.write(offset_level_modifier(
        s.totem_levels_modified,
        s.totem_level_modifier,
    ));
    sink.write(s.update_base_stats_to_generation);
    sink.write(s.update_moves_to_generation);
    let curve = s.selected_exp_curve.ok_or(SettingsError::MissingExpCurve)?;
    sink.write(i32::from(curve.to_byte()));
    sink.write(offset_level_modifier(
        s.static_level_modified,
        s.static_level_modifier,
    ));
    sink.write(flags(&[
        s.randomize_held_items_for_boss_trainer_pokemon,
        s.randomize_held_items_for_important_trainer_pokemon,
        s.randomize_held_items_for_regular_trainer_pokemon,
        s.consumable_items_only_for_trainer_pokemon,
        s.sensible_items_only_for_trainer_pokemon,
        s.highest_level_only_gets_items_for_trainer_pokemon,
        s.ensure_two_abilities,
    ]));
    sink.write(flags(&[
        s.pickup_items_mod == PickupItemsMod::Random,
        s.pickup_items_mod == PickupItemsMod::Unchanged,
        s.ban_bad_random_pickup_items,
        s.ban_irregular_alt_formes,
    ]));
    sink.write(
        s.elite_four_unique_pokemon_number | (s.minimum_catch_rate_level.wrapping_sub(1) << 3),
    );
    Ok(())
}

pub fn verify_checksum(data: &[u8]) -> Result<(), SettingsError> {
    let body_length = data
        .len()
        .checked_sub(CHECKSUM_TRAILER_LENGTH)
        .ok_or(SettingsError::TruncatedSettings { length: data.len() })?;
    let stored = i32::from_be_bytes([
        data[body_length],
        data[body_length + 1],
        data[body_length + 2],
        data[body_length + 3],
    ]);
    if crc32(&data[..body_length]) != stored {
        return Err(SettingsError::ChecksumMismatch);
    }
    Ok(())
}

fn bit(byte: u8, index: u32) -> bool {
    (byte >> index) & 1 == 1
}

fn choose_enum<E: JavaEnum>(bits: &[bool]) -> Result<E, SettingsError> {
    let mut chosen = None;
    for (index, &set) in bits.iter().enumerate() {
        if set {
            if chosen.is_some() {
                return Err(SettingsError::ConflictingEnumBits(E::TYPE_NAME));
            }
            chosen = Some(index);
        }
    }
    E::from_ordinal(chosen.unwrap_or(0)).ok_or(SettingsError::ConflictingEnumBits(E::TYPE_NAME))
}

fn restore_enum<E: JavaEnum>(byte: u8, indices: &[u32]) -> Result<E, SettingsError> {
    let bits: Vec<bool> = indices.iter().map(|&index| bit(byte, index)).collect();
    choose_enum(&bits)
}

fn read_i32_be(data: &[u8], offset: usize) -> i32 {
    i32::from_be_bytes([
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
    ])
}

fn read_u16_le(data: &[u8], offset: usize) -> i32 {
    i32::from(data[offset]) | (i32::from(data[offset + 1]) << 8)
}

fn low_seven_bits(byte: u8) -> i32 {
    i32::from(byte & 0x7F)
}

pub fn decode(data: &[u8]) -> Result<Settings, SettingsError> {
    verify_checksum(data)?;
    if data.len() <= LENGTH_OF_SETTINGS_DATA {
        return Err(SettingsError::TruncatedSettings { length: data.len() });
    }
    let mut s = Settings::default();
    read_pokemon_bytes(&mut s, data)?;
    read_move_trainer_and_wild_bytes(&mut s, data)?;
    read_static_tm_tutor_and_trade_bytes(&mut s, data)?;
    read_restriction_and_tweak_bytes(&mut s, data)?;
    read_late_bytes(&mut s, data)?;
    s.rom_name = Some(read_rom_name(data)?);
    Ok(s)
}

fn read_pokemon_bytes(s: &mut Settings, d: &[u8]) -> Result<(), SettingsError> {
    s.change_impossible_evolutions = bit(d[0], 0);
    s.update_moves = bit(d[0], 1);
    s.update_moves_legacy = bit(d[0], 2);
    s.randomize_trainer_names = bit(d[0], 3);
    s.randomize_trainer_class_names = bit(d[0], 4);
    s.make_evolutions_easier = bit(d[0], 5);
    s.remove_time_based_evolutions = bit(d[0], 6);

    s.base_statistics_mod = restore_enum(d[1], &[3, 2, 1])?;
    s.standardize_exp_curves = bit(d[1], 4);
    s.base_stats_follow_evolutions = bit(d[1], 0);
    s.update_base_stats = bit(d[1], 5);
    s.base_stats_follow_mega_evolutions = bit(d[1], 6);
    s.assign_evo_stats_randomly = bit(d[1], 7);

    s.types_mod = restore_enum(d[2], &[2, 0, 1])?;
    s.race_mode = bit(d[2], 3);
    s.set_all_block_broken_moves(bit(d[2], 4));
    s.limit_pokemon = bit(d[2], 5);
    s.types_follow_mega_evolutions = bit(d[2], 6);
    s.dual_type_only = bit(d[2], 7);

    s.abilities_mod = restore_enum(d[3], &[0, 1])?;
    s.allow_wonder_guard = bit(d[3], 2);
    s.abilities_follow_evolutions = bit(d[3], 3);
    s.ban_trapping_abilities = bit(d[3], 4);
    s.ban_negative_abilities = bit(d[3], 5);
    s.ban_bad_abilities = bit(d[3], 6);
    s.abilities_follow_mega_evolutions = bit(d[3], 7);

    s.starters_mod = restore_enum(d[4], &[2, 0, 1, 3])?;
    s.randomize_starters_held_items = bit(d[4], 4);
    s.ban_bad_random_starter_held_items = bit(d[4], 5);
    s.allow_starter_alt_formes = bit(d[4], 6);

    s.custom_starters = [
        read_u16_le(d, 5) + 1,
        read_u16_le(d, 7) + 1,
        read_u16_le(d, 9) + 1,
    ];
    Ok(())
}

fn read_move_trainer_and_wild_bytes(s: &mut Settings, d: &[u8]) -> Result<(), SettingsError> {
    s.movesets_mod = restore_enum(d[11], &[2, 1, 0, 3])?;
    s.start_with_guaranteed_moves = bit(d[11], 4);
    s.reorder_damaging_moves = bit(d[11], 5);
    s.guaranteed_move_count = i32::from((d[11] & 0xC0) >> 6) + 2;

    s.movesets_force_good_damaging = bit(d[12], 7);
    s.movesets_good_damaging_percent = low_seven_bits(d[12]);

    s.trainers_mod = restore_enum(d[13], &[0, 1, 2, 3, 4, 5])?;

    s.trainers_force_fully_evolved = bit(d[14], 7);
    s.trainers_force_fully_evolved_level = low_seven_bits(d[14]);

    s.wild_pokemon_mod = restore_enum(d[15], &[6, 5, 1, 4])?;
    s.wild_pokemon_restriction_mod =
        choose_enum(&[bit(d[15], 2), bit(d[16], 2), bit(d[15], 0), bit(d[15], 3)])?;
    s.use_time_based_encounters = bit(d[15], 7);

    s.use_minimum_catch_rate = bit(d[16], 0);
    s.block_wild_legendaries = bit(d[16], 1);
    s.randomize_wild_pokemon_held_items = bit(d[16], 3);
    s.ban_bad_random_wild_pokemon_held_items = bit(d[16], 4);
    s.balance_shaking_grass = bit(d[16], 7);
    Ok(())
}

fn read_static_tm_tutor_and_trade_bytes(s: &mut Settings, d: &[u8]) -> Result<(), SettingsError> {
    s.static_pokemon_mod = restore_enum(d[17], &[0, 1, 2, 3])?;
    s.limit_main_game_legendaries = bit(d[17], 4);
    s.limit600 = bit(d[17], 5);
    s.allow_static_alt_formes = bit(d[17], 6);
    s.swap_static_mega_evos = bit(d[17], 7);

    s.tms_mod = restore_enum(d[18], &[4, 3])?;
    s.tms_hms_compatibility_mod = restore_enum(d[18], &[2, 1, 0, 7])?;
    s.tm_level_up_move_sanity = bit(d[18], 5);
    s.keep_field_move_tms = bit(d[18], 6);

    s.full_hm_compat = bit(d[19], 0);
    s.tms_follow_evolutions = bit(d[19], 1);
    s.tutor_follow_evolutions = bit(d[19], 2);

    s.tms_force_good_damaging = bit(d[20], 7);
    s.tms_good_damaging_percent = low_seven_bits(d[20]);

    s.move_tutor_moves_mod = restore_enum(d[21], &[4, 3])?;
    s.move_tutors_compatibility_mod = restore_enum(d[21], &[2, 1, 0, 7])?;
    s.tutor_level_up_move_sanity = bit(d[21], 5);
    s.keep_field_move_tutors = bit(d[21], 6);

    s.tutors_force_good_damaging = bit(d[22], 7);
    s.tutors_good_damaging_percent = low_seven_bits(d[22]);

    s.in_game_trades_mod = restore_enum(d[23], &[6, 1, 0])?;
    s.randomize_in_game_trades_items = bit(d[23], 2);
    s.randomize_in_game_trades_ivs = bit(d[23], 3);
    s.randomize_in_game_trades_nicknames = bit(d[23], 4);
    s.randomize_in_game_trades_ots = bit(d[23], 5);

    s.field_items_mod = restore_enum(d[24], &[2, 1, 0, 4])?;
    s.ban_bad_random_field_items = bit(d[24], 3);

    s.randomize_move_powers = bit(d[25], 0);
    s.randomize_move_accuracies = bit(d[25], 1);
    s.randomize_move_pps = bit(d[25], 2);
    s.randomize_move_types = bit(d[25], 3);
    s.randomize_move_category = bit(d[25], 4);
    s.correct_static_music = bit(d[25], 5);

    s.evolutions_mod = restore_enum(d[26], &[0, 1, 7])?;
    s.evos_similar_strength = bit(d[26], 2);
    s.evos_same_typing = bit(d[26], 3);
    s.evos_max_three_stages = bit(d[26], 4);
    s.evos_force_change = bit(d[26], 5);
    s.evos_allow_alt_formes = bit(d[26], 6);

    s.trainers_use_pokemon_of_similar_strength = bit(d[27], 0);
    s.rival_carries_starter_throughout = bit(d[27], 1);
    s.trainers_match_typing_distribution = bit(d[27], 2);
    s.trainers_block_legendaries = bit(d[27], 3);
    s.trainers_block_early_wonder_guard = bit(d[27], 4);
    s.swap_trainer_mega_evos = bit(d[27], 5);
    s.shiny_chance = bit(d[27], 6);
    s.better_trainer_movesets = bit(d[27], 7);
    Ok(())
}

fn read_restriction_and_tweak_bytes(s: &mut Settings, d: &[u8]) -> Result<(), SettingsError> {
    let generation_limit = read_i32_be(d, 28);
    s.current_restrictions =
        (generation_limit != 0).then(|| GenRestrictions::from_state(generation_limit));
    s.current_misc_tweaks = read_i32_be(d, 32);

    s.trainers_level_modified = bit(d[36], 7);
    s.trainers_level_modifier = low_seven_bits(d[36]) - 50;
    s.shop_items_mod = restore_enum(d[37], &[2, 1, 0])?;
    s.ban_bad_random_shop_items = bit(d[37], 3);
    s.ban_regular_shop_items = bit(d[37], 4);
    s.ban_op_shop_items = bit(d[37], 5);
    s.balance_shop_prices = bit(d[37], 6);
    s.guarantee_evolution_items = bit(d[37], 7);

    s.wild_levels_modified = bit(d[38], 7);
    s.wild_level_modifier = low_seven_bits(d[38]) - 50;

    s.exp_curve_mod = restore_enum(d[39], &[0, 1, 2])?;
    s.block_broken_moveset_moves = bit(d[39], 3);
    s.block_broken_tm_moves = bit(d[39], 4);
    s.block_broken_tutor_moves = bit(d[39], 5);
    s.allow_trainer_alternate_formes = bit(d[39], 6);
    s.allow_wild_alt_formes = bit(d[39], 7);
    Ok(())
}

fn read_late_bytes(s: &mut Settings, d: &[u8]) -> Result<(), SettingsError> {
    s.double_battle_mode = bit(d[40], 0);
    s.additional_boss_trainer_pokemon = i32::from((d[40] & 0x0E) >> 1);
    s.additional_important_trainer_pokemon = i32::from((d[40] & 0x70) >> 4);
    s.weigh_duplicate_abilities_together = bit(d[40], 7);

    s.additional_regular_trainer_pokemon = i32::from(d[41] & 0x07);
    s.aura_mod = restore_enum(d[41], &[3, 4, 5])?;
    s.evolution_moves_for_all = bit(d[41], 6);
    s.guarantee_x_items = bit(d[41], 7);

    s.totem_pokemon_mod = restore_enum(d[42], &[0, 1, 2])?;
    s.ally_pokemon_mod = restore_enum(d[42], &[3, 4, 5])?;
    s.randomize_totem_held_items = bit(d[42], 6);
    s.allow_totem_alt_formes = bit(d[42], 7);
    s.totem_levels_modified = bit(d[43], 7);
    s.totem_level_modifier = low_seven_bits(d[43]) - 50;

    s.update_base_stats_to_generation = i32::from(d[44] as i8);
    s.update_moves_to_generation = i32::from(d[45] as i8);
    s.selected_exp_curve = ExpCurve::from_byte(d[46] as i8);

    s.static_level_modified = bit(d[47], 7);
    s.static_level_modifier = low_seven_bits(d[47]) - 50;

    s.randomize_held_items_for_boss_trainer_pokemon = bit(d[48], 0);
    s.randomize_held_items_for_important_trainer_pokemon = bit(d[48], 1);
    s.randomize_held_items_for_regular_trainer_pokemon = bit(d[48], 2);
    s.consumable_items_only_for_trainer_pokemon = bit(d[48], 3);
    s.sensible_items_only_for_trainer_pokemon = bit(d[48], 4);
    s.highest_level_only_gets_items_for_trainer_pokemon = bit(d[48], 5);
    s.ensure_two_abilities = bit(d[48], 6);

    s.pickup_items_mod = restore_enum(d[49], &[1, 0])?;
    s.ban_bad_random_pickup_items = bit(d[49], 2);
    s.ban_irregular_alt_formes = bit(d[49], 3);

    s.elite_four_unique_pokemon_number = i32::from(d[50] & 0x07);
    s.minimum_catch_rate_level = i32::from((d[50] & 0x38) >> 3) + 1;
    Ok(())
}

fn read_rom_name(d: &[u8]) -> Result<String, SettingsError> {
    let length = usize::from(d[LENGTH_OF_SETTINGS_DATA]);
    let start = LENGTH_OF_SETTINGS_DATA + 1;
    let bytes = d
        .get(start..start + length)
        .ok_or(SettingsError::TruncatedSettings { length: d.len() })?;
    Ok(decode_us_ascii(bytes))
}
