use randomon_data::StaticEncounter;
use randomon_unity::Value;

use crate::assets::BundleId;
use crate::bundles::Bundles;
use crate::constants;
use crate::error::Result;
use crate::handler::BdspRomHandler;
use crate::items::{decode_item_argument, encode_item_argument};

const WILD_BATTLE: i64 = 401;
const SPECIAL_WILD_BATTLE: i64 = 402;
const PLAY_CRY: i64 = 112;
const FLOAT_ARGUMENT: i64 = 1;

pub struct ScriptAsset {
    pub path_id: i64,
    pub name: String,
    pub value: Value,
    pub dirty: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StaticSlot {
    pub asset_name: String,
    pub species: i32,
    pub level: i32,
}

fn commands(script: &Value) -> &[Value] {
    script
        .field("Commands")
        .ok()
        .and_then(Value::as_array)
        .unwrap_or_default()
}

fn float_argument(arguments: &[Value], index: usize) -> Option<i32> {
    let argument = arguments.get(index)?;
    if argument.int_field("argType").ok()? != FLOAT_ARGUMENT {
        return None;
    }
    decode_item_argument(argument.int_field("data").ok()?)
}

fn command_id(arguments: &[Value]) -> Option<i64> {
    arguments.first()?.int_field("data").ok()
}

fn arguments(command: &Value) -> &[Value] {
    command
        .field("Arg")
        .ok()
        .and_then(Value::as_array)
        .unwrap_or_default()
}

fn battle_of(command: &Value) -> Option<(i32, i32)> {
    let arguments = arguments(command);
    let id = command_id(arguments)?;
    if id != WILD_BATTLE && id != SPECIAL_WILD_BATTLE {
        return None;
    }
    let species = float_argument(arguments, 1)?;
    let level = float_argument(arguments, 2)?;
    (1..=constants::SPECIES_COUNT)
        .contains(&species)
        .then_some((species, level))
}

fn scripts(asset: &Value) -> &[Value] {
    asset
        .field("Scripts")
        .ok()
        .and_then(Value::as_array)
        .unwrap_or_default()
}

pub fn load_battle_scripts(bundles: &Bundles) -> Result<Vec<ScriptAsset>> {
    Ok(bundles
        .monobehaviours_named(BundleId::EvScript, "")?
        .into_iter()
        .filter(|(_, value)| {
            scripts(value).iter().any(|script| {
                commands(script)
                    .iter()
                    .any(|command| battle_of(command).is_some())
            })
        })
        .map(|(path_id, value)| ScriptAsset {
            path_id,
            name: value
                .field("m_Name")
                .ok()
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned(),
            value,
            dirty: false,
        })
        .collect())
}

pub fn static_slots(assets: &[ScriptAsset]) -> Vec<StaticSlot> {
    let mut slots: Vec<StaticSlot> = Vec::new();
    for asset in assets {
        for script in scripts(&asset.value) {
            for command in commands(script) {
                let Some((species, level)) = battle_of(command) else {
                    continue;
                };
                let slot = StaticSlot {
                    asset_name: asset.name.clone(),
                    species,
                    level,
                };
                if !slots
                    .iter()
                    .any(|known| known.asset_name == slot.asset_name && known.species == species)
                {
                    slots.push(slot);
                }
            }
        }
    }
    slots
}

fn set_float_argument(command: &mut Value, index: usize, number: i32) -> Result<()> {
    if let Some(argument) = command
        .field_mut("Arg")?
        .as_array_mut()
        .and_then(|arguments| arguments.get_mut(index))
    {
        argument.set_int_field("data", encode_item_argument(number))?;
    }
    Ok(())
}

fn rewrite_asset(asset: &mut ScriptAsset, replacements: &[(i32, i32, i32)]) -> Result<()> {
    let replacement_for = |species: i32| {
        replacements
            .iter()
            .find(|(original, _, _)| *original == species)
            .map(|&(_, new_species, level)| (new_species, level))
    };
    let Some(scripts) = asset.value.field_mut("Scripts")?.as_array_mut() else {
        return Ok(());
    };
    for script in scripts.iter_mut() {
        let battles: Vec<i32> = commands(script)
            .iter()
            .filter_map(battle_of)
            .map(|(species, _)| species)
            .collect();
        if !battles
            .iter()
            .any(|&species| replacement_for(species).is_some())
        {
            continue;
        }
        let Some(list) = script.field_mut("Commands")?.as_array_mut() else {
            continue;
        };
        for command in list.iter_mut() {
            let arguments = arguments(command);
            let id = command_id(arguments);
            let Some(original) = float_argument(arguments, 1) else {
                continue;
            };
            if !battles.contains(&original) {
                continue;
            }
            let Some((species, level)) = replacement_for(original) else {
                continue;
            };
            match id {
                Some(WILD_BATTLE | SPECIAL_WILD_BATTLE) => {
                    let has_level = float_argument(arguments, 2).is_some();
                    set_float_argument(command, 1, species)?;
                    if has_level {
                        set_float_argument(command, 2, level)?;
                    }
                }
                Some(PLAY_CRY) => set_float_argument(command, 1, species)?,
                _ => {}
            }
        }
        asset.dirty = true;
    }
    Ok(())
}

impl BdspRomHandler {
    pub(crate) fn read_static_pokemon(&self) -> Vec<StaticEncounter> {
        static_slots(&self.battle_scripts)
            .iter()
            .map(|slot| {
                let mut encounter = StaticEncounter::new(self.pk(slot.species));
                encounter.level = slot.level;
                encounter
            })
            .collect()
    }

    pub(crate) fn write_static_pokemon(&mut self, statics: &[StaticEncounter]) -> Result<()> {
        let slots = static_slots(&self.battle_scripts);
        let mut replacements: Vec<(String, (i32, i32, i32))> = Vec::new();
        for (slot, encounter) in slots.iter().zip(statics) {
            let species = self.core.data.base_number(encounter.pkmn);
            let level = encounter.level.clamp(1, 100);
            replacements.push((slot.asset_name.clone(), (slot.species, species, level)));
        }
        for asset in self.battle_scripts.iter_mut() {
            let own: Vec<(i32, i32, i32)> = replacements
                .iter()
                .filter(|(name, _)| *name == asset.name)
                .map(|(_, replacement)| *replacement)
                .collect();
            if !own.is_empty() {
                rewrite_asset(asset, &own)?;
            }
        }
        Ok(())
    }

    pub(crate) fn write_back_battle_scripts(&mut self) -> Result<()> {
        for asset in self.battle_scripts.iter().filter(|asset| asset.dirty) {
            self.bundles
                .write(BundleId::EvScript, asset.path_id, &asset.value)?;
        }
        Ok(())
    }
}
