use randomon_java::math::round_double;
use randomon_java::{JavaRng, shuffle};
use randomon_settings::ExpCurve;

use crate::constants::species;
use crate::ids::{EvolutionListId, MegaEvolutionId, PokemonId};
use crate::types::Type;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StatLayout {
    Gen1,
    Standard,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BaseStats {
    pub layout: StatLayout,
    pub number: i32,
    pub hp: i32,
    pub attack: i32,
    pub defense: i32,
    pub spatk: i32,
    pub spdef: i32,
    pub speed: i32,
    pub special: i32,
}

impl BaseStats {
    pub fn bst(&self) -> i32 {
        match self.layout {
            StatLayout::Gen1 => self.hp + self.attack + self.defense + self.special + self.speed,
            StatLayout::Standard => {
                self.hp + self.attack + self.defense + self.spatk + self.spdef + self.speed
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct Pokemon {
    pub layout: StatLayout,
    pub name: String,
    pub number: i32,
    pub forme_suffix: String,
    pub base_forme: Option<PokemonId>,
    pub forme_number: i32,
    pub cosmetic_forms: i32,
    pub forme_sprite_index: i32,
    pub actually_cosmetic: bool,
    pub real_cosmetic_form_numbers: Vec<i32>,
    pub primary_type: Option<Type>,
    pub secondary_type: Option<Type>,
    pub hp: i32,
    pub attack: i32,
    pub defense: i32,
    pub spatk: i32,
    pub spdef: i32,
    pub speed: i32,
    pub special: i32,
    pub ability1: i32,
    pub ability2: i32,
    pub ability3: i32,
    pub catch_rate: i32,
    pub exp_yield: i32,
    pub guaranteed_held_item: i32,
    pub common_held_item: i32,
    pub rare_held_item: i32,
    pub dark_grass_held_item: i32,
    pub gender_ratio: i32,
    pub front_sprite_pointer: i32,
    pub pic_dimensions: i32,
    pub call_rate: i32,
    pub growth_curve: Option<ExpCurve>,
    pub evolutions_from: EvolutionListId,
    pub evolutions_to: EvolutionListId,
    pub mega_evolutions_from: Vec<MegaEvolutionId>,
    pub mega_evolutions_to: Vec<MegaEvolutionId>,
    pub shuffled_stats_order: Vec<usize>,
    pub temporary_flag: bool,
}

const LEGENDARIES: &[i32] = &[
    species::ARTICUNO,
    species::ZAPDOS,
    species::MOLTRES,
    species::MEWTWO,
    species::MEW,
    species::RAIKOU,
    species::ENTEI,
    species::SUICUNE,
    species::LUGIA,
    species::HO_OH,
    species::CELEBI,
    species::REGIROCK,
    species::REGICE,
    species::REGISTEEL,
    species::LATIAS,
    species::LATIOS,
    species::KYOGRE,
    species::GROUDON,
    species::RAYQUAZA,
    species::JIRACHI,
    species::DEOXYS,
    species::UXIE,
    species::MESPRIT,
    species::AZELF,
    species::DIALGA,
    species::PALKIA,
    species::HEATRAN,
    species::REGIGIGAS,
    species::GIRATINA,
    species::CRESSELIA,
    species::PHIONE,
    species::MANAPHY,
    species::DARKRAI,
    species::SHAYMIN,
    species::ARCEUS,
    species::VICTINI,
    species::COBALION,
    species::TERRAKION,
    species::VIRIZION,
    species::TORNADUS,
    species::THUNDURUS,
    species::RESHIRAM,
    species::ZEKROM,
    species::LANDORUS,
    species::KYUREM,
    species::KELDEO,
    species::MELOETTA,
    species::GENESECT,
    species::XERNEAS,
    species::YVELTAL,
    species::ZYGARDE,
    species::DIANCIE,
    species::HOOPA,
    species::VOLCANION,
    species::TYPE_NULL,
    species::SILVALLY,
    species::TAPU_KOKO,
    species::TAPU_LELE,
    species::TAPU_BULU,
    species::TAPU_FINI,
    species::COSMOG,
    species::COSMOEM,
    species::SOLGALEO,
    species::LUNALA,
    species::NECROZMA,
    species::MAGEARNA,
    species::MARSHADOW,
    species::ZERAORA,
];

const STRONG_LEGENDARIES: &[i32] = &[
    species::MEWTWO,
    species::LUGIA,
    species::HO_OH,
    species::KYOGRE,
    species::GROUDON,
    species::RAYQUAZA,
    species::DIALGA,
    species::PALKIA,
    species::REGIGIGAS,
    species::GIRATINA,
    species::ARCEUS,
    species::RESHIRAM,
    species::ZEKROM,
    species::KYUREM,
    species::XERNEAS,
    species::YVELTAL,
    species::COSMOG,
    species::COSMOEM,
    species::SOLGALEO,
    species::LUNALA,
];

const ULTRA_BEASTS: &[i32] = &[
    species::NIHILEGO,
    species::BUZZWOLE,
    species::PHEROMOSA,
    species::XURKITREE,
    species::CELESTEELA,
    species::KARTANA,
    species::GUZZLORD,
    species::POIPOLE,
    species::NAGANADEL,
    species::STAKATAKA,
    species::BLACEPHALON,
];

pub fn is_legendary_number(number: i32) -> bool {
    LEGENDARIES.contains(&number)
}

pub fn is_strong_legendary_number(number: i32) -> bool {
    STRONG_LEGENDARIES.contains(&number)
}

pub fn is_ultra_beast_number(number: i32) -> bool {
    ULTRA_BEASTS.contains(&number)
}

fn clamp_long_to_stat(value: i64) -> i32 {
    value.clamp(1, 255) as i32
}

fn java_double_max(first: f64, second: f64) -> f64 {
    if first.is_nan() || second.is_nan() {
        f64::NAN
    } else {
        first.max(second)
    }
}

fn java_double_min(first: f64, second: f64) -> f64 {
    if first.is_nan() || second.is_nan() {
        f64::NAN
    } else {
        first.min(second)
    }
}

fn clamp_double_to_stat(value: f64) -> i32 {
    java_double_min(255.0, java_double_max(1.0, value)) as i32
}

fn weighted_share(weight: f64, total_weight: f64, budget: i32) -> i32 {
    round_double(weight / total_weight * f64::from(budget)).max(1) as i32
}

fn rounded_difference(weight: f64, total_weight: f64, difference: f64) -> f64 {
    round_double((weight / total_weight) * difference) as f64
}

impl Pokemon {
    pub fn new(
        layout: StatLayout,
        number: i32,
        evolutions_from: EvolutionListId,
        evolutions_to: EvolutionListId,
    ) -> Self {
        let stat_count = match layout {
            StatLayout::Gen1 => 5,
            StatLayout::Standard => 6,
        };
        Self {
            layout,
            name: String::new(),
            number,
            forme_suffix: String::new(),
            base_forme: None,
            forme_number: 0,
            cosmetic_forms: 0,
            forme_sprite_index: 0,
            actually_cosmetic: false,
            real_cosmetic_form_numbers: Vec::new(),
            primary_type: None,
            secondary_type: None,
            hp: 0,
            attack: 0,
            defense: 0,
            spatk: 0,
            spdef: 0,
            speed: 0,
            special: 0,
            ability1: 0,
            ability2: 0,
            ability3: 0,
            catch_rate: 0,
            exp_yield: 0,
            guaranteed_held_item: 0,
            common_held_item: 0,
            rare_held_item: 0,
            dark_grass_held_item: 0,
            gender_ratio: 0,
            front_sprite_pointer: 0,
            pic_dimensions: 0,
            call_rate: 0,
            growth_curve: None,
            evolutions_from,
            evolutions_to,
            mega_evolutions_from: Vec::new(),
            mega_evolutions_to: Vec::new(),
            shuffled_stats_order: (0..stat_count).collect(),
            temporary_flag: false,
        }
    }

    pub fn stats(&self) -> BaseStats {
        BaseStats {
            layout: self.layout,
            number: self.number,
            hp: self.hp,
            attack: self.attack,
            defense: self.defense,
            spatk: self.spatk,
            spdef: self.spdef,
            speed: self.speed,
            special: self.special,
        }
    }

    pub fn bst(&self) -> i32 {
        self.stats().bst()
    }

    pub fn bst_for_power_levels(&self) -> i32 {
        match self.layout {
            StatLayout::Gen1 => self.bst(),
            StatLayout::Standard if self.number == species::SHEDINJA => {
                (self.attack + self.defense + self.spatk + self.spdef + self.speed) * 6 / 5
            }
            StatLayout::Standard => self.bst(),
        }
    }

    pub fn attack_special_attack_ratio(&self) -> f64 {
        let special_attack = match self.layout {
            StatLayout::Gen1 => self.special,
            StatLayout::Standard => self.spatk,
        };
        f64::from(self.attack) / (f64::from(self.attack) + f64::from(special_attack))
    }

    pub fn full_name(&self) -> String {
        format!("{}{}", self.name, self.forme_suffix)
    }

    pub fn sprite_index(&self) -> i32 {
        if self.forme_number == 0 {
            self.number
        } else {
            self.forme_sprite_index + self.forme_number - 1
        }
    }

    pub fn cosmetic_form_number(&self, number: i32) -> i32 {
        if self.real_cosmetic_form_numbers.is_empty() {
            number
        } else {
            self.real_cosmetic_form_numbers[number as usize]
        }
    }

    pub fn is_ultra_beast(&self) -> bool {
        is_ultra_beast_number(self.number)
    }

    fn stat_values(&self) -> Vec<i32> {
        match self.layout {
            StatLayout::Gen1 => vec![self.hp, self.attack, self.defense, self.special, self.speed],
            StatLayout::Standard => vec![
                self.hp,
                self.attack,
                self.defense,
                self.spatk,
                self.spdef,
                self.speed,
            ],
        }
    }

    fn apply_shuffled_order_to_stats(&mut self) {
        let stats = self.stat_values();
        let order = self.shuffled_stats_order.clone();
        let pick = |slot: usize| stats[order[slot]];
        match self.layout {
            StatLayout::Gen1 => {
                self.hp = pick(0);
                self.attack = pick(1);
                self.defense = pick(2);
                self.special = pick(3);
                self.speed = pick(4);
            }
            StatLayout::Standard => {
                self.hp = pick(0);
                self.attack = pick(1);
                self.defense = pick(2);
                self.spatk = pick(3);
                self.spdef = pick(4);
                self.speed = pick(5);
            }
        }
    }

    pub fn shuffle_stats(&mut self, random: &mut impl JavaRng) {
        shuffle(&mut self.shuffled_stats_order, random);
        self.apply_shuffled_order_to_stats();
    }

    pub fn copy_shuffled_stats_up_evolution(&mut self, evolves_from_order: &[usize]) {
        let own_order = self.shuffled_stats_order.clone();
        self.shuffled_stats_order = (0..own_order.len())
            .map(|stat| {
                own_order
                    .iter()
                    .position(|&slot| slot == stat)
                    .unwrap_or(usize::MAX)
            })
            .collect();
        self.apply_shuffled_order_to_stats();
        self.shuffled_stats_order = evolves_from_order.to_vec();
        self.apply_shuffled_order_to_stats();
    }

    pub fn randomize_stats_within_bst(&mut self, random: &mut impl JavaRng) {
        loop {
            match self.layout {
                StatLayout::Gen1 => self.roll_gen1_stats(random),
                StatLayout::Standard if self.number == species::SHEDINJA => {
                    self.roll_shedinja_stats(random)
                }
                StatLayout::Standard => self.roll_standard_stats(random),
            }
            if !self.has_unstorable_stat() {
                return;
            }
        }
    }

    fn has_unstorable_stat(&self) -> bool {
        match self.layout {
            StatLayout::Gen1 => [self.hp, self.attack, self.defense, self.special, self.speed]
                .iter()
                .any(|&stat| stat > 255),
            StatLayout::Standard => [
                self.hp,
                self.attack,
                self.defense,
                self.spatk,
                self.spdef,
                self.speed,
            ]
            .iter()
            .any(|&stat| stat > 255),
        }
    }

    fn roll_gen1_stats(&mut self, random: &mut impl JavaRng) {
        let bst = self.bst() - 60;
        let hp_weight = random.next_double();
        let attack_weight = random.next_double();
        let defense_weight = random.next_double();
        let special_weight = random.next_double();
        let speed_weight = random.next_double();
        let total = hp_weight + attack_weight + defense_weight + special_weight + speed_weight;
        self.hp = weighted_share(hp_weight, total, bst) + 20;
        self.attack = weighted_share(attack_weight, total, bst) + 10;
        self.defense = weighted_share(defense_weight, total, bst) + 10;
        self.special = weighted_share(special_weight, total, bst) + 10;
        self.speed = weighted_share(speed_weight, total, bst) + 10;
    }

    fn roll_shedinja_stats(&mut self, random: &mut impl JavaRng) {
        let bst = self.bst() - 51;
        let attack_weight = random.next_double();
        let defense_weight = random.next_double();
        let spatk_weight = random.next_double();
        let spdef_weight = random.next_double();
        let speed_weight = random.next_double();
        let total = attack_weight + defense_weight + spatk_weight + spdef_weight + speed_weight;
        self.hp = 1;
        self.attack = weighted_share(attack_weight, total, bst) + 10;
        self.defense = weighted_share(defense_weight, total, bst) + 10;
        self.spatk = weighted_share(spatk_weight, total, bst) + 10;
        self.spdef = weighted_share(spdef_weight, total, bst) + 10;
        self.speed = weighted_share(speed_weight, total, bst) + 10;
    }

    fn roll_standard_stats(&mut self, random: &mut impl JavaRng) {
        let bst = self.bst() - 70;
        let hp_weight = random.next_double();
        let attack_weight = random.next_double();
        let defense_weight = random.next_double();
        let spatk_weight = random.next_double();
        let spdef_weight = random.next_double();
        let speed_weight = random.next_double();
        let total =
            hp_weight + attack_weight + defense_weight + spatk_weight + spdef_weight + speed_weight;
        self.hp = weighted_share(hp_weight, total, bst) + 20;
        self.attack = weighted_share(attack_weight, total, bst) + 10;
        self.defense = weighted_share(defense_weight, total, bst) + 10;
        self.spatk = weighted_share(spatk_weight, total, bst) + 10;
        self.spdef = weighted_share(spdef_weight, total, bst) + 10;
        self.speed = weighted_share(speed_weight, total, bst) + 10;
    }

    pub fn copy_randomized_stats_up_evolution(&mut self, evolves_from: &BaseStats) {
        let ratio = f64::from(self.bst()) / f64::from(evolves_from.bst());
        let scaled = |stat: i32| clamp_long_to_stat(round_double(f64::from(stat) * ratio));
        self.hp = scaled(evolves_from.hp);
        self.attack = scaled(evolves_from.attack);
        self.defense = scaled(evolves_from.defense);
        self.speed = scaled(evolves_from.speed);
        match self.layout {
            StatLayout::Gen1 => self.special = scaled(evolves_from.special),
            StatLayout::Standard => {
                self.spatk = scaled(evolves_from.spatk);
                self.spdef = scaled(evolves_from.spdef);
            }
        }
    }

    pub fn assign_new_stats_for_evolution(
        &mut self,
        evolves_from: &BaseStats,
        random: &mut impl JavaRng,
    ) {
        let difference = f64::from(self.bst()) - f64::from(evolves_from.bst());
        match self.layout {
            StatLayout::Gen1 => {
                let hp_weight = random.next_double();
                let attack_weight = random.next_double();
                let defense_weight = random.next_double();
                let special_weight = random.next_double();
                let speed_weight = random.next_double();
                let total =
                    hp_weight + attack_weight + defense_weight + special_weight + speed_weight;
                let grown = |stat: i32, weight: f64| {
                    clamp_double_to_stat(
                        f64::from(stat) + rounded_difference(weight, total, difference),
                    )
                };
                self.hp = grown(evolves_from.hp, hp_weight);
                self.attack = grown(evolves_from.attack, attack_weight);
                self.defense = grown(evolves_from.defense, defense_weight);
                self.speed = grown(evolves_from.speed, speed_weight);
                self.special = grown(evolves_from.special, special_weight);
            }
            StatLayout::Standard => {
                let hp_weight = random.next_double();
                let attack_weight = random.next_double();
                let defense_weight = random.next_double();
                let spatk_weight = random.next_double();
                let spdef_weight = random.next_double();
                let speed_weight = random.next_double();
                let total = hp_weight
                    + attack_weight
                    + defense_weight
                    + spatk_weight
                    + spdef_weight
                    + speed_weight;
                let grown = |stat: i32, weight: f64| {
                    clamp_double_to_stat(
                        f64::from(stat) + rounded_difference(weight, total, difference),
                    )
                };
                self.hp = grown(evolves_from.hp, hp_weight);
                self.attack = grown(evolves_from.attack, attack_weight);
                self.defense = grown(evolves_from.defense, defense_weight);
                self.speed = grown(evolves_from.speed, speed_weight);
                self.spatk = grown(evolves_from.spatk, spatk_weight);
                self.spdef = grown(evolves_from.spdef, spdef_weight);
            }
        }
    }

    pub fn copy_base_forme_base_stats(&mut self, base: &BaseStats) {
        self.hp = base.hp;
        self.attack = base.attack;
        self.defense = base.defense;
        self.speed = base.speed;
        self.spatk = base.spatk;
        self.spdef = base.spdef;
    }
}
