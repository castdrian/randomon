use randomon_python_random::PythonRandom;

use crate::bundles::Bundles;
use crate::data::GameData;
use crate::error::{BdspError, Result};
use crate::log::SpoilerLog;
use crate::settings::BdspSettings;

pub struct Context<'a> {
    pub rng: PythonRandom,
    pub data: GameData,
    pub bundles: Bundles,
    pub settings: &'a BdspSettings,
    pub log: SpoilerLog,
    pub starters: Option<[u16; 3]>,
    pub exefs_patches: Vec<(String, Vec<u8>)>,
    pub learnsets_changed: bool,
    pub abilities_changed: bool,
}

impl Context<'_> {
    pub fn pick<T: Copy>(&mut self, pool: &[T], what: &str) -> Result<T> {
        self.rng
            .choice(pool)
            .copied()
            .map_err(|_| BdspError::EmptyPool(what.to_owned()))
    }

    pub fn chance(&mut self, probability: f64) -> bool {
        self.rng.random() < probability
    }

    pub fn reload_data(&mut self) -> Result<()> {
        let names = std::mem::take(&mut self.data.names);
        self.data = GameData::load(&self.bundles)?;
        self.data.names = names;
        Ok(())
    }
}

pub fn similar_strength_pool(data: &GameData, candidates: &[u16], target_total: u32) -> Vec<u16> {
    for tolerance_percent in (10..=60).step_by(5) {
        let tolerance = target_total * tolerance_percent / 100;
        let low = target_total.saturating_sub(tolerance);
        let high = target_total + tolerance;
        let close: Vec<u16> = candidates
            .iter()
            .copied()
            .filter(|&species| (low..=high).contains(&data.base_stat_total(species)))
            .collect();
        if close.len() >= 4 {
            return close;
        }
    }
    candidates.to_vec()
}

pub fn without(pool: &[u16], used: &[u16]) -> Vec<u16> {
    let rest: Vec<u16> = pool
        .iter()
        .copied()
        .filter(|species| !used.contains(species))
        .collect();
    if rest.is_empty() { pool.to_vec() } else { rest }
}
