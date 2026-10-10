use std::collections::BTreeMap;

use crate::constants::{abilities, items, moves, species};
use crate::types::{StatChange, stat};

pub const MIN_DAMAGING_MOVE_POWER: i32 = 50;
pub const HIGHEST_POKEMON_GEN: i32 = 9;
pub const LARGEST_NUMBER_OF_SPLIT_EVOS: i32 = 8;

const BANNED_RANDOM_MOVES: &[i32] = &[moves::STRUGGLE];

const BANNED_FOR_DAMAGING_MOVE: &[i32] = &[
    moves::SELF_DESTRUCT,
    moves::DREAM_EATER,
    moves::EXPLOSION,
    moves::SNORE,
    moves::FALSE_SWIPE,
    moves::FUTURE_SIGHT,
    moves::FAKE_OUT,
    moves::FOCUS_PUNCH,
    moves::DOOM_DESIRE,
    moves::FEINT,
    moves::LAST_RESORT,
    moves::SUCKER_PUNCH,
    moves::CONSTRICT,
    moves::RAGE,
    moves::ROLLOUT,
    moves::ICE_BALL,
    moves::SYNCHRONOISE,
    moves::SHELL_TRAP,
    moves::FOUL_PLAY,
    moves::SPIT_UP,
    moves::SONIC_BOOM,
    moves::DRAGON_RAGE,
    moves::HORN_DRILL,
    moves::GUILLOTINE,
    moves::FISSURE,
    moves::SHEER_COLD,
];

pub fn banned_random_move(number: i32) -> bool {
    BANNED_RANDOM_MOVES.contains(&number)
}

pub fn banned_for_damaging_move(number: i32) -> bool {
    BANNED_FOR_DAMAGING_MOVE.contains(&number)
}

pub const NORMAL_MULTIHIT_MOVES: &[i32] = &[
    moves::ARM_THRUST,
    moves::BARRAGE,
    moves::BONE_RUSH,
    moves::BULLET_SEED,
    moves::COMET_PUNCH,
    moves::DOUBLE_SLAP,
    moves::FURY_ATTACK,
    moves::FURY_SWIPES,
    moves::ICICLE_SPEAR,
    moves::PIN_MISSILE,
    moves::ROCK_BLAST,
    moves::SPIKE_CANNON,
    moves::TAIL_SLAP,
    moves::WATER_SHURIKEN,
];

pub const DOUBLE_HIT_MOVES: &[i32] = &[
    moves::BONEMERANG,
    moves::DOUBLE_HIT,
    moves::DOUBLE_IRON_BASH,
    moves::DOUBLE_KICK,
    moves::DRAGON_DARTS,
    moves::DUAL_CHOP,
    moves::GEAR_GRIND,
    moves::TWINEEDLE,
];

pub const VARYING_POWER_Z_MOVES: &[i32] = &[
    moves::BREAKNECK_BLITZ_PHYSICAL,
    moves::BREAKNECK_BLITZ_SPECIAL,
    moves::ALL_OUT_PUMMELING_PHYSICAL,
    moves::ALL_OUT_PUMMELING_SPECIAL,
    moves::SUPERSONIC_SKYSTRIKE_PHYSICAL,
    moves::SUPERSONIC_SKYSTRIKE_SPECIAL,
    moves::ACID_DOWNPOUR_PHYSICAL,
    moves::ACID_DOWNPOUR_SPECIAL,
    moves::TECTONIC_RAGE_PHYSICAL,
    moves::TECTONIC_RAGE_SPECIAL,
    moves::CONTINENTAL_CRUSH_PHYSICAL,
    moves::CONTINENTAL_CRUSH_SPECIAL,
    moves::SAVAGE_SPIN_OUT_PHYSICAL,
    moves::SAVAGE_SPIN_OUT_SPECIAL,
    moves::NEVER_ENDING_NIGHTMARE_PHYSICAL,
    moves::NEVER_ENDING_NIGHTMARE_SPECIAL,
    moves::CORKSCREW_CRASH_PHYSICAL,
    moves::CORKSCREW_CRASH_SPECIAL,
    moves::INFERNO_OVERDRIVE_PHYSICAL,
    moves::INFERNO_OVERDRIVE_SPECIAL,
    moves::HYDRO_VORTEX_PHYSICAL,
    moves::HYDRO_VORTEX_SPECIAL,
    moves::BLOOM_DOOM_PHYSICAL,
    moves::BLOOM_DOOM_SPECIAL,
    moves::GIGAVOLT_HAVOC_PHYSICAL,
    moves::GIGAVOLT_HAVOC_SPECIAL,
    moves::SHATTERED_PSYCHE_PHYSICAL,
    moves::SHATTERED_PSYCHE_SPECIAL,
    moves::SUBZERO_SLAMMER_PHYSICAL,
    moves::SUBZERO_SLAMMER_SPECIAL,
    moves::DEVASTATING_DRAKE_PHYSICAL,
    moves::DEVASTATING_DRAKE_SPECIAL,
    moves::BLACK_HOLE_ECLIPSE_PHYSICAL,
    moves::BLACK_HOLE_ECLIPSE_SPECIAL,
    moves::TWINKLE_TACKLE_PHYSICAL,
    moves::TWINKLE_TACKLE_SPECIAL,
];

pub const FIXED_POWER_Z_MOVES: &[i32] = &[
    moves::CATASTROPIKA,
    moves::SINISTER_ARROW_RAID,
    moves::MALICIOUS_MOONSAULT,
    moves::OCEANIC_OPERETTA,
    moves::GUARDIAN_OF_ALOLA,
    moves::SOUL_STEALING7_STAR_STRIKE,
    moves::STOKED_SPARKSURFER,
    moves::PULVERIZING_PANCAKE,
    moves::EXTREME_EVOBOOST,
    moves::GENESIS_SUPERNOVA,
    moves::TEN_MILLION_VOLT_THUNDERBOLT,
    moves::LIGHT_THAT_BURNS_THE_SKY,
    moves::SEARING_SUNRAZE_SMASH,
    moves::MENACING_MOONRAZE_MAELSTROM,
    moves::LETS_SNUGGLE_FOREVER,
    moves::SPLINTERED_STORMSHARDS,
    moves::CLANGOROUS_SOULBLAZE,
];

pub fn z_moves() -> Vec<i32> {
    FIXED_POWER_Z_MOVES
        .iter()
        .chain(VARYING_POWER_Z_MOVES)
        .copied()
        .collect()
}

pub fn is_z_move(number: i32) -> bool {
    FIXED_POWER_Z_MOVES.contains(&number) || VARYING_POWER_Z_MOVES.contains(&number)
}

fn change(map: &mut BTreeMap<i32, StatChange>, species: i32, stats: i32, values: &[i32]) {
    map.insert(species, StatChange::new(stats, values));
}

pub fn stat_changes(generation: i32) -> BTreeMap<i32, StatChange> {
    let mut map = BTreeMap::new();
    let m = &mut map;
    match generation {
        6 => {
            change(m, species::BUTTERFREE, stat::SPATK, &[90]);
            change(m, species::BEEDRILL, stat::ATK, &[90]);
            change(m, species::PIDGEOT, stat::SPEED, &[101]);
            change(m, species::PIKACHU, stat::DEF | stat::SPDEF, &[40, 50]);
            change(m, species::RAICHU, stat::SPEED, &[110]);
            change(m, species::NIDOQUEEN, stat::ATK, &[92]);
            change(m, species::NIDOKING, stat::ATK, &[102]);
            change(m, species::CLEFABLE, stat::SPATK, &[95]);
            change(m, species::WIGGLYTUFF, stat::SPATK, &[85]);
            change(m, species::VILEPLUME, stat::SPATK, &[110]);
            change(m, species::POLIWRATH, stat::ATK, &[95]);
            change(m, species::ALAKAZAM, stat::SPDEF, &[95]);
            change(m, species::VICTREEBEL, stat::SPDEF, &[70]);
            change(m, species::GOLEM, stat::ATK, &[120]);
            change(m, species::AMPHAROS, stat::DEF, &[85]);
            change(m, species::BELLOSSOM, stat::DEF, &[95]);
            change(m, species::AZUMARILL, stat::SPATK, &[60]);
            change(m, species::JUMPLUFF, stat::SPDEF, &[95]);
            change(m, species::BEAUTIFLY, stat::SPATK, &[100]);
            change(m, species::EXPLOUD, stat::SPDEF, &[73]);
            change(m, species::STARAPTOR, stat::SPDEF, &[60]);
            change(m, species::ROSERADE, stat::DEF, &[65]);
            change(m, species::STOUTLAND, stat::ATK, &[110]);
            change(m, species::UNFEZANT, stat::ATK, &[115]);
            change(m, species::GIGALITH, stat::SPDEF, &[80]);
            change(m, species::SEISMITOAD, stat::ATK, &[95]);
            change(m, species::LEAVANNY, stat::SPDEF, &[80]);
            change(m, species::SCOLIPEDE, stat::ATK, &[100]);
            change(m, species::KROOKODILE, stat::DEF, &[80]);
        }
        7 => {
            change(m, species::ARBOK, stat::ATK, &[95]);
            change(m, species::DUGTRIO, stat::ATK, &[100]);
            change(m, species::FARFETCHD, stat::ATK, &[90]);
            change(m, species::DODRIO, stat::SPEED, &[110]);
            change(m, species::ELECTRODE, stat::SPEED, &[150]);
            change(m, species::EXEGGUTOR, stat::SPDEF, &[75]);
            change(m, species::NOCTOWL, stat::SPATK, &[86]);
            change(m, species::ARIADOS, stat::SPDEF, &[70]);
            change(m, species::QWILFISH, stat::DEF, &[85]);
            change(m, species::MAGCARGO, stat::HP | stat::SPATK, &[60, 90]);
            change(
                m,
                species::CORSOLA,
                stat::HP | stat::DEF | stat::SPDEF,
                &[65, 95, 95],
            );
            change(m, species::MANTINE, stat::HP, &[85]);
            change(m, species::SWELLOW, stat::SPATK, &[75]);
            change(m, species::PELIPPER, stat::SPATK, &[95]);
            change(
                m,
                species::MASQUERAIN,
                stat::SPATK | stat::SPEED,
                &[100, 80],
            );
            change(m, species::DELCATTY, stat::SPEED, &[90]);
            change(m, species::VOLBEAT, stat::DEF | stat::SPDEF, &[75, 85]);
            change(m, species::ILLUMISE, stat::DEF | stat::SPDEF, &[75, 85]);
            change(m, species::LUNATONE, stat::HP, &[90]);
            change(m, species::SOLROCK, stat::HP, &[90]);
            change(
                m,
                species::CHIMECHO,
                stat::HP | stat::DEF | stat::SPDEF,
                &[75, 80, 90],
            );
            change(m, species::WOOBAT, stat::HP, &[65]);
            change(m, species::CRUSTLE, stat::ATK, &[105]);
            change(m, species::BEARTIC, stat::ATK, &[130]);
            change(m, species::CRYOGONAL, stat::HP | stat::DEF, &[80, 50]);
        }
        8 => {
            change(m, species::AEGISLASH, stat::DEF | stat::SPDEF, &[140, 140]);
        }
        9 => {
            change(m, species::CRESSELIA, stat::DEF | stat::SPDEF, &[110, 120]);
            change(m, species::ZACIAN, stat::ATK, &[120]);
            change(m, species::ZAMAZENTA, stat::ATK, &[120]);
        }
        _ => {}
    }
    map
}

pub const X_ITEMS: &[i32] = &[
    items::GUARD_SPEC,
    items::DIRE_HIT,
    items::X_ATTACK,
    items::X_DEFENSE,
    items::X_SPEED,
    items::X_ACCURACY,
    items::X_SP_ATK,
    items::X_SP_DEF,
];

pub const BATTLE_TRAPPING_ABILITIES: &[i32] = &[
    abilities::SHADOW_TAG,
    abilities::MAGNET_PULL,
    abilities::ARENA_TRAP,
];

pub const NEGATIVE_ABILITIES: &[i32] = &[
    abilities::DEFEATIST,
    abilities::SLOW_START,
    abilities::TRUANT,
    abilities::KLUTZ,
    abilities::STALL,
];

pub const BAD_ABILITIES: &[i32] = &[
    abilities::MINUS,
    abilities::PLUS,
    abilities::ANTICIPATION,
    abilities::FOREWARN,
    abilities::FRISK,
    abilities::HONEY_GATHER,
    abilities::AURA_BREAK,
    abilities::RECEIVER,
    abilities::POWER_OF_ALCHEMY,
];

pub const DOUBLE_BATTLE_ABILITIES: &[i32] = &[
    abilities::FRIEND_GUARD,
    abilities::HEALER,
    abilities::TELEPATHY,
    abilities::SYMBIOSIS,
    abilities::BATTERY,
];

pub const DUPLICATE_ABILITIES: &[i32] = &[
    abilities::VITAL_SPIRIT,
    abilities::WHITE_SMOKE,
    abilities::PURE_POWER,
    abilities::SHELL_ARMOR,
    abilities::AIR_LOCK,
    abilities::SOLID_ROCK,
    abilities::IRON_BARBS,
    abilities::TURBOBLAZE,
    abilities::TERAVOLT,
    abilities::EMERGENCY_EXIT,
    abilities::DAZZLING,
    abilities::TANGLING_HAIR,
    abilities::POWER_OF_ALCHEMY,
    abilities::FULL_METAL_BODY,
    abilities::SHADOW_SHIELD,
    abilities::PRISM_ARMOR,
    abilities::LIBERO,
    abilities::STALWART,
];

pub const NO_POWER_NON_STATUS_MOVES: &[i32] = &[
    moves::GUILLOTINE,
    moves::HORN_DRILL,
    moves::SONIC_BOOM,
    moves::LOW_KICK,
    moves::COUNTER,
    moves::SEISMIC_TOSS,
    moves::DRAGON_RAGE,
    moves::FISSURE,
    moves::NIGHT_SHADE,
    moves::BIDE,
    moves::PSYWAVE,
    moves::SUPER_FANG,
    moves::FLAIL,
    moves::REVENGE,
    moves::RETURN_THE_MOVE_NOT_THE_KEYWORD,
    moves::PRESENT,
    moves::FRUSTRATION,
    moves::MAGNITUDE,
    moves::MIRROR_COAT,
    moves::BEAT_UP,
    moves::SPIT_UP,
    moves::SHEER_COLD,
];

pub const CANNOT_BE_OBSOLETED_MOVES: &[i32] = &[
    moves::RETURN_THE_MOVE_NOT_THE_KEYWORD,
    moves::FRUSTRATION,
    moves::ENDEAVOR,
    moves::FLAIL,
    moves::REVERSAL,
    moves::HIDDEN_POWER,
    moves::STORED_POWER,
    moves::SMELLING_SALTS,
    moves::FLING,
    moves::POWER_TRIP,
    moves::COUNTER,
    moves::MIRROR_COAT,
    moves::SUPER_FANG,
];

pub const CANNOT_OBSOLETE_MOVES: &[i32] = &[
    moves::GEAR_UP,
    moves::MAGNETIC_FLUX,
    moves::FOCUS_PUNCH,
    moves::EXPLOSION,
    moves::SELF_DESTRUCT,
    moves::GEOMANCY,
    moves::VENOM_DRENCH,
];

pub const DOUBLE_BATTLE_MOVES: &[i32] = &[
    moves::FOLLOW_ME,
    moves::HELPING_HAND,
    moves::RAGE_POWDER,
    moves::AFTER_YOU,
    moves::ALLY_SWITCH,
    moves::HEAL_PULSE,
    moves::QUASH,
    moves::ION_DELUGE,
    moves::MAT_BLOCK,
    moves::AROMATIC_MIST,
    moves::ELECTRIFY,
    moves::INSTRUCT,
    moves::SPOTLIGHT,
    moves::DECORATE,
    moves::LIFE_DEW,
    moves::COACHING,
];

pub const USELESS_MOVES: &[i32] = &[
    moves::SPLASH,
    moves::CELEBRATE,
    moves::HOLD_HANDS,
    moves::TELEPORT,
    moves::REFLECT_TYPE,
];

pub const REQUIRES_OTHER_MOVE: &[i32] = &[
    moves::SPIT_UP,
    moves::SWALLOW,
    moves::DREAM_EATER,
    moves::NIGHTMARE,
];
