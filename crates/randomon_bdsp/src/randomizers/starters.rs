use randomon_unity::Value;

use crate::assets::{BundleId, path_ids};
use crate::context::Context;
use crate::data::{GameData, is_legendary};
use crate::error::{Result, data_error};
use crate::exefs::starter_patches;
use crate::settings::StarterMode;

const SECTION: &str = "Starters";
const STARTER_LABELS: [&str; 3] = [
    "8-EV_POKESELECT_02",
    "8-EV_POKESELECT_03",
    "8-EV_POKESELECT_04",
];
const COLOR_WORDS: [usize; 2] = [1, 5];
const CATEGORY_WORD: usize = 2;
const NAME_WORD: usize = 6;
const DRAW_ATTEMPTS: usize = 200;

pub fn dex_color_tag(color: u8) -> &'static str {
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

fn starter_pool(data: &GameData, mode: StarterMode) -> Vec<u16> {
    data.species
        .keys()
        .copied()
        .filter(|&species| !is_legendary(species))
        .filter(|&species| match mode {
            StarterMode::ThreeStageBasics => {
                data.is_basic(species) && data.evolution_depth(species) == 2
            }
            StarterMode::AnyBasic => data.is_basic(species),
            _ => true,
        })
        .collect()
}

fn shares_type(data: &GameData, chosen: &[u16], candidate: u16) -> bool {
    let Some(candidate) = data.species(candidate) else {
        return false;
    };
    chosen.iter().any(|&other| {
        data.species(other)
            .is_some_and(|other| other.types.iter().any(|&kind| candidate.has_type(kind)))
    })
}

fn choose_starters(context: &mut Context<'_>) -> Result<[u16; 3]> {
    let pool = starter_pool(&context.data, context.settings.starters);
    let distinct_types = context.settings.starters_distinct_types;
    let mut chosen: Vec<u16> = Vec::with_capacity(3);
    for _ in 0..3 {
        let mut pick = context.pick(&pool, "starter")?;
        for _ in 0..DRAW_ATTEMPTS {
            let duplicate = chosen.contains(&pick);
            let clash = distinct_types && shares_type(&context.data, &chosen, pick);
            if !duplicate && !clash {
                break;
            }
            pick = context.pick(&pool, "starter")?;
        }
        chosen.push(pick);
    }
    Ok([chosen[0], chosen[1], chosen[2]])
}

fn estimated_width(word: &Value, new_text: &str) -> Result<f64> {
    let old_text = word.field("str")?.as_str().unwrap_or_default();
    let old_width = word.field("strWidth")?.as_f64().unwrap_or(-1.0);
    let old_length = old_text.chars().count();
    if old_width <= 0.0 || old_length == 0 {
        return Ok(old_width);
    }
    Ok(old_width * new_text.chars().count() as f64 / old_length as f64)
}

fn set_word(words: &mut Value, index: usize, text: &str) -> Result<()> {
    let word = words.item_mut(index)?;
    let width = estimated_width(word, text)?;
    word.field_mut("str")?.set_string(text)?;
    word.field_mut("strWidth")?.set_float(width)?;
    Ok(())
}

fn update_selection_text(context: &mut Context<'_>, starters: [u16; 3]) -> Result<()> {
    let texts: Vec<(String, String, String)> = starters
        .iter()
        .map(|&species| {
            let color = context.data.species(species).map_or(255, |info| info.color);
            (
                dex_color_tag(color).to_owned(),
                context.data.names.category(species),
                context.data.names.species(species),
            )
        })
        .collect();
    context
        .bundles
        .update(BundleId::English, path_ids::ENGLISH_SCENARIO1, |table| {
            let labels = table
                .field_mut("labelDataArray")?
                .as_array_mut()
                .ok_or_else(|| data_error("labelDataArray is not an array"))?;
            for (label_name, (color, category, name)) in STARTER_LABELS.iter().zip(&texts) {
                let label = labels
                    .iter_mut()
                    .find(|label| {
                        label.field("labelName").ok().and_then(Value::as_str) == Some(label_name)
                    })
                    .ok_or_else(|| data_error(format!("message label {label_name} not found")))?;
                let words = label.field_mut("wordDataArray")?;
                for index in COLOR_WORDS {
                    set_word(words, index, color)?;
                }
                set_word(words, CATEGORY_WORD, category)?;
                set_word(words, NAME_WORD, name)?;
            }
            Ok(())
        })
}

pub fn randomize(context: &mut Context<'_>) -> Result<()> {
    if context.settings.starters == StarterMode::Unchanged {
        return Ok(());
    }
    let starters = choose_starters(context)?;
    update_selection_text(context, starters)?;
    context.exefs_patches = starter_patches(starters);
    context.starters = Some(starters);
    for (slot, species) in ["Grass slot", "Fire slot", "Water slot"]
        .iter()
        .zip(starters)
    {
        let name = context.data.names.species(species);
        context.log.line(SECTION, format!("{slot}: {name}"));
    }
    Ok(())
}
