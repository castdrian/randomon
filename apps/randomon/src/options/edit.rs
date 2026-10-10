use randomon_settings::{ExpCurve, MiscTweak, Settings, TrainersMod};

use super::spec::{FlagSetter, NumberSetter, Selector};

#[derive(Debug, Clone, Copy)]
pub enum Edit {
    Flag(FlagSetter, bool),
    Number(NumberSetter, i32),
    Select(Selector),
    ExpCurve(ExpCurve),
    BaseStatGeneration(i32),
    MoveGeneration(i32),
    Starter(usize, i32),
    TrainerMode(TrainersMod),
    MiscTweak(MiscTweak, bool),
}

impl Edit {
    pub fn apply(self, settings: &mut Settings) {
        match self {
            Edit::Flag(set, value) => set(settings, value),
            Edit::Number(set, value) => set(settings, value),
            Edit::Select(select) => select(settings),
            Edit::ExpCurve(curve) => settings.selected_exp_curve = Some(curve),
            Edit::BaseStatGeneration(generation) => {
                settings.update_base_stats_to_generation = generation;
            }
            Edit::MoveGeneration(generation) => settings.update_moves_to_generation = generation,
            Edit::Starter(slot, value) => {
                if let Some(starter) = settings.custom_starters.get_mut(slot) {
                    *starter = value;
                }
            }
            Edit::TrainerMode(mode) => settings.trainers_mod = mode,
            Edit::MiscTweak(tweak, enabled) => {
                settings.current_misc_tweaks = if enabled {
                    settings.current_misc_tweaks | tweak.value()
                } else {
                    settings.current_misc_tweaks & !tweak.value()
                };
            }
        }
    }
}
