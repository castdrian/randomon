use randomon_data::PokemonId;

use crate::assets::{SPECIES_CATEGORIES, SPECIES_NAMES};
use crate::error::Result;
use crate::handler::BdspRomHandler;
use crate::tables::{Table, int};
use crate::text::set_word;

const STARTER_LABELS: [&str; 3] = [
    "8-EV_POKESELECT_02",
    "8-EV_POKESELECT_03",
    "8-EV_POKESELECT_04",
];
const COLOR_WORDS: [usize; 2] = [1, 5];
const CATEGORY_WORD: usize = 2;
const NAME_WORD: usize = 6;

pub fn dex_color_tag(color: i32) -> &'static str {
    match color {
        0 => "<color=#F01E1EFF>",
        1 => "<color=#0064FFFF>",
        2 => "<color=#CB1003FF>",
        3 => "<color=#00FF00FF>",
        4 => "<color=#383431FF>",
        5 => "<color=#8E5B2CFF>",
        6 => "<color=#CC00CCFF>",
        7 => "<color=#669999FF>",
        8 => "<color=#B0B0B0FF>",
        9 => "<color=#FF00FFFF>",
        _ => "<color=#FFFFFFFF>",
    }
}

fn line(text: &str, index: i32) -> String {
    text.lines()
        .nth(index as usize)
        .map(|line| line.trim().to_owned())
        .unwrap_or_default()
}

impl BdspRomHandler {
    fn dex_color(&self, species: i32) -> Result<i32> {
        for row in self.tables.rows(Table::Personal, "Personal")? {
            if int(row, "id")? == species {
                return int(row, "color");
            }
        }
        Ok(-1)
    }

    pub(crate) fn write_starters(&mut self, starters: &[PokemonId]) -> Result<()> {
        let numbers: Vec<i32> = starters
            .iter()
            .take(3)
            .map(|&pokemon| self.core.data.base_number(pokemon))
            .collect();
        let [grass, fire, water] = [numbers[0], numbers[1], numbers[2]];
        self.starters = [grass, fire, water];
        let mut texts = Vec::new();
        for &species in &self.starters {
            let category = line(SPECIES_CATEGORIES, species).replace("Pokemon", "Pokémon");
            texts.push((
                dex_color_tag(self.dex_color(species)?),
                if category.is_empty() {
                    "Mystery Pokémon".to_owned()
                } else {
                    category
                },
                line(SPECIES_NAMES, species),
            ));
        }
        let Some(table) = self.messages.scenario_with_label(STARTER_LABELS[0]) else {
            return Ok(());
        };
        for (label, (color, category, name)) in STARTER_LABELS.iter().zip(texts) {
            table.update_label(label, |words| {
                for index in COLOR_WORDS {
                    set_word(words, index, color)?;
                }
                set_word(words, CATEGORY_WORD, &category)?;
                set_word(words, NAME_WORD, &name)
            })?;
        }
        Ok(())
    }
}
