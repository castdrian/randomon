use randomon_data::constants::moves as m;
use randomon_data::{MoveCategory, MoveId, Type};
use randomon_java::JavaRng;
use randomon_java::math::round_double;
use randomon_settings::Settings;

use crate::engine::types::random_type;
use crate::handler::RomHandler;

fn loaded_moves<H: RomHandler + ?Sized>(handler: &H) -> Vec<MoveId> {
    handler
        .core()
        .data
        .move_ids()
        .into_iter()
        .flatten()
        .collect()
}

pub fn randomize_move_powers<H: RomHandler + ?Sized>(handler: &mut H) {
    let mut random = handler.core().engine.random.clone();
    for id in loaded_moves(handler) {
        let mv = handler.core_mut().data.mv_mut(id);
        if mv.internal_id == m::STRUGGLE || mv.power < 10 {
            continue;
        }
        mv.power = if random.next_int_bounded(3) != 2 {
            random.next_int_bounded(11) * 5 + 50
        } else {
            random.next_int_bounded(27) * 5 + 20
        };
        for _ in 0..2 {
            if random.next_int_bounded(100) == 0 {
                mv.power += 50;
            }
        }
        if mv.hit_count != 1.0 {
            mv.power = (round_double(f64::from(mv.power) / mv.hit_count / 5.0) * 5) as i32;
            if mv.power == 0 {
                mv.power = 5;
            }
        }
    }
}

pub fn randomize_move_pps<H: RomHandler + ?Sized>(handler: &mut H) {
    let mut random = handler.core().engine.random.clone();
    for id in loaded_moves(handler) {
        let mv = handler.core_mut().data.mv_mut(id);
        if mv.internal_id == m::STRUGGLE {
            continue;
        }
        mv.pp = if random.next_int_bounded(3) != 2 {
            random.next_int_bounded(3) * 5 + 15
        } else {
            random.next_int_bounded(8) * 5 + 5
        };
    }
}

fn count_down_accuracy(random: &mut impl JavaRng, stop_threshold: i32) -> f64 {
    let mut accuracy = 100.0;
    while accuracy > 20.0 {
        if random.next_int_bounded(10) < stop_threshold {
            break;
        }
        accuracy -= 5.0;
    }
    accuracy
}

pub fn randomize_move_accuracies<H: RomHandler + ?Sized>(handler: &mut H) {
    let mut random = handler.core().engine.random.clone();
    for id in loaded_moves(handler) {
        let mv = handler.core_mut().data.mv_mut(id);
        if mv.internal_id == m::STRUGGLE || mv.hitratio < 5.0 {
            continue;
        }
        if mv.hitratio <= 50.0 {
            mv.hitratio = f64::from(random.next_int_bounded(7) * 5 + 20);
            if random.next_int_bounded(10) == 0 {
                mv.hitratio = (mv.hitratio * 3.0 / 2.0) / 5.0 * 5.0;
            }
        } else if mv.hitratio < 90.0 {
            mv.hitratio = count_down_accuracy(&mut random, 2);
        } else {
            mv.hitratio = count_down_accuracy(&mut random, 4);
        }
    }
}

pub fn randomize_move_types<H: RomHandler + ?Sized>(handler: &mut H) {
    for id in loaded_moves(handler) {
        let mv = handler.core().data.mv(id);
        if mv.internal_id == m::STRUGGLE || mv.kind.is_none() {
            continue;
        }
        let kind = random_type(handler);
        handler.core_mut().data.mv_mut(id).kind = Some(kind);
    }
}

pub fn randomize_move_category<H: RomHandler + ?Sized>(handler: &mut H) {
    if !handler.has_physical_special_split() {
        return;
    }
    let mut random = handler.core().engine.random.clone();
    for id in loaded_moves(handler) {
        let mv = handler.core_mut().data.mv_mut(id);
        if mv.internal_id == m::STRUGGLE || mv.category == MoveCategory::Status {
            continue;
        }
        if random.next_int_bounded(2) == 0 {
            mv.category = if mv.category == MoveCategory::Physical {
                MoveCategory::Special
            } else {
                MoveCategory::Physical
            };
        }
    }
}

struct MoveUpdater<'a, H: RomHandler + ?Sized> {
    handler: &'a mut H,
}

impl<H: RomHandler + ?Sized> MoveUpdater<'_, H> {
    fn record(&mut self, number: i32, kind: usize) {
        if let Some(updates) = self.handler.core_mut().engine.move_updates.as_mut() {
            updates.entry(number).or_insert([false; 5])[kind] = true;
        }
    }

    fn with_move(
        &mut self,
        number: i32,
        change: impl FnOnce(&mut randomon_data::Move) -> bool,
        kind: usize,
    ) {
        let Some(id) = self
            .handler
            .core()
            .data
            .move_by_number(number)
            .map(|mv| MoveId::from_index(mv.number as usize))
        else {
            return;
        };
        if change(self.handler.core_mut().data.mv_mut(id)) {
            self.record(number, kind);
        }
    }

    fn power(&mut self, number: i32, power: i32) {
        self.with_move(
            number,
            |mv| {
                let changed = mv.power != power;
                mv.power = power;
                changed
            },
            0,
        );
    }

    fn pp(&mut self, number: i32, pp: i32) {
        self.with_move(
            number,
            |mv| {
                let changed = mv.pp != pp;
                mv.pp = pp;
                changed
            },
            1,
        );
    }

    fn accuracy(&mut self, number: i32, accuracy: i32) {
        self.with_move(
            number,
            |mv| {
                if (mv.hitratio - f64::from(accuracy)).abs() >= 1.0 {
                    mv.hitratio = f64::from(accuracy);
                    true
                } else {
                    false
                }
            },
            2,
        );
    }

    fn kind(&mut self, number: i32, kind: Type) {
        self.with_move(
            number,
            |mv| {
                let changed = mv.kind != Some(kind);
                mv.kind = Some(kind);
                changed
            },
            3,
        );
    }

    fn category(&mut self, number: i32, category: MoveCategory) {
        self.with_move(
            number,
            |mv| {
                let changed = mv.category != category;
                mv.category = category;
                changed
            },
            4,
        );
    }
}

pub fn update_moves<H: RomHandler + ?Sized>(handler: &mut H, settings: &Settings) {
    let generation = settings.update_moves_to_generation;
    let own = handler.generation_of_pokemon();
    let perfect_accuracy = handler.core().engine.perfect_accuracy;
    let mut u = MoveUpdater { handler };
    if generation >= 2 && own < 2 {
        u.kind(m::KARATE_CHOP, Type::Fighting);
        u.kind(m::GUST, Type::Flying);
        u.power(m::WING_ATTACK, 60);
        u.accuracy(m::WHIRLWIND, 100);
        u.kind(m::SAND_ATTACK, Type::Ground);
        u.power(m::DOUBLE_EDGE, 120);
        u.accuracy(m::BLIZZARD, 70);
        u.accuracy(m::ROCK_THROW, 90);
        u.accuracy(m::HYPNOSIS, 60);
        u.power(m::SELF_DESTRUCT, 200);
        u.power(m::EXPLOSION, 250);
        u.power(m::DIG, 60);
    }
    if generation >= 3 && own < 3 {
        u.accuracy(m::RAZOR_WIND, 100);
        u.accuracy(m::LOW_KICK, 100);
    }
    if generation >= 4 && own < 4 {
        u.power(m::FLY, 90);
        u.pp(m::VINE_WHIP, 15);
        u.pp(m::ABSORB, 25);
        u.pp(m::MEGA_DRAIN, 15);
        u.power(m::DIG, 80);
        u.pp(m::RECOVER, 10);
        u.accuracy(m::FLASH, 100);
        u.power(m::PETAL_DANCE, 90);
        u.accuracy(m::DISABLE, 80);
        u.power(m::JUMP_KICK, 85);
        u.power(m::HIGH_JUMP_KICK, 100);
        if own >= 2 {
            u.power(m::ZAP_CANNON, 120);
            u.power(m::OUTRAGE, 120);
            u.pp(m::OUTRAGE, 10);
            u.pp(m::GIGA_DRAIN, 10);
            u.power(m::ROCK_SMASH, 40);
        }
        if own == 3 {
            u.pp(m::STOCKPILE, 20);
            u.power(m::DIVE, 80);
            u.power(m::LEAF_BLADE, 90);
        }
    }
    if generation >= 5 && own < 5 {
        u.accuracy(m::BIND, 85);
        u.pp(m::JUMP_KICK, 10);
        u.power(m::JUMP_KICK, 100);
        u.power(m::TACKLE, 50);
        u.accuracy(m::TACKLE, 100);
        u.accuracy(m::WRAP, 90);
        u.pp(m::THRASH, 10);
        u.power(m::THRASH, 120);
        u.accuracy(m::DISABLE, 100);
        u.pp(m::PETAL_DANCE, 10);
        u.power(m::PETAL_DANCE, 120);
        u.accuracy(m::FIRE_SPIN, 85);
        u.power(m::FIRE_SPIN, 35);
        u.accuracy(m::TOXIC, 90);
        u.accuracy(m::CLAMP, 85);
        u.pp(m::CLAMP, 15);
        u.pp(m::HIGH_JUMP_KICK, 10);
        u.power(m::HIGH_JUMP_KICK, 130);
        u.accuracy(m::GLARE, 90);
        u.accuracy(m::POISON_GAS, 80);
        u.accuracy(m::CRABHAMMER, 90);
        if own >= 2 {
            u.kind(m::CURSE, Type::Ghost);
            u.accuracy(m::COTTON_SPORE, 100);
            u.accuracy(m::SCARY_FACE, 100);
            u.accuracy(m::BONE_RUSH, 90);
            u.power(m::GIGA_DRAIN, 75);
            u.power(m::FURY_CUTTER, 20);
            u.pp(m::FUTURE_SIGHT, 10);
            u.power(m::FUTURE_SIGHT, 100);
            u.accuracy(m::FUTURE_SIGHT, 100);
            u.power(m::WHIRLPOOL, 35);
            u.accuracy(m::WHIRLPOOL, 85);
        }
        if own >= 3 {
            u.power(m::UPROAR, 90);
            u.power(m::SAND_TOMB, 35);
            u.accuracy(m::SAND_TOMB, 85);
            u.power(m::BULLET_SEED, 25);
            u.power(m::ICICLE_SPEAR, 25);
            u.power(m::COVET, 60);
            u.accuracy(m::ROCK_BLAST, 90);
            u.power(m::DOOM_DESIRE, 140);
            u.accuracy(m::DOOM_DESIRE, 100);
        }
        if own == 4 {
            u.power(m::FEINT, 30);
            u.power(m::LAST_RESORT, 140);
            u.pp(m::DRAIN_PUNCH, 10);
            u.power(m::DRAIN_PUNCH, 75);
            u.accuracy(m::MAGMA_STORM, 75);
        }
    }
    if generation >= 6 && own < 6 {
        u.pp(m::SWORDS_DANCE, 20);
        u.accuracy(m::WHIRLWIND, perfect_accuracy);
        u.pp(m::VINE_WHIP, 25);
        u.power(m::VINE_WHIP, 45);
        u.power(m::PIN_MISSILE, 25);
        u.accuracy(m::PIN_MISSILE, 95);
        u.power(m::FLAMETHROWER, 90);
        u.power(m::HYDRO_PUMP, 110);
        u.power(m::SURF, 90);
        u.power(m::ICE_BEAM, 90);
        u.power(m::BLIZZARD, 110);
        u.pp(m::GROWTH, 20);
        u.power(m::THUNDERBOLT, 90);
        u.power(m::THUNDER, 110);
        u.pp(m::MINIMIZE, 10);
        u.pp(m::BARRIER, 20);
        u.power(m::LICK, 30);
        u.power(m::SMOG, 30);
        u.power(m::FIRE_BLAST, 110);
        u.pp(m::SKULL_BASH, 10);
        u.power(m::SKULL_BASH, 130);
        u.accuracy(m::GLARE, 100);
        u.accuracy(m::POISON_GAS, 90);
        u.power(m::BUBBLE, 40);
        u.accuracy(m::PSYWAVE, 100);
        u.pp(m::ACID_ARMOR, 20);
        u.power(m::CRABHAMMER, 100);
        if own >= 2 {
            u.pp(m::THIEF, 25);
            u.power(m::THIEF, 60);
            u.power(m::SNORE, 50);
            u.power(m::FURY_CUTTER, 40);
            u.power(m::FUTURE_SIGHT, 120);
        }
        if own >= 3 {
            u.power(m::HEAT_WAVE, 95);
            u.accuracy(m::WILL_OWISP, 85);
            u.power(m::SMELLING_SALTS, 70);
            u.power(m::KNOCK_OFF, 65);
            u.power(m::METEOR_MASH, 90);
            u.accuracy(m::METEOR_MASH, 90);
            u.power(m::AIR_CUTTER, 60);
            u.power(m::OVERHEAT, 130);
            u.pp(m::ROCK_TOMB, 15);
            u.power(m::ROCK_TOMB, 60);
            u.accuracy(m::ROCK_TOMB, 95);
            u.pp(m::EXTRASENSORY, 20);
            u.power(m::MUDDY_WATER, 90);
            u.pp(m::COVET, 25);
        }
        if own >= 4 {
            u.power(m::WAKE_UP_SLAP, 70);
            u.pp(m::TAILWIND, 15);
            u.power(m::ASSURANCE, 60);
            u.accuracy(m::PSYCHO_SHIFT, 100);
            u.power(m::AURA_SPHERE, 80);
            u.pp(m::AIR_SLASH, 15);
            u.power(m::DRAGON_PULSE, 85);
            u.power(m::POWER_GEM, 80);
            u.power(m::ENERGY_BALL, 90);
            u.power(m::DRACO_METEOR, 130);
            u.power(m::LEAF_STORM, 130);
            u.accuracy(m::GUNK_SHOT, 80);
            u.power(m::CHATTER, 65);
            u.power(m::MAGMA_STORM, 100);
        }
        if own == 5 {
            u.power(m::SYNCHRONOISE, 120);
            u.power(m::LOW_SWEEP, 65);
            u.power(m::HEX, 65);
            u.power(m::INCINERATE, 60);
            u.power(m::WATER_PLEDGE, 80);
            u.power(m::FIRE_PLEDGE, 80);
            u.power(m::GRASS_PLEDGE, 80);
            u.power(m::STRUGGLE_BUG, 50);
            u.power(m::FROST_BREATH, 45);
            u.power(m::STORM_THROW, 45);
            u.pp(m::SACRED_SWORD, 15);
            u.power(m::HURRICANE, 110);
            u.power(m::TECHNO_BLAST, 120);
        }
    }
    if generation >= 7 && own < 7 {
        u.power(m::LEECH_LIFE, 80);
        u.pp(m::LEECH_LIFE, 10);
        u.pp(m::SUBMISSION, 20);
        u.power(m::TACKLE, 40);
        u.accuracy(m::THUNDER_WAVE, 90);
        if own >= 2 {
            u.accuracy(m::SWAGGER, 85);
        }
        if own >= 3 {
            u.pp(m::KNOCK_OFF, 20);
        }
        if own >= 4 {
            u.accuracy(m::DARK_VOID, 50);
            u.power(m::SUCKER_PUNCH, 70);
        }
        if own == 6 {
            u.accuracy(m::AROMATIC_MIST, perfect_accuracy);
            u.power(m::FELL_STINGER, 50);
            u.power(m::FLYING_PRESS, 100);
            u.pp(m::MAT_BLOCK, 10);
            u.power(m::MYSTICAL_FIRE, 75);
            u.power(m::PARABOLIC_CHARGE, 65);
            u.accuracy(m::TOPSY_TURVY, perfect_accuracy);
            u.category(m::WATER_SHURIKEN, MoveCategory::Special);
        }
    }
    if generation >= 8 && own < 8 {
        if own >= 2 {
            u.power(m::RAPID_SPIN, 50);
        }
        if own == 7 {
            u.power(m::MULTI_ATTACK, 120);
        }
    }
    if generation >= 9 && own < 9 {
        u.pp(m::RECOVER, 5);
        u.pp(m::SOFT_BOILED, 5);
        u.pp(m::REST, 5);
        if own >= 2 {
            u.pp(m::MILK_DRINK, 5);
        }
        if own >= 3 {
            u.power(m::LUSTER_PURGE, 95);
            u.power(m::MIST_BALL, 95);
            u.pp(m::SLACK_OFF, 5);
        }
        if own >= 4 {
            u.pp(m::ROOST, 5);
        }
        if own >= 7 {
            u.pp(m::SHORE_UP, 5);
        }
        if own >= 8 {
            u.power(m::GRASSY_GLIDE, 55);
            u.power(m::WICKED_BLOW, 75);
            u.power(m::GLACIAL_LANCE, 120);
        }
    }
}
