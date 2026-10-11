use crate::capabilities::Capabilities;

type Caps = Capabilities;

pub fn generation_3_plus(caps: &Caps) -> bool {
    caps.generation_at_least(3)
}

pub fn trainer_movesets(caps: &Caps) -> bool {
    caps.generation_at_least(2)
}

pub fn trainer_held_items(caps: &Caps) -> bool {
    caps.generation_at_least(2)
}

pub fn additional_trainer_pokemon(_: &Caps) -> bool {
    true
}

pub fn generation_4_plus(caps: &Caps) -> bool {
    caps.generation_at_least(4)
}

pub fn generation_7_plus(caps: &Caps) -> bool {
    caps.generation_at_least(7)
}

pub fn generation_below_7(caps: &Caps) -> bool {
    caps.generation_below(7)
}

pub fn generation_below_8(caps: &Caps) -> bool {
    caps.generation_below(8)
}

pub fn generation_5(caps: &Caps) -> bool {
    caps.generation_is(5)
}

pub fn generation_7(caps: &Caps) -> bool {
    caps.generation_is(7)
}

pub fn not_generation_1(caps: &Caps) -> bool {
    caps.generation_is_not(1)
}

pub fn mega_evolutions(caps: &Caps) -> bool {
    caps.has_mega_evolutions
}

pub fn starter_alt_formes(caps: &Caps) -> bool {
    caps.has_starter_alt_formes
}

pub fn starter_held_items(caps: &Caps) -> bool {
    caps.supports_starter_held_items
}

pub fn third_starter(caps: &Caps) -> bool {
    caps.starter_slots() >= 3
}

pub fn static_pokemon(caps: &Caps) -> bool {
    caps.can_change_static_pokemon
}

pub fn main_game_legendaries(caps: &Caps) -> bool {
    caps.can_change_static_pokemon && caps.has_main_game_legendaries
}

pub fn static_alt_formes(caps: &Caps) -> bool {
    caps.can_change_static_pokemon && caps.has_static_alt_formes
}

pub fn static_mega_swaps(caps: &Caps) -> bool {
    caps.can_change_static_pokemon && caps.generation_is(6) && !caps.force_swap_static_mega_evos
}

pub fn static_music_fix(caps: &Caps) -> bool {
    caps.can_change_static_pokemon && caps.has_static_music_fix
}

pub fn physical_special_split(caps: &Caps) -> bool {
    caps.has_physical_special_split
}

pub fn four_starting_moves(caps: &Caps) -> bool {
    caps.supports_four_starting_moves
}

pub fn functional_formes(caps: &Caps) -> bool {
    caps.has_functional_formes
}

pub fn time_based_encounters(caps: &Caps) -> bool {
    caps.has_time_based_encounters
}

pub fn wild_alt_formes(caps: &Caps) -> bool {
    caps.has_wild_alt_formes
}

pub fn move_tutors(caps: &Caps) -> bool {
    caps.has_move_tutors
}

pub fn shop_randomization(caps: &Caps) -> bool {
    caps.has_shop_randomization
}

pub fn pickup_items(caps: &Caps) -> bool {
    caps.abilities_per_pokemon > 0
}
