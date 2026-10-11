#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum BundleId {
    Masterdatas,
    PersonalMasterdatas,
    Gamesettings,
    EvScript,
    UgData,
    English,
    GlobalGameManagers,
}

impl BundleId {
    pub const ALL: [BundleId; 7] = [
        BundleId::Masterdatas,
        BundleId::PersonalMasterdatas,
        BundleId::Gamesettings,
        BundleId::EvScript,
        BundleId::UgData,
        BundleId::English,
        BundleId::GlobalGameManagers,
    ];

    pub fn relative_path(self) -> &'static str {
        match self {
            BundleId::Masterdatas => "StreamingAssets/AssetAssistant/Dpr/masterdatas",
            BundleId::PersonalMasterdatas => {
                "StreamingAssets/AssetAssistant/Pml/personal_masterdatas"
            }
            BundleId::Gamesettings => {
                "StreamingAssets/AssetAssistant/Dpr/scriptableobjects/gamesettings"
            }
            BundleId::EvScript => "StreamingAssets/AssetAssistant/Dpr/ev_script",
            BundleId::UgData => "StreamingAssets/AssetAssistant/UnderGround/data/ugdata",
            BundleId::English => "StreamingAssets/AssetAssistant/Message/english",
            BundleId::GlobalGameManagers => "globalgamemanagers",
        }
    }

    pub fn is_required(self) -> bool {
        !matches!(self, BundleId::GlobalGameManagers)
    }
}

pub mod path_ids {
    pub const TRAINER_TABLE: i64 = 676_024_375_065_692_598;
    pub const POKEMON_INFO: i64 = 3_987_417_216_388_098_002;
    pub const SHOP_TABLE: i64 = 5_586_546_078_177_307_292;
    pub const WAZA_TABLE: i64 = -8_825_204_264_973_564_398;
    pub const ITEM_TABLE: i64 = 252_928_009_371_549_925;
    pub const WAZA_OBOE_TABLE: i64 = 1_345_203_096_983_357_567;
    pub const EVOLVE_TABLE: i64 = 5_139_195_221_601_552_760;
    pub const PERSONAL_TABLE: i64 = 6_925_071_152_922_426_992;
    pub const FIELD_ENCOUNT_TABLE_D: i64 = 361_824_127_573_837_173;
    pub const FIELD_ENCOUNT_TABLE_P: i64 = -9_035_030_829_162_387_677;
    pub const MAP_INFO: i64 = -1_288_842_406_239_278_419;
    pub const FLD_ITEM: i64 = -4_074_276_362_271_022_109;
    pub const ENGLISH_SCENARIO1: i64 = -5_307_844_841_844_767_521;
    pub const TIME_MANAGER: i64 = 8;
    pub const QUALITY_SETTINGS: i64 = 12;
    pub const UG_ENCOUNT_TABLES: [i64; 11] = [
        2_921_998_521_783_056_987,
        3_120_829_428_501_700_040,
        8_994_974_075_820_978_024,
        554_439_615_776_598_099,
        1_749_855_953_384_930_022,
        -233_820_855_963_829_723,
        -8_064_315_274_480_865_434,
        6_067_500_095_907_821_527,
        -6_533_677_965_289_877_687,
        -117_713_607_888_148_647,
        -4_148_679_105_701_947_902,
    ];
}

pub const SPECIES_NAMES: &str = include_str!("../assets/pokemon.txt");
pub const SPECIES_CATEGORIES: &str = include_str!("../assets/categories.txt");
pub const BODY_SIZES: &str = include_str!("../assets/pokeBodySize.txt");
pub const FAST_RUNNERS: &str = include_str!("../assets/runSpeed5.txt");
