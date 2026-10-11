use std::collections::BTreeMap;

use randomon_unity::Value;

use crate::assets::{BundleId, SPECIES_CATEGORIES, SPECIES_NAMES, path_ids};
use crate::bundles::Bundles;
use crate::error::{Result, data_error};

pub const MAX_SPECIES: u16 = 493;
pub const MAX_ABILITY: u16 = 267;
pub const STRUGGLE: u16 = 165;
pub const TM_COUNT: i64 = 100;

pub const LEGENDARIES: [u16; 35] = [
    144, 145, 146, 150, 151, 243, 244, 245, 249, 250, 251, 377, 378, 379, 380, 381, 382, 383, 384,
    385, 386, 480, 481, 482, 483, 484, 485, 486, 487, 488, 489, 490, 491, 492, 493,
];

pub const BROKEN_ABILITIES: [u16; 11] = [25, 176, 209, 210, 211, 225, 241, 248, 258, 266, 267];

pub fn is_legendary(species: u16) -> bool {
    LEGENDARIES.contains(&species)
}

pub fn generation_of(species: u16) -> u8 {
    match species {
        1..=151 => 1,
        152..=251 => 2,
        252..=386 => 3,
        _ => 4,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Species {
    pub id: u16,
    pub types: [u8; 2],
    pub base_stat_total: u32,
    pub abilities: [u16; 3],
    pub color: u8,
}

impl Species {
    pub fn has_type(&self, kind: u8) -> bool {
        self.types.contains(&kind)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MoveInfo {
    pub id: u16,
    pub kind: u8,
    pub damage_type: u8,
    pub power: u8,
}

impl MoveInfo {
    pub fn is_damaging(&self) -> bool {
        self.damage_type != 0 && self.power > 1
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemInfo {
    pub id: u16,
    pub pocket: u8,
    pub price: i64,
}

#[derive(Debug, Clone, Default)]
pub struct Names {
    pub species: Vec<String>,
    pub categories: Vec<String>,
    pub moves: BTreeMap<u16, String>,
    pub items: BTreeMap<u16, String>,
    pub abilities: BTreeMap<u16, String>,
}

impl Names {
    pub fn species(&self, id: u16) -> String {
        self.species
            .get(usize::from(id))
            .cloned()
            .unwrap_or_else(|| format!("#{id}"))
    }

    pub fn category(&self, id: u16) -> String {
        self.categories
            .get(usize::from(id))
            .filter(|text| !text.is_empty())
            .map(|text| text.replace("Pokemon", "Pokémon"))
            .unwrap_or_else(|| "Mystery Pokémon".to_owned())
    }

    pub fn move_name(&self, id: u16) -> String {
        self.moves
            .get(&id)
            .cloned()
            .unwrap_or_else(|| format!("Move {id}"))
    }

    pub fn item(&self, id: u16) -> String {
        self.items
            .get(&id)
            .cloned()
            .unwrap_or_else(|| format!("Item {id}"))
    }

    pub fn ability(&self, id: u16) -> String {
        self.abilities
            .get(&id)
            .cloned()
            .unwrap_or_else(|| format!("Ability {id}"))
    }
}

#[derive(Debug, Clone)]
pub struct GameData {
    pub species: BTreeMap<u16, Species>,
    pub moves: BTreeMap<u16, MoveInfo>,
    pub items: BTreeMap<u16, ItemInfo>,
    pub evolutions: BTreeMap<u16, Vec<u16>>,
    pub pre_evolution: BTreeMap<u16, u16>,
    pub names: Names,
}

fn as_u16(value: i64) -> Result<u16> {
    u16::try_from(value).map_err(|_| data_error(format!("value {value} out of range")))
}

fn as_u8(value: i64) -> Result<u8> {
    u8::try_from(value).map_err(|_| data_error(format!("value {value} out of range")))
}

fn array<'a>(value: &'a Value, name: &str) -> Result<&'a [Value]> {
    value
        .field(name)?
        .as_array()
        .ok_or_else(|| data_error(format!("{name} is not an array")))
}

fn load_species(bundles: &Bundles) -> Result<BTreeMap<u16, Species>> {
    let table = bundles.read(BundleId::PersonalMasterdatas, path_ids::PERSONAL_TABLE)?;
    let mut species = BTreeMap::new();
    for entry in array(&table, "Personal")? {
        let id = as_u16(entry.int_field("id")?)?;
        let monsno = as_u16(entry.int_field("monsno")?)?;
        if id != monsno || id == 0 || id > MAX_SPECIES {
            continue;
        }
        let stat = |name: &str| entry.int_field(name).map(|value| value as u32);
        let base_stat_total = stat("basic_hp")?
            + stat("basic_atk")?
            + stat("basic_def")?
            + stat("basic_agi")?
            + stat("basic_spatk")?
            + stat("basic_spdef")?;
        species.insert(
            id,
            Species {
                id,
                types: [
                    as_u8(entry.int_field("type1")?)?,
                    as_u8(entry.int_field("type2")?)?,
                ],
                base_stat_total,
                abilities: [
                    as_u16(entry.int_field("tokusei1")?)?,
                    as_u16(entry.int_field("tokusei2")?)?,
                    as_u16(entry.int_field("tokusei3")?)?,
                ],
                color: as_u8(entry.int_field("color")?)?,
            },
        );
    }
    if species.len() != usize::from(MAX_SPECIES) {
        return Err(data_error(format!(
            "expected {MAX_SPECIES} species in PersonalTable, found {}",
            species.len()
        )));
    }
    Ok(species)
}

fn load_moves(bundles: &Bundles) -> Result<BTreeMap<u16, MoveInfo>> {
    let table = bundles.read(BundleId::PersonalMasterdatas, path_ids::WAZA_TABLE)?;
    let mut moves = BTreeMap::new();
    for entry in array(&table, "Waza")? {
        let id = as_u16(entry.int_field("wazaNo")?)?;
        if id == 0 || id == STRUGGLE || entry.int_field("isValid")? != 1 {
            continue;
        }
        moves.insert(
            id,
            MoveInfo {
                id,
                kind: as_u8(entry.int_field("type")?)?,
                damage_type: as_u8(entry.int_field("damageType")?)?,
                power: as_u8(entry.int_field("power")?)?,
            },
        );
    }
    Ok(moves)
}

fn load_items(bundles: &Bundles) -> Result<BTreeMap<u16, ItemInfo>> {
    let table = bundles.read(BundleId::PersonalMasterdatas, path_ids::ITEM_TABLE)?;
    let mut items = BTreeMap::new();
    for entry in array(&table, "Item")? {
        let id = as_u16(entry.int_field("no")?)?;
        items.insert(
            id,
            ItemInfo {
                id,
                pocket: as_u8(entry.int_field("fld_pocket")?)?,
                price: entry.int_field("price")?,
            },
        );
    }
    Ok(items)
}

pub struct EvolutionStep {
    pub target: u16,
}

pub fn evolution_steps(entry: &Value) -> Result<Vec<EvolutionStep>> {
    let data = array(entry, "ar")?;
    data.as_chunks::<5>()
        .0
        .iter()
        .map(|chunk| {
            Ok(EvolutionStep {
                target: as_u16(chunk[2].int()?)?,
            })
        })
        .collect()
}

fn load_evolutions(bundles: &Bundles) -> Result<BTreeMap<u16, Vec<u16>>> {
    let table = bundles.read(BundleId::PersonalMasterdatas, path_ids::EVOLVE_TABLE)?;
    let mut evolutions = BTreeMap::new();
    for entry in array(&table, "Evolve")? {
        let id = as_u16(entry.int_field("id")?)?;
        if id == 0 || id > MAX_SPECIES {
            continue;
        }
        let targets: Vec<u16> = evolution_steps(entry)?
            .into_iter()
            .map(|step| step.target)
            .filter(|&target| (1..=MAX_SPECIES).contains(&target) && target != id)
            .collect();
        if !targets.is_empty() {
            evolutions.insert(id, targets);
        }
    }
    Ok(evolutions)
}

fn message_texts(bundles: &Bundles, name: &str) -> BTreeMap<u16, String> {
    let Ok(found) = bundles.monobehaviours_named(BundleId::English, name) else {
        return BTreeMap::new();
    };
    let Some((_, table)) = found
        .into_iter()
        .find(|(_, value)| value.field("m_Name").ok().and_then(Value::as_str) == Some(name))
    else {
        return BTreeMap::new();
    };
    let mut texts = BTreeMap::new();
    let Ok(labels) = array(&table, "labelDataArray") else {
        return texts;
    };
    for label in labels {
        let Ok(index) = label.int_field("arrayIndex") else {
            continue;
        };
        let text: String = array(label, "wordDataArray")
            .unwrap_or_default()
            .iter()
            .filter_map(|word| word.field("str").ok().and_then(Value::as_str))
            .collect();
        if let Ok(index) = u16::try_from(index) {
            texts.insert(index, text);
        }
    }
    texts
}

fn text_lines(text: &str) -> Vec<String> {
    text.lines().map(|line| line.trim().to_owned()).collect()
}

impl GameData {
    pub fn load(bundles: &Bundles) -> Result<Self> {
        let evolutions = load_evolutions(bundles)?;
        let mut pre_evolution = BTreeMap::new();
        for (&source, targets) in &evolutions {
            for &target in targets {
                pre_evolution.entry(target).or_insert(source);
            }
        }
        let mut species_names = text_lines(SPECIES_NAMES);
        if let Some(first) = species_names.first_mut() {
            first.clear();
        }
        Ok(Self {
            species: load_species(bundles)?,
            moves: load_moves(bundles)?,
            items: load_items(bundles)?,
            evolutions,
            pre_evolution,
            names: Names {
                species: species_names,
                categories: text_lines(SPECIES_CATEGORIES),
                moves: message_texts(bundles, "english_ss_wazaname"),
                items: message_texts(bundles, "english_ss_itemname"),
                abilities: message_texts(bundles, "english_ss_tokusei"),
            },
        })
    }

    pub fn species(&self, id: u16) -> Option<&Species> {
        self.species.get(&id)
    }

    pub fn is_basic(&self, id: u16) -> bool {
        !self.pre_evolution.contains_key(&id)
    }

    pub fn evolution_depth(&self, id: u16) -> usize {
        let mut depth = 0;
        let mut current = id;
        while let Some(next) = self
            .evolutions
            .get(&current)
            .and_then(|targets| targets.first())
        {
            depth += 1;
            current = *next;
            if depth > 4 {
                break;
            }
        }
        depth
    }

    pub fn family_root(&self, id: u16) -> u16 {
        let mut current = id;
        let mut steps = 0;
        while let Some(&previous) = self.pre_evolution.get(&current) {
            current = previous;
            steps += 1;
            if steps > 4 {
                break;
            }
        }
        current
    }

    pub fn evolve_to_stage(&self, root: u16, stage: usize) -> u16 {
        let mut current = root;
        for _ in 0..stage {
            match self
                .evolutions
                .get(&current)
                .and_then(|targets| targets.first())
            {
                Some(&next) => current = next,
                None => break,
            }
        }
        current
    }

    pub fn stage_of(&self, id: u16) -> usize {
        let mut stage = 0;
        let mut current = id;
        while let Some(&previous) = self.pre_evolution.get(&current) {
            current = previous;
            stage += 1;
            if stage > 4 {
                break;
            }
        }
        stage
    }

    pub fn base_stat_total(&self, id: u16) -> u32 {
        self.species(id)
            .map_or(0, |species| species.base_stat_total)
    }

    pub fn valid_move(&self, id: u16) -> bool {
        self.moves.contains_key(&id)
    }

    pub fn valid_species(&self, id: u16) -> bool {
        self.species.contains_key(&id)
    }
}
