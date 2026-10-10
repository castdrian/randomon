#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct GenRestrictions {
    pub allow_gen1: bool,
    pub allow_gen2: bool,
    pub allow_gen3: bool,
    pub allow_gen4: bool,
    pub allow_gen5: bool,
    pub allow_gen6: bool,
    pub allow_gen7: bool,
    pub allow_evolutionary_relatives: bool,
}

#[derive(Clone, Copy)]
enum MegaType {
    Normal,
    Fighting,
    Flying,
    Grass,
    Water,
    Fire,
    Rock,
    Ground,
    Psychic,
    Bug,
    Dragon,
    Electric,
    Ghost,
    Poison,
    Ice,
    Steel,
    Dark,
    Fairy,
}

const ALL_TYPES_MASK: u32 = (1 << 18) - 1;

fn type_mask(types: &[MegaType]) -> u32 {
    types
        .iter()
        .fold(0, |mask, &kind| mask | (1 << kind as u32))
}

impl GenRestrictions {
    pub fn from_state(state: i32) -> Self {
        Self {
            allow_gen1: state & 1 > 0,
            allow_gen2: state & 2 > 0,
            allow_gen3: state & 4 > 0,
            allow_gen4: state & 8 > 0,
            allow_gen5: state & 16 > 0,
            allow_gen6: state & 32 > 0,
            allow_gen7: state & 64 > 0,
            allow_evolutionary_relatives: state & 128 > 0,
        }
    }

    pub fn to_state(self) -> i32 {
        [
            self.allow_gen1,
            self.allow_gen2,
            self.allow_gen3,
            self.allow_gen4,
            self.allow_gen5,
            self.allow_gen6,
            self.allow_gen7,
            self.allow_evolutionary_relatives,
        ]
        .iter()
        .enumerate()
        .fold(0, |state, (bit, &allowed)| {
            state | (i32::from(allowed) << bit)
        })
    }

    pub fn nothing_selected(self) -> bool {
        !(self.allow_gen1
            || self.allow_gen2
            || self.allow_gen3
            || self.allow_gen4
            || self.allow_gen5
            || self.allow_gen6
            || self.allow_gen7)
    }

    pub fn limit_to_gen(&mut self, generation: i32) {
        self.allow_gen2 &= generation >= 2;
        self.allow_gen3 &= generation >= 3;
        self.allow_gen4 &= generation >= 4;
        self.allow_gen5 &= generation >= 5;
        self.allow_gen6 &= generation >= 6;
        self.allow_gen7 &= generation >= 7;
    }

    pub fn allow_trainer_swap_mega_evolvables(
        self,
        is_xy: bool,
        is_type_themed_trainers: bool,
    ) -> bool {
        if is_type_themed_trainers {
            self.mega_evolutions_of_every_type_are_in_pool(is_xy)
        } else {
            self.mega_evolutions_are_in_pool(is_xy)
        }
    }

    pub fn mega_evolutions_of_every_type_are_in_pool(self, is_xy: bool) -> bool {
        use MegaType::*;
        let mut pool = 0;
        if self.allow_gen1 {
            pool |= type_mask(&[
                Grass, Poison, Fire, Flying, Water, Psychic, Ghost, Normal, Bug, Rock,
            ]);
        }
        if self.allow_gen2 {
            pool |= type_mask(&[Electric, Bug, Steel, Fighting, Dark, Fire, Rock]);
            if !is_xy {
                pool |= type_mask(&[Ground]);
            }
        }
        if self.allow_gen3 {
            pool |= type_mask(&[
                Fire, Fighting, Psychic, Fairy, Steel, Rock, Electric, Ghost, Dark, Dragon,
            ]);
            if !is_xy {
                pool |= type_mask(&[Grass, Water, Ground, Flying, Ice]);
            }
        }
        if self.allow_gen4 {
            pool |= type_mask(&[Dragon, Ground, Fighting, Steel, Grass, Ice]);
            if !is_xy {
                pool |= type_mask(&[Normal, Psychic]);
            }
        }
        if self.allow_gen5 && !is_xy {
            pool |= type_mask(&[Normal]);
        }
        if self.allow_gen6 && !is_xy {
            pool |= type_mask(&[Rock, Fairy]);
        }
        pool == ALL_TYPES_MASK
    }

    pub fn mega_evolutions_are_in_pool(self, is_xy: bool) -> bool {
        let early_generations =
            self.allow_gen1 || self.allow_gen2 || self.allow_gen3 || self.allow_gen4;
        if is_xy {
            early_generations
        } else {
            early_generations || self.allow_gen5 || self.allow_gen6
        }
    }
}
