pub const PERSONAL: &str = "bin/pml/personal/personal_total.bin";
pub const LEARNSETS: &str = "bin/pml/waza_oboe/wazaoboe_total.bin";
pub const EVOLUTIONS: &str = "bin/pml/evolution";
pub const EGG_MOVES: &str = "bin/pml/tamagowaza";
pub const MOVES: &str = "bin/pml/waza";
pub const ITEMS: &str = "bin/pml/item/item.dat";
pub const ITEM_HASHES: &str = "bin/pml/item/item_hash_to_index.dat";
pub const TRAINER_DATA: &str = "bin/trainer/trainer_data";
pub const TRAINER_POKE: &str = "bin/trainer/trainer_poke";
pub const TRAINER_CLASSES: &str = "bin/trainer/trainer_type";
pub const DATA_TABLE: &str = "bin/archive/field/resident/data_table.gfpak";
pub const PLACEMENT: &str = "bin/archive/field/resident/placement.gfpak";
pub const STATICS: &str = "bin/script_event_data/event_encount_data.bin";
pub const GIFTS: &str = "bin/script_event_data/add_poke.bin";
pub const TRADES: &str = "bin/script_event_data/field_trade.bin";
pub const SHOPS: &str = "bin/appli/shop/bin/shop_data.bin";
pub const UNDERGROUND: &str = "bin/appli/chika/data_table/underground_exploration_poke.bin";
pub const MESSAGES: &str = "bin/message";
pub const DEFAULT_LANGUAGE: &str = "English";

pub const REQUIRED: [&str; 6] = [PERSONAL, LEARNSETS, MOVES, DATA_TABLE, STATICS, GIFTS];

pub fn evolution_file(index: usize) -> String {
    format!("{EVOLUTIONS}/evo_{index:03}.bin")
}

pub fn egg_move_file(index: usize) -> String {
    format!("{EGG_MOVES}/tamagowaza_{index:04}.bin")
}

pub fn move_file(index: usize) -> String {
    format!("{MOVES}/{}", crate::formats::moves::file_name(index))
}

pub fn trainer_data_file(index: usize) -> String {
    format!(
        "{TRAINER_DATA}/{}",
        crate::formats::trainers::data_file_name(index)
    )
}

pub fn trainer_poke_file(index: usize) -> String {
    format!(
        "{TRAINER_POKE}/{}",
        crate::formats::trainers::poke_file_name(index)
    )
}

pub fn trainer_class_file(index: usize) -> String {
    format!(
        "{TRAINER_CLASSES}/{}",
        crate::formats::trainers::class_file_name(index)
    )
}

pub fn common_text_file(language: &str, file_name: &str) -> String {
    format!("{MESSAGES}/{language}/common/{file_name}")
}

pub fn script_text_file(language: &str, file_name: &str) -> String {
    format!("{MESSAGES}/{language}/script/{file_name}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn indexed_paths_are_zero_padded() {
        assert_eq!(evolution_file(1), "bin/pml/evolution/evo_001.bin");
        assert_eq!(evolution_file(1170), "bin/pml/evolution/evo_1170.bin");
        assert_eq!(egg_move_file(7), "bin/pml/tamagowaza/tamagowaza_0007.bin");
        assert_eq!(move_file(33), "bin/pml/waza/waza0033.wazabin");
        assert_eq!(
            trainer_poke_file(5),
            "bin/trainer/trainer_poke/trainer_poke_005.bin"
        );
        assert_eq!(
            common_text_file("English", "monsname.dat"),
            "bin/message/English/common/monsname.dat"
        );
    }
}
