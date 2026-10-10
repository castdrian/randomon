use crate::constants::{abilities, moves};
use crate::effectiveness::Effectiveness;
use crate::game_data::GameData;
use crate::ids::MoveId;
use crate::moves::Move;
use crate::pokemon::Pokemon;
use crate::types::{
    CriticalChance, MoveCategory, StatChangeMoveType, StatChangeType, StatusMoveType, StatusType,
    Type,
};

pub struct MissingEffectivenessTable;

struct Collector<'a> {
    data: &'a GameData,
    move_list: &'a [MoveId],
    numbers: Vec<i32>,
}

impl<'a> Collector<'a> {
    fn new(data: &'a GameData, move_list: &'a [MoveId]) -> Self {
        Self {
            data,
            move_list,
            numbers: Vec::new(),
        }
    }

    fn add(&mut self, number: i32) {
        self.numbers.push(number);
    }

    fn add_all(&mut self, numbers: &[i32]) {
        self.numbers.extend_from_slice(numbers);
    }

    fn add_where(&mut self, predicate: impl Fn(&Move) -> bool) {
        for &id in self.move_list {
            let mv = self.data.mv(id);
            if predicate(mv) {
                self.numbers.push(mv.number);
            }
        }
    }

    fn finish(self) -> Vec<MoveId> {
        let mut result: Vec<MoveId> = Vec::new();
        for &id in self.move_list {
            if self.numbers.contains(&self.data.mv(id).number) && !result.contains(&id) {
                result.push(id);
            }
        }
        result
    }
}

fn damaging_of_type(mv: &Move, kind: Type) -> bool {
    mv.kind == Some(kind) && mv.category != MoveCategory::Status
}

fn has_type(first: Option<Type>, second: Option<Type>, kind: Type) -> bool {
    first == Some(kind) || second == Some(kind)
}

pub fn soft_ability_move_synergy(
    data: &GameData,
    ability: i32,
    move_list: &[MoveId],
    type1: Option<Type>,
    type2: Option<Type>,
) -> Vec<MoveId> {
    let mut c = Collector::new(data, move_list);
    match ability {
        abilities::DRIZZLE | abilities::PRIMORDIAL_SEA => {
            c.add_where(|mv| damaging_of_type(mv, Type::Water))
        }
        abilities::DROUGHT | abilities::DESOLATE_LAND => {
            c.add_where(|mv| damaging_of_type(mv, Type::Fire))
        }
        abilities::REFRIGERATE => {
            if has_type(type1, type2, Type::Ice) {
                c.add_where(|mv| damaging_of_type(mv, Type::Normal));
            }
        }
        abilities::GALE_WINGS => c.add_where(|mv| mv.kind == Some(Type::Flying)),
        abilities::PIXILATE => {
            if has_type(type1, type2, Type::Fairy) {
                c.add_where(|mv| damaging_of_type(mv, Type::Normal));
            }
        }
        abilities::AERILATE => {
            if has_type(type1, type2, Type::Flying) {
                c.add_where(|mv| damaging_of_type(mv, Type::Normal));
            }
        }
        abilities::DARK_AURA => c.add_where(|mv| damaging_of_type(mv, Type::Dark)),
        abilities::FAIRY_AURA => c.add_where(|mv| damaging_of_type(mv, Type::Fairy)),
        abilities::STEELWORKER | abilities::STEELY_SPIRIT => {
            c.add_where(|mv| damaging_of_type(mv, Type::Steel))
        }
        abilities::GALVANIZE => {
            if has_type(type1, type2, Type::Electric) {
                c.add_where(|mv| damaging_of_type(mv, Type::Normal));
            }
        }
        abilities::ELECTRIC_SURGE | abilities::TRANSISTOR => {
            c.add_where(|mv| damaging_of_type(mv, Type::Electric))
        }
        abilities::PSYCHIC_SURGE => c.add_where(|mv| damaging_of_type(mv, Type::Psychic)),
        abilities::GRASSY_SURGE => c.add_where(|mv| damaging_of_type(mv, Type::Grass)),
        abilities::DRAGONS_MAW => c.add_where(|mv| damaging_of_type(mv, Type::Dragon)),
        _ => {}
    }
    c.finish()
}

pub fn soft_ability_move_anti_synergy(
    data: &GameData,
    ability: i32,
    move_list: &[MoveId],
) -> Vec<MoveId> {
    let mut c = Collector::new(data, move_list);
    match ability {
        abilities::DRIZZLE => c.add_where(|mv| damaging_of_type(mv, Type::Fire)),
        abilities::DROUGHT => c.add_where(|mv| damaging_of_type(mv, Type::Water)),
        abilities::MISTY_SURGE => c.add_where(|mv| damaging_of_type(mv, Type::Dragon)),
        _ => {}
    }
    c.finish()
}

fn raises_or_lowers_user_stats(mv: &Move) -> bool {
    mv.stat_change_move_type == StatChangeMoveType::DamageUser
        || mv.stat_change_move_type == StatChangeMoveType::NoDamageUser
}

fn is_poisoning(mv: &Move) -> bool {
    mv.status_type == StatusType::Poison || mv.status_type == StatusType::ToxicPoison
}

pub fn hard_ability_move_synergy(
    data: &GameData,
    ability: i32,
    type1: Option<Type>,
    type2: Option<Type>,
    move_list: &[MoveId],
    generation: i32,
    perfect_accuracy: i32,
) -> Vec<MoveId> {
    let mut c = Collector::new(data, move_list);
    match ability {
        abilities::DRIZZLE | abilities::PRIMORDIAL_SEA => {
            c.add_all(&[moves::THUNDER, moves::HURRICANE]);
            if has_type(type1, type2, Type::Water) {
                c.add(moves::WEATHER_BALL);
            }
        }
        abilities::SPEED_BOOST => {
            c.add_all(&[moves::BATON_PASS, moves::STORED_POWER, moves::POWER_TRIP])
        }
        abilities::STURDY => {
            if generation >= 5 {
                c.add_all(&[
                    moves::ENDEAVOR,
                    moves::COUNTER,
                    moves::MIRROR_COAT,
                    moves::FLAIL,
                    moves::REVERSAL,
                ]);
            }
        }
        abilities::SAND_VEIL | abilities::SAND_RUSH => c.add(moves::SANDSTORM),
        abilities::STATIC_THE_ABILITY_NOT_THE_KEYWORD => {
            c.add_all(&[moves::SMELLING_SALTS, moves::HEX])
        }
        abilities::COMPOUND_EYES => c.add_where(|mv| mv.hitratio > 0.0 && mv.hitratio <= 80.0),
        abilities::OWN_TEMPO | abilities::TANGLED_FEET => {
            c.add_all(&[moves::PETAL_DANCE, moves::THRASH, moves::OUTRAGE]);
        }
        abilities::SHADOW_TAG | abilities::ARENA_TRAP => c.add(moves::PERISH_SONG),
        abilities::POISON_POINT => c.add_all(&[moves::VENOSHOCK, moves::HEX]),
        abilities::EFFECT_SPORE | abilities::FLAME_BODY => c.add(moves::HEX),
        abilities::SERENE_GRACE => c.add_where(|mv| {
            ((mv.stat_change_move_type == StatChangeMoveType::DamageTarget
                || mv.stat_change_move_type == StatChangeMoveType::DamageUser)
                && mv.stat_changes[0].percent_chance < 100.0)
                || (mv.status_move_type == StatusMoveType::Damage
                    && mv.status_percent_chance < 100.0)
                || mv.flinch_percent_chance > 0.0
        }),
        abilities::SWIFT_SWIM
        | abilities::RAIN_DISH
        | abilities::DRY_SKIN
        | abilities::HYDRATION => {
            c.add(moves::RAIN_DANCE);
        }
        abilities::CHLOROPHYLL | abilities::HARVEST | abilities::LEAF_GUARD => {
            c.add(moves::SUNNY_DAY)
        }
        abilities::SOUNDPROOF => c.add(moves::PERISH_SONG),
        abilities::SAND_STREAM => {
            if has_type(type1, type2, Type::Rock) {
                c.add(moves::WEATHER_BALL);
            }
        }
        abilities::EARLY_BIRD | abilities::SHED_SKIN => c.add(moves::REST),
        abilities::TRUANT => c.add(moves::TRANSFORM),
        abilities::HUSTLE => {
            c.add_where(|mv| {
                raises_or_lowers_user_stats(mv)
                    && mv.has_specific_stat_change(StatChangeType::Accuracy, true)
            });
            c.add_where(|mv| {
                mv.category == MoveCategory::Physical && mv.hitratio == f64::from(perfect_accuracy)
            });
        }
        abilities::GUTS => c.add(moves::FACADE),
        abilities::ROCK_HEAD => c.add_where(|mv| mv.recoil_percent > 0),
        abilities::DROUGHT | abilities::DESOLATE_LAND => {
            c.add_all(&[
                moves::SOLAR_BEAM,
                moves::SOLAR_BLADE,
                moves::MORNING_SUN,
                moves::SYNTHESIS,
                moves::MOONLIGHT,
            ]);
            if generation >= 5 {
                c.add(moves::GROWTH);
            }
            if has_type(type1, type2, Type::Fire) {
                c.add(moves::WEATHER_BALL);
            }
        }
        abilities::IRON_FIST => c.add_where(|mv| mv.is_punch_move),
        abilities::SNOW_CLOAK | abilities::ICE_BODY | abilities::SLUSH_RUSH => c.add(moves::HAIL),
        abilities::UNBURDEN => c.add_all(&[moves::FLING, moves::ACROBATICS]),
        abilities::SIMPLE => {
            c.add_where(|mv| raises_or_lowers_user_stats(mv) && mv.stat_changes[0].stages > 0);
            c.add(moves::ACUPRESSURE);
        }
        abilities::ADAPTABILITY => c.add_where(|mv| {
            mv.category != MoveCategory::Status && (mv.kind == type1 || mv.kind == type2)
        }),
        abilities::SKILL_LINK => c.add_where(|mv| mv.hit_count >= 3.0),
        abilities::SNIPER => c.add_where(|mv| {
            mv.critical_chance == CriticalChance::Increased
                || mv.critical_chance == CriticalChance::Guaranteed
        }),
        abilities::MAGIC_GUARD => {
            c.add_where(|mv| mv.recoil_percent > 0);
            c.add(moves::MIND_BLOWN);
        }
        abilities::STALL => c.add_all(&[moves::METAL_BURST, moves::PAYBACK]),
        abilities::SUPER_LUCK => c.add_where(|mv| mv.critical_chance == CriticalChance::Increased),
        abilities::ANALYTIC => c.add_where(|mv| mv.power > 0 && mv.priority < 0),
        abilities::NO_GUARD => c.add_where(|mv| mv.hitratio > 0.0 && mv.hitratio <= 70.0),
        abilities::TECHNICIAN => {
            c.add_where(|mv| (mv.power >= 40 && mv.power <= 60) || mv.hit_count > 1.0)
        }
        abilities::SLOW_START => c.add_all(&[
            moves::TRANSFORM,
            moves::PROTECT,
            moves::DETECT,
            moves::KINGS_SHIELD,
            moves::BANEFUL_BUNKER,
            moves::FLY,
            moves::DIG,
            moves::BOUNCE,
            moves::DIVE,
        ]),
        abilities::SNOW_WARNING => {
            c.add_all(&[moves::AURORA_VEIL, moves::BLIZZARD]);
            if has_type(type1, type2, Type::Ice) {
                c.add(moves::WEATHER_BALL);
            }
        }
        abilities::RECKLESS => {
            c.add_where(|mv| mv.recoil_percent > 0);
            c.add_all(&[moves::JUMP_KICK, moves::HIGH_JUMP_KICK]);
        }
        abilities::BAD_DREAMS => c.add_where(|mv| mv.status_type == StatusType::Sleep),
        abilities::SHEER_FORCE => c.add_where(|mv| {
            mv.stat_change_move_type == StatChangeMoveType::DamageTarget
                || (mv.stat_change_move_type == StatChangeMoveType::DamageUser
                    && mv.stat_changes[0].stages > 0)
                || mv.status_move_type == StatusMoveType::Damage
                || mv.flinch_percent_chance > 0.0
        }),
        abilities::CONTRARY => c.add_where(|mv| {
            mv.stat_change_move_type == StatChangeMoveType::DamageUser
                && mv.stat_changes[0].stages < 0
        }),
        abilities::HEAVY_METAL => c.add_all(&[moves::HEAT_CRASH, moves::HEAVY_SLAM]),
        abilities::MOODY => c.add_all(&[moves::STORED_POWER, moves::POWER_TRIP]),
        abilities::POISON_TOUCH | abilities::TOUGH_CLAWS => c.add_where(|mv| mv.makes_contact),
        abilities::REGENERATOR => {
            c.add_all(&[moves::U_TURN, moves::VOLT_SWITCH, moves::PARTING_SHOT])
        }
        abilities::PRANKSTER => {
            c.add_where(|mv| mv.status_move_type == StatusMoveType::NoDamage);
            c.add_all(&[
                moves::DESTINY_BOND,
                moves::ENCORE,
                moves::REFLECT,
                moves::LIGHT_SCREEN,
                moves::GRUDGE,
                moves::PAIN_SPLIT,
                moves::SUBSTITUTE,
            ]);
        }
        abilities::STRONG_JAW => c.add_all(&[
            moves::BITE,
            moves::CRUNCH,
            moves::FIRE_FANG,
            moves::FISHIOUS_REND,
            moves::HYPER_FANG,
            moves::ICE_FANG,
            moves::JAW_LOCK,
            moves::POISON_FANG,
            moves::PSYCHIC_FANGS,
            moves::THUNDER_FANG,
        ]),
        abilities::MEGA_LAUNCHER => c.add_all(&[
            moves::AURA_SPHERE,
            moves::DARK_PULSE,
            moves::DRAGON_PULSE,
            moves::ORIGIN_PULSE,
            moves::TERRAIN_PULSE,
            moves::WATER_PULSE,
        ]),
        abilities::WIMP_OUT | abilities::EMERGENCY_EXIT => {
            c.add_all(&[moves::FAKE_OUT, moves::FIRST_IMPRESSION])
        }
        abilities::MERCILESS => {
            c.add_where(is_poisoning);
            c.add(moves::BANEFUL_BUNKER);
        }
        abilities::LIQUID_VOICE => {
            if has_type(type1, type2, Type::Water) {
                c.add_where(|mv| mv.is_sound_move && mv.power > 0);
            }
        }
        abilities::TRIAGE => c.add_where(|mv| mv.absorb_percent > 0),
        abilities::SURGE_SURFER => c.add(moves::ELECTRIC_TERRAIN),
        abilities::CORROSION => {
            c.add_where(|mv| mv.category == MoveCategory::Status && is_poisoning(mv))
        }
        _ => {}
    }
    c.finish()
}

const WEATHER_SETUP_ANTI_SYNERGY: &[i32] = &[
    moves::SOLAR_BEAM,
    moves::SOLAR_BLADE,
    moves::MORNING_SUN,
    moves::SYNTHESIS,
    moves::MOONLIGHT,
    moves::RAIN_DANCE,
    moves::SUNNY_DAY,
    moves::HAIL,
    moves::SANDSTORM,
];

const SUN_ANTI_SYNERGY: &[i32] = &[
    moves::THUNDER,
    moves::HURRICANE,
    moves::RAIN_DANCE,
    moves::SUNNY_DAY,
    moves::HAIL,
    moves::SANDSTORM,
];

pub fn hard_ability_move_anti_synergy(
    data: &GameData,
    ability: i32,
    move_list: &[MoveId],
) -> Vec<MoveId> {
    let mut c = Collector::new(data, move_list);
    match ability {
        abilities::PRIMORDIAL_SEA => {
            c.add_where(|mv| damaging_of_type(mv, Type::Fire));
            c.add_all(WEATHER_SETUP_ANTI_SYNERGY);
        }
        abilities::DRIZZLE | abilities::SAND_STREAM | abilities::SNOW_WARNING => {
            c.add_all(WEATHER_SETUP_ANTI_SYNERGY)
        }
        abilities::SPEED_BOOST => {
            c.add_where(|mv| {
                mv.stat_change_move_type == StatChangeMoveType::NoDamageUser
                    && mv.stat_changes[0].kind == StatChangeType::Speed
                    && mv.stat_changes[0].stages > 0
            });
            c.add_all(&[moves::PSYCH_UP, moves::HAZE]);
        }
        abilities::DESOLATE_LAND => {
            c.add_where(|mv| damaging_of_type(mv, Type::Water));
            c.add_all(SUN_ANTI_SYNERGY);
        }
        abilities::DROUGHT => c.add_all(SUN_ANTI_SYNERGY),
        abilities::NO_GUARD => {
            c.add_all(&[moves::LOCK_ON, moves::MIND_READER]);
            c.add_where(|mv| {
                mv.has_specific_stat_change(StatChangeType::Accuracy, true)
                    || mv.has_specific_stat_change(StatChangeType::Evasion, true)
                    || mv.has_specific_stat_change(StatChangeType::Evasion, false)
            });
        }
        abilities::DAMP => c.add_all(&[
            moves::SELF_DESTRUCT,
            moves::EXPLOSION,
            moves::MIND_BLOWN,
            moves::MISTY_EXPLOSION,
        ]),
        abilities::INSOMNIA
        | abilities::VITAL_SPIRIT
        | abilities::COMATOSE
        | abilities::SWEET_VEIL => c.add(moves::REST),
        abilities::AIR_LOCK | abilities::CLOUD_NINE | abilities::DELTA_STREAM => {
            c.add_all(&[
                moves::RAIN_DANCE,
                moves::SUNNY_DAY,
                moves::SANDSTORM,
                moves::HAIL,
            ]);
        }
        abilities::SIMPLE => {
            c.add_where(|mv| raises_or_lowers_user_stats(mv) && mv.stat_changes[0].stages < 0)
        }
        abilities::CONTRARY => {
            c.add_where(|mv| raises_or_lowers_user_stats(mv) && mv.stat_changes[0].stages > 0);
            c.add(moves::SHELL_SMASH);
        }
        abilities::LIGHT_METAL => c.add_all(&[moves::HEAT_CRASH, moves::HEAVY_SLAM]),
        abilities::ELECTRIC_SURGE => {
            c.add_where(|mv| {
                mv.category == MoveCategory::Status && mv.status_type == StatusType::Sleep
            });
            c.add(moves::REST);
        }
        abilities::PSYCHIC_SURGE => c.add_where(|mv| mv.priority > 0),
        abilities::MISTY_SURGE => {
            c.add_where(|mv| {
                mv.category == MoveCategory::Status
                    && matches!(
                        mv.status_type,
                        StatusType::Burn
                            | StatusType::Freeze
                            | StatusType::Paralyze
                            | StatusType::Sleep
                            | StatusType::Poison
                            | StatusType::ToxicPoison
                    )
            });
            c.add(moves::REST);
        }
        abilities::GRASSY_SURGE => {
            c.add_all(&[moves::EARTHQUAKE, moves::MAGNITUDE, moves::BULLDOZE])
        }
        _ => {}
    }
    c.finish()
}

fn hp_share(pk: &Pokemon) -> f64 {
    f64::from(pk.hp) / f64::from(pk.bst())
}

fn speed_share(pk: &Pokemon) -> f64 {
    f64::from(pk.speed) / f64::from(pk.bst())
}

pub fn stat_move_synergy(data: &GameData, pk: &Pokemon, move_list: &[MoveId]) -> Vec<MoveId> {
    let mut c = Collector::new(data, move_list);
    let bst = f64::from(pk.bst());
    if hp_share(pk) < 1.0 / 8.0 {
        c.add_all(&[moves::PAIN_SPLIT, moves::ENDEAVOR]);
    }
    if hp_share(pk) >= 1.0 / 4.0 {
        c.add_all(&[
            moves::WATER_SPOUT,
            moves::ERUPTION,
            moves::COUNTER,
            moves::MIRROR_COAT,
        ]);
    }
    if pk.attack * 2 < pk.defense {
        c.add(moves::POWER_TRICK);
    }
    if f64::from(pk.attack + pk.spatk) / bst < 1.0 / 4.0 {
        c.add(moves::POWER_SPLIT);
    }
    if f64::from(pk.defense + pk.spdef) / bst < 1.0 / 4.0 {
        c.add(moves::GUARD_SPLIT);
    }
    if speed_share(pk) < 1.0 / 8.0 {
        c.add(moves::GYRO_BALL);
    }
    if speed_share(pk) >= 1.0 / 4.0 {
        c.add(moves::ELECTRO_BALL);
    }
    c.finish()
}

pub fn stat_move_anti_synergy(data: &GameData, pk: &Pokemon, move_list: &[MoveId]) -> Vec<MoveId> {
    let mut c = Collector::new(data, move_list);
    let bst = f64::from(pk.bst());
    if hp_share(pk) >= 1.0 / 4.0 {
        c.add_all(&[moves::PAIN_SPLIT, moves::ENDEAVOR]);
    }
    if pk.defense * 2 < pk.attack {
        c.add(moves::POWER_TRICK);
    }
    if f64::from(pk.attack + pk.spatk) / bst >= 1.0 / 3.0 {
        c.add(moves::POWER_SPLIT);
    }
    if f64::from(pk.defense + pk.spdef) / bst >= 1.0 / 3.0 {
        c.add(moves::GUARD_SPLIT);
    }
    if speed_share(pk) >= 1.0 / 4.0 {
        c.add(moves::GYRO_BALL);
    }
    if speed_share(pk) < 1.0 / 8.0 {
        c.add(moves::ELECTRO_BALL);
    }
    c.finish()
}

fn speed_control(mv: &Move) -> bool {
    (mv.stat_change_move_type == StatChangeMoveType::DamageTarget
        && mv.has_specific_stat_change(StatChangeType::Speed, false))
        || (raises_or_lowers_user_stats(mv)
            && mv.has_specific_stat_change(StatChangeType::Speed, true))
}

fn boosts_heavily(mv: &Move) -> bool {
    mv.stat_changes[0].stages >= 2 || mv.stat_changes[1].kind != StatChangeType::None
}

pub fn move_synergy(data: &GameData, first: &Move, move_list: &[MoveId]) -> Vec<MoveId> {
    let mut c = Collector::new(data, move_list);
    if speed_control(first) {
        c.add_where(|mv| mv.flinch_percent_chance > 0.0 && mv.priority == 0);
    }
    if first.flinch_percent_chance > 0.0 && first.priority == 0 {
        c.add_where(speed_control);
    }
    if boosts_heavily(first) {
        c.add_all(&[moves::BATON_PASS, moves::STORED_POWER, moves::POWER_TRIP]);
    }
    if first.status_type == StatusType::Sleep {
        c.add_all(&[moves::DREAM_EATER, moves::NIGHTMARE, moves::HEX]);
    }
    match first.number {
        moves::TOXIC => {
            c.add_all(&[
                moves::PROTECT,
                moves::DETECT,
                moves::KINGS_SHIELD,
                moves::DIG,
                moves::FLY,
                moves::BOUNCE,
                moves::DIVE,
            ]);
            c.add_where(|mv| mv.is_trap_move);
            c.add_all(&[moves::VENOSHOCK, moves::HEX]);
        }
        moves::POISON_POWDER | moves::POISON_GAS | moves::BANEFUL_BUNKER | moves::TOXIC_THREAD => {
            c.add_all(&[moves::VENOSHOCK, moves::HEX]);
        }
        moves::VENOSHOCK => {
            c.add_where(|mv| mv.category == MoveCategory::Status && is_poisoning(mv))
        }
        moves::PROTECT | moves::DETECT | moves::KINGS_SHIELD => {
            c.add_all(&[moves::TOXIC, moves::LEECH_SEED, moves::WILL_OWISP]);
        }
        moves::BATON_PASS => {
            c.add_where(boosts_heavily);
            c.add(moves::SHELL_SMASH);
        }
        moves::WILL_OWISP => c.add(moves::HEX),
        moves::LOCK_ON | moves::MIND_READER => c.add_where(|mv| mv.hitratio <= 50.0),
        moves::SUNNY_DAY => c.add_all(&[moves::SOLAR_BLADE, moves::SOLAR_BEAM]),
        moves::RAIN_DANCE => c.add_all(&[moves::THUNDER, moves::HURRICANE]),
        moves::POWER_SWAP => c.add_where(|mv| {
            mv.stat_change_move_type == StatChangeMoveType::DamageUser
                && (mv.has_specific_stat_change(StatChangeType::Attack, false)
                    || mv.has_specific_stat_change(StatChangeType::SpecialAttack, false))
        }),
        moves::ENDURE => c.add_all(&[moves::REVERSAL, moves::FLAIL, moves::ENDEAVOR]),
        moves::ENDEAVOR => c.add_where(|mv| mv.category != MoveCategory::Status && mv.priority > 0),
        moves::THUNDER_WAVE | moves::GLARE | moves::STUN_SPORE => {
            c.add_where(|mv| mv.flinch_percent_chance > 0.0);
            c.add_where(|mv| {
                mv.category == MoveCategory::Status && mv.status_type == StatusType::Confusion
            });
            c.add(moves::HEX);
        }
        moves::HAIL => c.add_all(&[moves::BLIZZARD, moves::AURORA_VEIL]),
        moves::STOCKPILE => c.add_all(&[moves::SPIT_UP, moves::SWALLOW]),
        moves::SPIT_UP | moves::SWALLOW => c.add(moves::STOCKPILE),
        moves::LEECH_SEED | moves::PERISH_SONG => c.add_where(|mv| mv.is_trap_move),
        moves::SPIKES | moves::STEALTH_ROCK | moves::TOXIC_SPIKES => {
            c.add_all(&[
                moves::ROAR,
                moves::WHIRLWIND,
                moves::DRAGON_TAIL,
                moves::CIRCLE_THROW,
            ]);
        }
        moves::REST => c.add(moves::SLEEP_TALK),
        moves::FOCUS_ENERGY => c.add_where(|mv| mv.critical_chance == CriticalChance::Increased),
        moves::FOCUS_PUNCH | moves::DREAM_EATER | moves::NIGHTMARE => {
            c.add_where(|mv| {
                mv.status_move_type == StatusMoveType::NoDamage
                    && mv.status_type == StatusType::Sleep
            });
        }
        moves::TORMENT => c.add(moves::ENCORE),
        moves::ENCORE => c.add(moves::TORMENT),
        moves::HEX => {
            c.add_where(|mv| {
                mv.status_move_type == StatusMoveType::NoDamage
                    && mv.status_type != StatusType::Confusion
            });
            c.add(moves::BANEFUL_BUNKER);
        }
        moves::STORED_POWER => {
            c.add_where(|mv| {
                mv.stat_changes[0].stages > 1 || mv.stat_changes[1].kind != StatChangeType::None
            });
            c.add_all(&[moves::ACUPRESSURE, moves::SHELL_SMASH]);
        }
        moves::SWAGGER => c.add(moves::PUNISHMENT),
        moves::PUNISHMENT => c.add(moves::SWAGGER),
        moves::SHELL_SMASH => c.add(moves::STORED_POWER),
        _ => {}
    }
    c.finish()
}

pub fn soft_move_synergy(
    data: &GameData,
    first: &Move,
    move_list: &[MoveId],
    generation: i32,
    effectiveness_updated: bool,
) -> Result<Vec<MoveId>, MissingEffectivenessTable> {
    let mut c = Collector::new(data, move_list);
    if first.category != MoveCategory::Status {
        let attacking = first.kind.ok_or(MissingEffectivenessTable)?;
        for resisting in
            Effectiveness::not_very_effective(attacking, generation, effectiveness_updated)
        {
            let super_effective: Vec<Type> =
                Effectiveness::against(resisting, None, generation, effectiveness_updated)
                    .ok_or(MissingEffectivenessTable)?
                    .into_iter()
                    .filter(|(_, effect)| *effect == Effectiveness::Double)
                    .map(|(kind, _)| kind)
                    .collect();
            c.add_where(|mv| {
                mv.category != MoveCategory::Status
                    && mv.kind.is_some_and(|kind| super_effective.contains(&kind))
            });
        }
    }
    match first.number {
        moves::SWORDS_DANCE => c.add_where(|mv| mv.category == MoveCategory::Physical),
        moves::NASTY_PLOT | moves::TAIL_GLOW => {
            c.add_where(|mv| mv.category == MoveCategory::Special)
        }
        _ => {}
    }
    Ok(c.finish())
}

pub fn hard_move_anti_synergy(data: &GameData, first: &Move, move_list: &[MoveId]) -> Vec<MoveId> {
    let mut c = Collector::new(data, move_list);
    if first.category == MoveCategory::Status && first.status_type != StatusType::None {
        c.add_where(|mv| {
            mv.category == MoveCategory::Status
                && mv.status_type != StatusType::None
                && (mv.status_type == first.status_type
                    || (first.status_type != StatusType::Confusion
                        && mv.status_type != StatusType::Confusion))
        });
    }
    match first.number {
        moves::PROTECT => c.add_all(&[moves::DETECT, moves::BANEFUL_BUNKER, moves::KINGS_SHIELD]),
        moves::DETECT => c.add_all(&[moves::PROTECT, moves::BANEFUL_BUNKER, moves::KINGS_SHIELD]),
        moves::KINGS_SHIELD => c.add_all(&[moves::PROTECT, moves::DETECT, moves::BANEFUL_BUNKER]),
        moves::BANEFUL_BUNKER => c.add_all(&[moves::PROTECT, moves::DETECT, moves::KINGS_SHIELD]),
        moves::RETURN_THE_MOVE_NOT_THE_KEYWORD => c.add(moves::FRUSTRATION),
        moves::FRUSTRATION => c.add(moves::RETURN_THE_MOVE_NOT_THE_KEYWORD),
        moves::LEECH_SEED | moves::PERISH_SONG => {
            c.add_all(&[
                moves::WHIRLWIND,
                moves::ROAR,
                moves::CIRCLE_THROW,
                moves::DRAGON_TAIL,
            ]);
        }
        _ => {}
    }
    match first.kind {
        Some(Type::Fire) if first.category != MoveCategory::Status => c.add(moves::WATER_SPORT),
        Some(Type::Electric) if first.category != MoveCategory::Status => c.add(moves::MUD_SPORT),
        _ => {}
    }
    c.finish()
}

pub fn soft_move_anti_synergy(data: &GameData, first: &Move, move_list: &[MoveId]) -> Vec<MoveId> {
    let mut c = Collector::new(data, move_list);
    if first.category != MoveCategory::Status {
        c.add_where(|mv| mv.category != MoveCategory::Status && mv.kind == first.kind);
    }
    match first.number {
        moves::WATER_SPORT => c.add_where(|mv| damaging_of_type(mv, Type::Fire)),
        moves::MUD_SPORT => c.add_where(|mv| damaging_of_type(mv, Type::Electric)),
        _ => {}
    }
    c.finish()
}

pub fn requires_other_move(data: &GameData, first: &Move, move_list: &[MoveId]) -> Vec<MoveId> {
    let mut c = Collector::new(data, move_list);
    match first.number {
        moves::SPIT_UP | moves::SWALLOW => c.add(moves::STOCKPILE),
        moves::DREAM_EATER | moves::NIGHTMARE => {
            c.add_where(|mv| {
                mv.category == MoveCategory::Status && mv.status_type == StatusType::Sleep
            });
        }
        _ => {}
    }
    c.finish()
}
