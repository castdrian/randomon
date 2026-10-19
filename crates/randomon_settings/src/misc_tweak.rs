pub const NO_MISC_TWEAKS: i32 = 0;

macro_rules! misc_tweaks {
    ($($variant:ident = $bit:literal, $constant:literal, $bundle_key:literal, $name:literal, $priority:literal;)+) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub enum MiscTweak {
            $($variant,)+
        }

        impl MiscTweak {
            pub const ALL: &'static [MiscTweak] = &[$(MiscTweak::$variant,)+];

            pub const fn value(self) -> i32 {
                match self {
                    $(MiscTweak::$variant => 1 << $bit,)+
                }
            }

            pub const fn constant_name(self) -> &'static str {
                match self {
                    $(MiscTweak::$variant => $constant,)+
                }
            }

            pub const fn bundle_key(self) -> &'static str {
                match self {
                    $(MiscTweak::$variant => $bundle_key,)+
                }
            }

            pub const fn name(self) -> &'static str {
                match self {
                    $(MiscTweak::$variant => $name,)+
                }
            }

            pub const fn priority(self) -> i32 {
                match self {
                    $(MiscTweak::$variant => $priority,)+
                }
            }
        }
    };
}

misc_tweaks! {
    BwExpPatch = 0, "BW_EXP_PATCH", "bwPatch", "B/W Exp Patch", 0;
    NerfXAccuracy = 1, "NERF_X_ACCURACY", "nerfXAcc", "Nerf X Accuracy", 0;
    FixCritRate = 2, "FIX_CRIT_RATE", "critRateFix", "\"Fix\" Crit Rate", 0;
    FastestText = 3, "FASTEST_TEXT", "fastestText", "Fastest Text", 0;
    RunningShoesIndoors = 4, "RUNNING_SHOES_INDOORS", "runningShoes", "Running Shoes Indoors", 0;
    RandomizePcPotion = 5, "RANDOMIZE_PC_POTION", "pcPotion", "Randomize PC Potion", 0;
    AllowPikachuEvolution = 6, "ALLOW_PIKACHU_EVOLUTION", "pikachuEvo", "Allow Pikachu Evolution", 0;
    NationalDexAtStart = 7, "NATIONAL_DEX_AT_START", "nationalDex", "Give National Dex at Start", 0;
    UpdateTypeEffectiveness = 8, "UPDATE_TYPE_EFFECTIVENESS", "typeEffectiveness", "Update Type Effectiveness", 0;
    ForceChallengeMode = 9, "FORCE_CHALLENGE_MODE", "forceChallengeMode", "Force Challenge Mode", 0;
    LowerCasePokemonNames = 10, "LOWER_CASE_POKEMON_NAMES", "lowerCaseNames", "Lower Case Pokemon Names", 0;
    RandomizeCatchingTutorial = 11, "RANDOMIZE_CATCHING_TUTORIAL", "catchingTutorial", "Randomize Catching Tutorial", 0;
    BanLuckyEgg = 12, "BAN_LUCKY_EGG", "luckyEgg", "Ban Lucky Egg", 1;
    NoFreeLuckyEgg = 13, "NO_FREE_LUCKY_EGG", "freeLuckyEgg", "No Free Lucky Egg", 0;
    BanBigManiacItems = 14, "BAN_BIG_MANIAC_ITEMS", "maniacItems", "Ban Big Money Maniac Items", 1;
    SosBattlesForAll = 15, "SOS_BATTLES_FOR_ALL", "sosBattles", "All Wild Pokemon Can Call Allies", 0;
    BalanceStaticLevels = 16, "BALANCE_STATIC_LEVELS", "balanceStaticLevels", "Balance Static Pokemon Levels", 0;
    RetainAltFormes = 17, "RETAIN_ALT_FORMES", "retainAltFormes", "Don't Revert Temporary Alt Formes", 0;
    RunWithoutRunningShoes = 18, "RUN_WITHOUT_RUNNING_SHOES", "runWithoutRunningShoes", "Run Without Running Shoes", 0;
    FasterHpAndExpBars = 19, "FASTER_HP_AND_EXP_BARS", "fasterHpAndExpBars", "Faster HP and EXP Bars", 0;
    FastDistortionWorld = 20, "FAST_DISTORTION_WORLD", "fastDistortionWorld", "Fast Distortion World", 0;
    UpdateRotomFormeTyping = 21, "UPDATE_ROTOM_FORME_TYPING", "updateRotomFormeTyping", "Update Rotom Appliance Typings", 0;
    DisableLowHpMusic = 22, "DISABLE_LOW_HP_MUSIC", "disableLowHpMusic", "Disable Low HP Music", 0;
    FollowingPokemonSpeeds = 23, "FOLLOWING_POKEMON_SPEEDS", "followingPokemonSpeeds", "HG/SS Following Pokemon Speeds", 0;
    TrueSizePokemon = 24, "TRUE_SIZE_POKEMON", "trueSizePokemon", "True-Size Pokemon", 0;
    RemoveFogAndWeather = 25, "REMOVE_FOG_AND_WEATHER", "removeFogAndWeather", "Remove Fog and Disruptive Weather", 0;
    BikeEverywhere = 26, "BIKE_EVERYWHERE", "bikeEverywhere", "Bike Everywhere", 0;
    SixtyFps = 27, "SIXTY_FPS", "sixtyFps", "60 FPS", 0;
    FasterGameSpeed = 28, "FASTER_GAME_SPEED", "fasterGameSpeed", "Faster Game Speed", 0;
}

impl MiscTweak {
    pub fn from_value(value: i32) -> Option<Self> {
        Self::ALL
            .iter()
            .copied()
            .find(|tweak| tweak.value() == value)
    }

    pub fn is_set_in(self, mask: i32) -> bool {
        mask & self.value() > 0
    }

    pub fn selected(mask: i32) -> Vec<Self> {
        Self::ALL
            .iter()
            .copied()
            .filter(|tweak| tweak.is_set_in(mask))
            .collect()
    }

    pub fn in_application_order(available_mask: i32, selected_mask: i32) -> Vec<Self> {
        let mut tweaks: Vec<Self> = Self::ALL
            .iter()
            .copied()
            .filter(|tweak| tweak.is_set_in(available_mask) && tweak.is_set_in(selected_mask))
            .collect();
        tweaks.sort_by_key(|tweak| std::cmp::Reverse(tweak.priority()));
        tweaks
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RomTweakSupport {
    Gen1 {
        has_bw_exp_tweak: bool,
        has_x_accuracy_nerf_tweak: bool,
        has_crit_rate_tweak: bool,
        has_text_delay_function: bool,
        has_pc_potion: bool,
        has_pikachu_evolution_jump: bool,
        has_catching_tutorial: bool,
    },
    Gen2 {
        has_bw_exp_tweak: bool,
        has_text_delay_function: bool,
        has_catching_tutorial: bool,
    },
    Gen3 {
        has_run_indoors_tweak: bool,
        has_fastest_text: bool,
        has_catching_tutorial: bool,
        has_pc_potion: bool,
        is_fire_red_leaf_green: bool,
    },
    Gen4 {
        has_fastest_text_tweak: bool,
        has_national_dex_tweak: bool,
        has_fast_distortion_world_tweak: bool,
        is_platinum_or_heart_gold_soul_silver: bool,
    },
    Gen5 {
        has_fastest_text_tweak: bool,
        has_national_dex_tweak: bool,
        is_black_white: bool,
        is_black2_white2: bool,
    },
    Gen6,
    Gen7,
    Bdsp,
}

fn mask_of(entries: &[(MiscTweak, bool)]) -> i32 {
    entries
        .iter()
        .filter(|(_, available)| *available)
        .fold(0, |mask, (tweak, _)| mask | tweak.value())
}

impl RomTweakSupport {
    pub fn available_mask(self) -> i32 {
        use MiscTweak::*;
        match self {
            Self::Gen1 {
                has_bw_exp_tweak,
                has_x_accuracy_nerf_tweak,
                has_crit_rate_tweak,
                has_text_delay_function,
                has_pc_potion,
                has_pikachu_evolution_jump,
                has_catching_tutorial,
            } => mask_of(&[
                (LowerCasePokemonNames, true),
                (UpdateTypeEffectiveness, true),
                (BwExpPatch, has_bw_exp_tweak),
                (NerfXAccuracy, has_x_accuracy_nerf_tweak),
                (FixCritRate, has_crit_rate_tweak),
                (FastestText, has_text_delay_function),
                (RandomizePcPotion, has_pc_potion),
                (AllowPikachuEvolution, has_pikachu_evolution_jump),
                (RandomizeCatchingTutorial, has_catching_tutorial),
            ]),
            Self::Gen2 {
                has_bw_exp_tweak,
                has_text_delay_function,
                has_catching_tutorial,
            } => mask_of(&[
                (LowerCasePokemonNames, true),
                (UpdateTypeEffectiveness, true),
                (BwExpPatch, has_bw_exp_tweak),
                (FastestText, has_text_delay_function),
                (RandomizeCatchingTutorial, has_catching_tutorial),
                (BanLuckyEgg, true),
            ]),
            Self::Gen3 {
                has_run_indoors_tweak,
                has_fastest_text,
                has_catching_tutorial,
                has_pc_potion,
                is_fire_red_leaf_green,
            } => mask_of(&[
                (LowerCasePokemonNames, true),
                (NationalDexAtStart, true),
                (UpdateTypeEffectiveness, true),
                (RunningShoesIndoors, has_run_indoors_tweak),
                (FastestText, has_fastest_text),
                (RandomizeCatchingTutorial, has_catching_tutorial),
                (RandomizePcPotion, has_pc_potion),
                (BanLuckyEgg, true),
                (RunWithoutRunningShoes, true),
                (BalanceStaticLevels, is_fire_red_leaf_green),
            ]),
            Self::Gen4 {
                has_fastest_text_tweak,
                has_national_dex_tweak,
                has_fast_distortion_world_tweak,
                is_platinum_or_heart_gold_soul_silver,
            } => mask_of(&[
                (LowerCasePokemonNames, true),
                (RandomizeCatchingTutorial, true),
                (UpdateTypeEffectiveness, true),
                (FastestText, has_fastest_text_tweak),
                (BanLuckyEgg, true),
                (NationalDexAtStart, has_national_dex_tweak),
                (RunWithoutRunningShoes, true),
                (FasterHpAndExpBars, true),
                (FastDistortionWorld, has_fast_distortion_world_tweak),
                (
                    UpdateRotomFormeTyping,
                    is_platinum_or_heart_gold_soul_silver,
                ),
            ]),
            Self::Gen5 {
                has_fastest_text_tweak,
                has_national_dex_tweak,
                is_black_white,
                is_black2_white2,
            } => mask_of(&[
                (FastestText, has_fastest_text_tweak),
                (BanLuckyEgg, true),
                (NoFreeLuckyEgg, true),
                (BanBigManiacItems, true),
                (UpdateTypeEffectiveness, true),
                (BalanceStaticLevels, is_black_white),
                (NationalDexAtStart, has_national_dex_tweak),
                (RunWithoutRunningShoes, true),
                (ForceChallengeMode, is_black2_white2),
                (DisableLowHpMusic, true),
            ]),
            Self::Gen6 => mask_of(&[
                (FastestText, true),
                (BanLuckyEgg, true),
                (RetainAltFormes, true),
                (NationalDexAtStart, true),
            ]),
            Self::Gen7 => mask_of(&[
                (FastestText, true),
                (BanLuckyEgg, true),
                (SosBattlesForAll, true),
                (RetainAltFormes, true),
            ]),
            Self::Bdsp => mask_of(&[
                (BanLuckyEgg, true),
                (FollowingPokemonSpeeds, true),
                (TrueSizePokemon, true),
                (RemoveFogAndWeather, true),
                (BikeEverywhere, true),
                (SixtyFps, true),
                (FasterGameSpeed, true),
            ]),
        }
    }
}
