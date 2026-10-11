use crate::constants as c;
use crate::handler::Gen3RomHandler;

const VANILLA_FIRE_RED_10_CRC32: i64 = 3_716_707_868;
const JAMBO_HACK_SIGNATURE_OFFSET: i32 = 0x3EB20;
const JAMBO_HACK_SIGNATURE: i32 = 0x4708_4918;
const JAMBO_FIRST_ROUTINE_POINTER: i32 = 0x3EB84;
const JAMBO_MOVESETS_POINTER_DISTANCE: i32 = 75;
const MOVESETS_POINTER: i32 = 0x3EA7C;
const TMHM_COMPAT_POINTER: i32 = 0x43C68;
const EVOLUTIONS_POINTER: i32 = 0x42F6C;
const TUTOR_COMPAT_POINTER: i32 = 0x120C30;
const MOVE_DESCRIPTIONS_POINTER: i32 = 0xE5440;
const TRAINERS_POINTER: i32 = 0xFC00;
const LARGEST_HACK_POKEDEX_ENTRY: i32 = 1023;
const LONGEST_SANE_DESCRIPTION: i32 = 100;
const LARGEST_SCANNED_ENTRIES: i32 = 4096;
const TRAINER_POKEMON_TYPES: i32 = 4;
const LARGEST_TRAINER_TEAM: i32 = 6;

impl Gen3RomHandler {
    fn basic_bpre10_hack_detection(&self) -> bool {
        self.rom.len() as usize != c::SIZE_16M || self.actual_crc32 != VANILLA_FIRE_RED_10_CRC32
    }

    pub(crate) fn basic_bpre10_hack_support(&mut self) {
        if !self.basic_bpre10_hack_detection() {
            return;
        }
        self.is_rom_hack = true;
        let mut pokemon_count = self.count_hack_pokemon_names();
        let movesets = self.locate_hack_movesets();
        self.entry.set_value("PokemonMovesets", movesets);
        while pokemon_count >= 0
            && self
                .rom
                .valid_pointer(movesets + pokemon_count * 4)
                .is_none()
        {
            pokemon_count -= 1;
        }
        let order = self.value("PokedexOrder");
        for index in 1..=pokemon_count {
            if self.rom.word(order + (index - 1) * 2) > LARGEST_HACK_POKEDEX_ENTRY {
                pokemon_count = index - 1;
                break;
            }
        }
        self.entry.set_value("PokemonCount", pokemon_count);
        let pointers = [
            ("PokemonTMHMCompat", TMHM_COMPAT_POINTER),
            ("PokemonEvolutions", EVOLUTIONS_POINTER),
            ("MoveTutorCompatibility", TUTOR_COMPAT_POINTER),
            ("MoveDescriptions", MOVE_DESCRIPTIONS_POINTER),
            ("TrainerData", TRAINERS_POINTER),
        ];
        for (key, location) in pointers {
            let pointer = self.rom.pointer(location);
            self.entry.set_value(key, pointer);
        }
        let move_count = self.count_hack_moves();
        self.entry.set_value("MoveCount", move_count);
        let trainer_count = self.count_hack_trainers();
        self.entry.set_value("TrainerCount", trainer_count);
    }

    fn count_hack_pokemon_names(&self) -> i32 {
        let names = self.value("PokemonNames");
        let name_length = self.value("PokemonNameLength");
        let mut count = 0;
        while count < LARGEST_SCANNED_ENTRIES {
            let offset = names + (count + 1) * name_length;
            let length = self.string_length_at(offset);
            if length > 0 && length < name_length && self.rom.byte(offset) != 0 {
                count += 1;
            } else {
                break;
            }
        }
        let last_name = self.read_variable_string(names + count * name_length);
        if last_name == "?" || last_name == "-" {
            count -= 1;
        }
        count
    }

    fn locate_hack_movesets(&mut self) -> i32 {
        if self.rom.long(JAMBO_HACK_SIGNATURE_OFFSET) == JAMBO_HACK_SIGNATURE {
            let routine = self.rom.pointer(JAMBO_FIRST_ROUTINE_POINTER);
            self.jambo_moveset_hack = true;
            self.rom.pointer(routine + JAMBO_MOVESETS_POINTER_DISTANCE)
        } else {
            self.jambo_moveset_hack = false;
            self.rom.pointer(MOVESETS_POINTER)
        }
    }

    fn count_hack_moves(&self) -> i32 {
        let descriptions = self.value("MoveDescriptions");
        let mut count = 0;
        while count < LARGEST_SCANNED_ENTRIES {
            let Some(pointer) = self.rom.valid_pointer(descriptions + count * 4) else {
                break;
            };
            let length = self.string_length_at(pointer);
            if length > 0 && length < LONGEST_SANE_DESCRIPTION {
                count += 1;
            } else {
                break;
            }
        }
        count
    }

    fn count_hack_trainers(&self) -> i32 {
        let trainers = self.value("TrainerData");
        let entry_size = self.value("TrainerEntrySize");
        let name_length = self.value("TrainerNameLength");
        let mut count = 1;
        while count < LARGEST_SCANNED_ENTRIES {
            let offset = trainers + entry_size * count;
            if self.rom.int(offset) >= TRAINER_POKEMON_TYPES {
                break;
            }
            let team_size = self.rom.int(offset + (entry_size - 8));
            if team_size == 0 || team_size > LARGEST_TRAINER_TEAM {
                break;
            }
            if self.rom.valid_pointer(offset + (entry_size - 4)).is_none() {
                break;
            }
            if self.string_length_at(offset + 4) >= name_length {
                break;
            }
            count += 1;
        }
        count
    }
}
