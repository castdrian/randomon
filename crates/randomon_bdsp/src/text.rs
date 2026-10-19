use std::collections::BTreeMap;

use randomon_unity::Value;

use crate::assets::BundleId;
use crate::bundles::Bundles;
use crate::error::{Result, data_error};

pub const MOVE_NAMES: &str = "english_ss_wazaname";
pub const ITEM_NAMES: &str = "english_ss_itemname";
pub const ABILITY_NAMES: &str = "english_ss_tokusei";
pub const TRAINER_NAMES: &str = "english_dp_trainers_name";
pub const TRAINER_CLASS_NAMES: &str = "english_dp_trainers_type";
pub const SCENARIO_TABLES: [&str; 3] = [
    "english_dp_scenario1",
    "english_dp_scenario2",
    "english_dp_scenario3",
];

pub struct MessageTable {
    pub path_id: i64,
    pub value: Value,
    pub dirty: bool,
}

impl MessageTable {
    pub fn load_all(bundles: &Bundles, names: &[&str]) -> Result<Vec<Self>> {
        let mut found: BTreeMap<String, (i64, Value)> = bundles
            .monobehaviours_named(BundleId::English, "english_")?
            .into_iter()
            .filter_map(|(path_id, value)| {
                let name = value.field("m_Name").ok()?.as_str()?.to_owned();
                names
                    .contains(&name.as_str())
                    .then_some((name, (path_id, value)))
            })
            .collect();
        names
            .iter()
            .map(|name| {
                let (path_id, value) = found
                    .remove(*name)
                    .ok_or_else(|| data_error(format!("message table {name} not found")))?;
                Ok(Self {
                    path_id,
                    value,
                    dirty: false,
                })
            })
            .collect()
    }

    pub fn has_label(&self, label_name: &str) -> bool {
        self.labels().is_ok_and(|labels| {
            labels.iter().any(|label| {
                label.field("labelName").ok().and_then(Value::as_str) == Some(label_name)
            })
        })
    }

    pub fn text_of(&self, label_name: &str) -> Option<String> {
        self.labels()
            .ok()?
            .iter()
            .find(|label| label.field("labelName").ok().and_then(Value::as_str) == Some(label_name))
            .and_then(|label| label_text(label).ok())
    }

    fn labels(&self) -> Result<&[Value]> {
        self.value
            .field("labelDataArray")?
            .as_array()
            .ok_or_else(|| data_error("labelDataArray is not an array"))
    }

    fn labels_mut(&mut self) -> Result<&mut Vec<Value>> {
        self.value
            .field_mut("labelDataArray")?
            .as_array_mut()
            .ok_or_else(|| data_error("labelDataArray is not an array"))
    }

    pub fn by_index(&self) -> Result<BTreeMap<i32, String>> {
        let mut texts = BTreeMap::new();
        for label in self.labels()? {
            let index = label.int_field("arrayIndex")?;
            texts.insert(index as i32, label_text(label)?);
        }
        Ok(texts)
    }

    pub fn by_label(&self) -> Result<BTreeMap<String, String>> {
        let mut texts = BTreeMap::new();
        for label in self.labels()? {
            let name = label
                .field("labelName")?
                .as_str()
                .unwrap_or_default()
                .to_owned();
            texts.insert(name, label_text(label)?);
        }
        Ok(texts)
    }

    pub fn set_label_text(&mut self, label_name: &str, text: &str) -> Result<()> {
        let label = self
            .labels_mut()?
            .iter_mut()
            .find(|label| label.field("labelName").ok().and_then(Value::as_str) == Some(label_name))
            .ok_or_else(|| data_error(format!("message label {label_name} not found")))?;
        let words = label
            .field_mut("wordDataArray")?
            .as_array_mut()
            .ok_or_else(|| data_error("wordDataArray is not an array"))?;
        let Some(mut word) = words.first().cloned() else {
            return Err(data_error(format!(
                "message label {label_name} has no words"
            )));
        };
        let width = scaled_width(&word, text)?;
        word.field_mut("str")?.set_string(text)?;
        word.field_mut("strWidth")?.set_float(width)?;
        words.clear();
        words.push(word);
        self.dirty = true;
        Ok(())
    }

    pub fn update_label(
        &mut self,
        label_name: &str,
        edit: impl FnOnce(&mut Vec<Value>) -> Result<()>,
    ) -> Result<()> {
        let label = self
            .labels_mut()?
            .iter_mut()
            .find(|label| label.field("labelName").ok().and_then(Value::as_str) == Some(label_name))
            .ok_or_else(|| data_error(format!("message label {label_name} not found")))?;
        let words = label
            .field_mut("wordDataArray")?
            .as_array_mut()
            .ok_or_else(|| data_error("wordDataArray is not an array"))?;
        edit(words)?;
        self.dirty = true;
        Ok(())
    }

    pub fn write_back(&self, bundles: &mut Bundles) -> Result<()> {
        if self.dirty {
            bundles.write(BundleId::English, self.path_id, &self.value)?;
        }
        Ok(())
    }
}

fn label_text(label: &Value) -> Result<String> {
    Ok(label
        .field("wordDataArray")?
        .as_array()
        .unwrap_or_default()
        .iter()
        .filter_map(|word| word.field("str").ok().and_then(Value::as_str))
        .collect())
}

pub fn scaled_width(word: &Value, new_text: &str) -> Result<f64> {
    let old_text = word.field("str")?.as_str().unwrap_or_default();
    let old_width = word.field("strWidth")?.as_f64().unwrap_or(-1.0);
    let old_length = old_text.chars().count();
    if old_width <= 0.0 || old_length == 0 {
        return Ok(old_width);
    }
    Ok(old_width * new_text.chars().count() as f64 / old_length as f64)
}

pub fn set_word(words: &mut [Value], index: usize, text: &str) -> Result<()> {
    let word = words
        .get_mut(index)
        .ok_or_else(|| data_error(format!("message word {index} not found")))?;
    let width = scaled_width(word, text)?;
    word.field_mut("str")?.set_string(text)?;
    word.field_mut("strWidth")?.set_float(width)?;
    Ok(())
}
