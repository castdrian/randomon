use randomon_data::PokemonId;
use randomon_data::text::java_upper_case;

use crate::constants as c;
use crate::handler::Gen3RomHandler;

const FRLG_STARTER_TEXTS: [(i32, &str); 3] = [
    (c::FRLG_BASE_STARTER1, "you want to go with\\nthe "),
    (c::FRLG_BASE_STARTER2, "you’re claiming the\\n"),
    (c::FRLG_BASE_STARTER3, "you’ve decided on the\\n"),
];

impl Gen3RomHandler {
    fn starter_offsets(&self) -> [i32; 3] {
        let base = self.value("StarterPokemon");
        if self.rom_type().is_frlg() {
            [
                base,
                base + c::FRLG_STARTER2_OFFSET,
                base + c::FRLG_STARTER3_OFFSET,
            ]
        } else {
            [
                base,
                base + c::RSE_STARTER2_OFFSET,
                base + c::RSE_STARTER3_OFFSET,
            ]
        }
    }

    pub(crate) fn read_starters(&self) -> Vec<PokemonId> {
        self.starter_offsets()
            .iter()
            .map(|&offset| self.pokemon_or_fallback(self.rom.word(offset)))
            .collect()
    }

    pub(crate) fn write_starters(&mut self, starters: &[PokemonId]) -> bool {
        let [first, second, third] = starters else {
            return false;
        };
        self.attempt_obedience_evolution_patches();
        let internal = [
            self.internal_of(*first),
            self.internal_of(*second),
            self.internal_of(*third),
        ];
        let offsets = self.starter_offsets();
        if self.rom_type().is_frlg() {
            for index in 0..3 {
                self.rom.set_word(offsets[index], internal[index]);
                self.rom.set_word(
                    offsets[index] + c::FRLG_STARTER_REPEAT_OFFSET,
                    internal[(index + 1) % 3],
                );
            }
            if !matches!(self.entry.region_letter(), Some('J' | 'B')) {
                self.write_frlg_starter_texts(starters);
            }
        } else {
            for (offset, value) in offsets.into_iter().zip(internal) {
                self.rom.set_word(offset, value);
            }
        }
        true
    }

    fn write_frlg_starter_texts(&mut self, starters: &[PokemonId]) {
        for (index, (base_starter, oak_text)) in FRLG_STARTER_TEXTS.into_iter().enumerate() {
            let base = PokemonId::from_index(base_starter as usize);
            if !self.core.data.exists(base) {
                continue;
            }
            let name = java_upper_case(&self.core.data.pk(base).name);
            let needle = self.translate(&name);
            let found = self.rom.search(&needle);
            if let Some(&offset) = found.first() {
                self.write_frlg_starter_text(offset, starters[index], oak_text);
            }
        }
    }

    fn write_frlg_starter_text(&mut self, offset: i32, starter: PokemonId, oak_text: &str) {
        let pk = self.core.data.pk(starter);
        let mut kind = pk
            .primary_type
            .map_or_else(|| "???".to_owned(), |kind| kind.java_name().to_owned());
        if kind == "NORMAL"
            && let Some(secondary) = pk.secondary_type
        {
            kind = secondary.java_name().to_owned();
        }
        let speech = format!(
            "{name} is your choice.\\pSo, \\v01, {oak_text}{kind} POKéMON {name}?",
            name = pk.name
        );
        let length = self.string_length_at(offset) + 1;
        self.write_fixed_string(&speech, offset, length);
    }

    pub(crate) fn read_starter_held_items(&self) -> Vec<i32> {
        if self.rom_type().is_frlg() {
            let base = self.value("StarterPokemon");
            return vec![self.rom.word(base + c::FRLG_STARTER_ITEMS_OFFSET)];
        }
        let base = self.value("StarterItems");
        let low = self.rom.int(base);
        let high = self.rom.int(base + 2);
        if high == 0 {
            vec![low]
        } else {
            vec![high + 0xFF]
        }
    }

    pub(crate) fn write_starter_held_items(&mut self, items: &[i32]) {
        let [item] = items else {
            return;
        };
        let item = *item;
        if self.rom_type().is_frlg() {
            let base = self.value("StarterPokemon");
            self.rom.set_word(base + c::FRLG_STARTER_ITEMS_OFFSET, item);
            return;
        }
        let base = self.value("StarterItems");
        let add_r2 = c::GBA_ADD_RX_OPCODE | c::GBA_R2;
        if item <= 0xFF {
            self.rom.set_byte(base, item as u8);
            self.rom.set_byte(base + 2, 0);
        } else {
            self.rom.set_byte(base, 0xFF);
            self.rom.set_byte(base + 2, (item - 0xFF) as u8);
        }
        self.rom.set_byte(base + 3, add_r2);
    }
}
