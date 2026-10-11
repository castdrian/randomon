use randomon_core::engine::evolutions::{add_evo_update_level, add_evo_update_stone};
use randomon_core::{RandomizationError, Result};
use randomon_data::constants::{gb, species};
use randomon_data::{EvolutionId, EvolutionType, PokemonId, Type};
use randomon_java::JavaRng;
use randomon_settings::MiscTweak;

use crate::assets::patch_bytes;
use crate::constants::{self as c, POKEMON_COUNT};
use crate::handler::Gen2RomHandler;
use crate::items;

const NEUTRAL: u8 = 10;
const HAPPINESS_REQUIRED: u8 = 220;
const EASIER_HAPPINESS_REQUIRED: u8 = 160;
const CONDITIONAL_JUMP: u8 = 0xCA;
const UNCONDITIONAL_JUMP: u8 = 0xC3;

enum EvolutionChange {
    Stone(i32),
    Level(i32),
}

impl Gen2RomHandler {
    pub(crate) fn random_species_except_unown(&mut self) -> i32 {
        let random = &mut self.core.engine.random;
        let mut number = random.next_int_bounded(POKEMON_COUNT) + 1;
        while number == species::UNOWN {
            number = random.next_int_bounded(POKEMON_COUNT) + 1;
        }
        number
    }

    pub(crate) fn randomize_intro(&mut self) {
        let number = self.random_species_except_unown() as u8;
        self.set_byte(self.value("IntroSpriteOffset"), number);
        self.set_byte(self.value("IntroCryOffset"), number);
    }

    fn randomize_catching_tutorial(&mut self) {
        if !self.entry.has_array("CatchingTutorialOffsets") {
            return;
        }
        let number = self.random_species_except_unown() as u8;
        for &offset in self.array("CatchingTutorialOffsets") {
            self.set_byte(offset, number);
        }
    }

    fn apply_bw_exp_patch(&mut self) -> Result<()> {
        let Some(name) = self.entry.code_tweaks.get("BWXPTweak") else {
            return Ok(());
        };
        let patch = patch_bytes(name)
            .ok_or_else(|| RandomizationError::Io(format!("missing patch {name}")))?;
        randomon_gb::apply_ips_patch(&mut self.rom, patch).map_err(RandomizationError::Io)
    }

    fn apply_fastest_text(&mut self) {
        let offset = self.value("TextDelayFunctionOffset");
        if offset != 0 {
            self.set_byte(offset, gb::Z80_RET);
        }
    }

    fn apply_camel_case_names(&mut self) {
        for id in self.pokemon_list.clone().into_iter().flatten() {
            let name = randomon_data::text::camel_case(&self.core.data.pk(id).name);
            self.core.data.pk_mut(id).name = name;
        }
    }

    fn update_type_effectiveness(&mut self) {
        let mut table = self.type_relationships();
        self.core
            .engine
            .handler_log("--Updating Type Effectiveness--");
        for (attacker, defender, effectiveness) in &mut table {
            if *attacker == Some(Type::Ghost) && *defender == Some(Type::Steel) {
                *effectiveness = NEUTRAL;
                self.core.engine.handler_log(
                    "Replaced: Ghost not very effective vs Steel => Ghost neutral vs Steel",
                );
            } else if *attacker == Some(Type::Dark) && *defender == Some(Type::Steel) {
                *effectiveness = NEUTRAL;
                self.core.engine.handler_log(
                    "Replaced: Dark not very effective vs Steel => Dark neutral vs Steel",
                );
            }
        }
        self.core.engine.handler_log_blank_line();
        let mut offset = self.value("TypeEffectivenessOffset");
        for (attacker, defender, effectiveness) in table {
            self.set_byte(offset, c::type_to_byte(attacker));
            self.set_byte(offset + 1, c::type_to_byte(defender));
            self.set_byte(offset + 2, effectiveness);
            offset += 3;
        }
        self.effectiveness_updated = true;
    }

    pub(crate) fn available_misc_tweaks(&self) -> i32 {
        let mut available =
            MiscTweak::LowerCasePokemonNames.value() | MiscTweak::UpdateTypeEffectiveness.value();
        if self.entry.code_tweaks.contains_key("BWXPTweak") {
            available |= MiscTweak::BwExpPatch.value();
        }
        if self.value("TextDelayFunctionOffset") != 0 {
            available |= MiscTweak::FastestText.value();
        }
        if self.entry.has_array("CatchingTutorialOffsets") {
            available |= MiscTweak::RandomizeCatchingTutorial.value();
        }
        available | MiscTweak::BanLuckyEgg.value()
    }

    pub(crate) fn apply_tweak(&mut self, tweak: MiscTweak) -> Result<()> {
        match tweak {
            MiscTweak::BwExpPatch => self.apply_bw_exp_patch()?,
            MiscTweak::FastestText => self.apply_fastest_text(),
            MiscTweak::LowerCasePokemonNames => self.apply_camel_case_names(),
            MiscTweak::RandomizeCatchingTutorial => self.randomize_catching_tutorial(),
            MiscTweak::BanLuckyEgg => {
                self.allowed_items.ban_singles(&[items::LUCKY_EGG]);
                self.non_bad_items.ban_singles(&[items::LUCKY_EGG]);
            }
            MiscTweak::UpdateTypeEffectiveness => self.update_type_effectiveness(),
            _ => {}
        }
        Ok(())
    }

    pub(crate) fn guarantee_catching(&mut self) -> Result<()> {
        let prefix = self.entry.string("GuaranteedCatchPrefix");
        let offset = self.find_unique(prefix)?;
        if offset > 0 {
            let target = offset + prefix.len() as i32 / 2;
            if self.byte(target) == CONDITIONAL_JUMP {
                self.set_byte(target, UNCONDITIONAL_JUMP);
            }
        }
        Ok(())
    }

    pub(crate) fn lower_happiness_requirement(&mut self) -> Result<()> {
        let offset = self.find_unique(c::FRIENDSHIP_VALUE_FOR_EVO_LOCATOR)?;
        if offset > 0 && self.byte(offset + 1) == HAPPINESS_REQUIRED {
            self.set_byte(offset + 1, EASIER_HAPPINESS_REQUIRED);
        }
        Ok(())
    }

    fn all_evolutions(&self) -> Vec<EvolutionId> {
        self.pokemon_list
            .iter()
            .flatten()
            .flat_map(|&id| self.core.data.evolutions_from(id).to_vec())
            .collect()
    }

    fn impossible_evolution_change(&self, evo: EvolutionId) -> Option<EvolutionChange> {
        let evolution = self.core.data.evo(evo);
        if !matches!(
            evolution.kind,
            EvolutionType::Trade | EvolutionType::TradeItem
        ) {
            return None;
        }
        let from = self.core.data.pk(evolution.from).number;
        let change = if from == species::SLOWPOKE {
            EvolutionChange::Stone(items::WATER_STONE)
        } else if from == species::SEADRA {
            EvolutionChange::Level(40)
        } else if from == species::POLIWHIRL || evolution.kind == EvolutionType::Trade {
            EvolutionChange::Level(37)
        } else {
            EvolutionChange::Level(30)
        };
        Some(change)
    }

    fn apply_evolution_change(
        &mut self,
        evo: EvolutionId,
        change: EvolutionChange,
        time_based: bool,
    ) {
        let item_names = &self.item_names;
        let core = &mut self.core;
        let updates = if time_based {
            &mut core.engine.time_based_evolution_updates
        } else {
            &mut core.engine.impossible_evolution_updates
        };
        let evolution = core.data.evo_mut(evo);
        match change {
            EvolutionChange::Stone(item) => {
                evolution.kind = EvolutionType::Stone;
                evolution.extra_info = item;
                add_evo_update_stone(&core.data, updates, evo, &item_names[item as usize]);
            }
            EvolutionChange::Level(level) => {
                evolution.kind = EvolutionType::Level;
                evolution.extra_info = level;
                add_evo_update_level(&core.data, updates, evo);
            }
        }
    }

    pub(crate) fn remove_trade_evolutions(&mut self) {
        for evo in self.all_evolutions() {
            if let Some(change) = self.impossible_evolution_change(evo) {
                self.apply_evolution_change(evo, change, false);
            }
        }
    }

    pub(crate) fn remove_time_evolutions(&mut self) {
        for evo in self.all_evolutions() {
            let change = match self.core.data.evo(evo).kind {
                EvolutionType::HappinessDay => EvolutionChange::Stone(items::SUN_STONE),
                EvolutionType::HappinessNight => EvolutionChange::Stone(items::MOON_STONE),
                _ => continue,
            };
            self.apply_evolution_change(evo, change, true);
        }
    }

    pub(crate) fn remove_evolutions_outside_pool(&mut self) {
        let included: Vec<PokemonId> = self.core.engine.pool.main.clone();
        for id in self.pokemon_list.clone().into_iter().flatten() {
            let mut keep = Vec::new();
            for evo in self.core.data.evolutions_from(id).to_vec() {
                let evolution = self.core.data.evo(evo).clone();
                if included.contains(&evolution.from) && included.contains(&evolution.to) {
                    keep.push(evolution);
                } else {
                    self.core.data.remove_first_equal_to(evolution.to, evo);
                }
            }
            let data = &mut self.core.data;
            let retained: Vec<EvolutionId> = data
                .evolutions_from(id)
                .iter()
                .copied()
                .filter(|&evo| keep.iter().any(|kept| data.evo(evo).java_equals(kept)))
                .collect();
            *data.evolutions_from_mut(id) = retained;
        }
    }
}
