use randomon_settings::{
    AbilitiesMod, AllyPokemonMod, AuraMod, BaseStatisticsMod, EvolutionsMod, ExpCurveMod,
    FieldItemsMod, InGameTradesMod, MoveTutorMovesMod, MoveTutorsCompatibilityMod, MovesetsMod,
    PickupItemsMod, ShopItemsMod, StartersMod, StaticPokemonMod, TmsHmsCompatibilityMod, TmsMod,
    TotemPokemonMod, TypesMod, WildPokemonMod, WildPokemonRestrictionMod,
};

use super::rules as r;
use super::spec::{
    Action, Attachment, Card, Control, Item, Number, NumberFormat, NumberStyle, Pick, Radio, Row,
    Text, never_reset,
};
use super::visibility as v;
use crate::section::Section;

macro_rules! toggle {
    ($name:literal, $field:ident) => {
        toggle!($name, concat!("GUI.", $name, ".toolTipText"), $field)
    };
    ($name:literal, $help:expr, $field:ident) => {
        Item::new(
            Text::Bundle(concat!("GUI.", $name, ".text")),
            Some($help),
            Control::Toggle {
                get: |s| s.$field,
                set: |s, value| s.$field = value,
                attachment: None,
            },
            |s, d| s.$field = d.$field,
        )
    };
}

macro_rules! counted_toggle {
    ($name:literal, $help:expr, $field:ident, $max:expr) => {
        Item::new(
            Text::Bundle(concat!("GUI.", $name, ".text")),
            Some($help),
            Control::Toggle {
                get: |s| s.$field > 0,
                set: |s, value| s.$field = i32::from(value),
                attachment: Some(Attachment::Number(Number {
                    get: |s| s.$field,
                    set: |s, value| s.$field = value,
                    min: 1,
                    max: $max,
                    step: 1,
                    style: NumberStyle::Stepper,
                    format: NumberFormat::Plain,
                    help: None,
                })),
            },
            |s, d| s.$field = d.$field,
        )
    };
}

macro_rules! choice {
    ($name:literal, $field:ident = $value:expr) => {
        choice!(
            $name,
            concat!("GUI.", $name, ".toolTipText"),
            $field = $value
        )
    };
    ($name:literal, $help:expr, $field:ident = $value:expr) => {
        Item::new(
            Text::Bundle(concat!("GUI.", $name, ".text")),
            Some($help),
            Control::Choice {
                get: |s| s.$field == $value,
                select: |s| s.$field = $value,
            },
            never_reset,
        )
    };
}

macro_rules! radio {
    ($field:ident, [$($choice:expr),+ $(,)?]) => {
        Radio::new(&[$($choice),+], |s, d| s.$field = d.$field)
    };
}

macro_rules! number {
    ($field:ident, $min:expr, $max:expr, $step:expr, $format:expr) => {
        number!($field, $min, $max, $step, $format, None)
    };
    ($field:ident, $min:expr, $max:expr, $step:expr, $format:expr, $help:expr) => {
        Attachment::Number(Number {
            get: |s| s.$field,
            set: |s, value| s.$field = value,
            min: $min,
            max: $max,
            step: $step,
            style: NumberStyle::Slider,
            format: $format,
            help: $help,
        })
    };
}

macro_rules! level_modifier {
    ($field:ident) => {
        number!($field, -50, 50, 2, NumberFormat::SignedPercent)
    };
}

macro_rules! good_damaging {
    ($field:ident, $help:literal) => {
        number!(
            $field,
            0,
            100,
            5,
            NumberFormat::Percent,
            Some(concat!("GUI.", $help, ".toolTipText"))
        )
    };
}

const fn pick(label: Text, kind: Pick) -> Item {
    Item::new(label, None, Control::Pick(kind), never_reset)
}

const GENERAL: &[Row] = &[
    Row::Item(Item::new(
        Text::Bundle("GUI.limitPokemonCheckBox.text"),
        Some("GUI.limitPokemonCheckBox.toolTipText"),
        Control::Toggle {
            get: |s| s.limit_pokemon,
            set: |s, value| s.limit_pokemon = value,
            attachment: Some(Attachment::Action(Action::EditGenerationLimits)),
        },
        |s, d| {
            s.limit_pokemon = d.limit_pokemon;
            s.current_restrictions = d.current_restrictions;
        },
    )),
    Row::Item(
        toggle!("noIrregularAltFormesCheckBox", ban_irregular_alt_formes)
            .shown_when(v::generation_4_plus),
    ),
    Row::Item(toggle!("raceModeCheckBox", race_mode)),
];

const BASE_STATISTICS: &[Row] = &[
    Row::Radio(radio!(
        base_statistics_mod,
        [
            choice!(
                "pbsUnchangedRadioButton",
                base_statistics_mod = BaseStatisticsMod::Unchanged
            ),
            choice!(
                "pbsShuffleRadioButton",
                base_statistics_mod = BaseStatisticsMod::Shuffle
            ),
            choice!(
                "pbsRandomRadioButton",
                base_statistics_mod = BaseStatisticsMod::Random
            ),
        ]
    )),
    Row::Item(
        toggle!("pbsFollowEvolutionsCheckBox", base_stats_follow_evolutions)
            .enabled_when(r::base_stats_follow_evolutions),
    ),
    Row::Item(
        toggle!(
            "pbsFollowMegaEvosCheckBox",
            base_stats_follow_mega_evolutions
        )
        .shown_when(v::mega_evolutions)
        .enabled_when(r::base_stats_follow_megas),
    ),
    Row::Item(
        toggle!(
            "pbsAssignEvoStatsRandomlyCheckBox",
            "GUI.pbsAssignEvoStatsRandomlyCheckBox.tooltipText",
            assign_evo_stats_randomly
        )
        .enabled_when(r::assign_evo_stats_randomly),
    ),
    Row::Item(
        toggle!("pbsUpdateBaseStatsCheckBox", update_base_stats)
            .enabled_when(r::update_base_stats)
            .with(Attachment::Pick(Pick::BaseStatGeneration)),
    ),
    Row::Item(
        toggle!("pbsStandardizeEXPCurvesCheckBox", standardize_exp_curves)
            .with(Attachment::Pick(Pick::ExpCurve)),
    ),
    Row::Radio(
        radio!(
            exp_curve_mod,
            [
                choice!(
                    "pbsLegendariesSlowRadioButton",
                    exp_curve_mod = ExpCurveMod::Legendaries
                )
                .enabled_when(r::standardize_exp_curves),
                choice!(
                    "pbsStrongLegendariesSlowRadioButton",
                    exp_curve_mod = ExpCurveMod::StrongLegendaries
                )
                .enabled_when(r::standardize_exp_curves),
                choice!(
                    "pbsAllMediumFastRadioButton",
                    exp_curve_mod = ExpCurveMod::All
                )
                .enabled_when(r::standardize_exp_curves),
            ]
        )
        .nested(),
    ),
];

const TYPES: &[Row] = &[
    Row::Radio(radio!(
        types_mod,
        [
            choice!("ptUnchangedRadioButton", types_mod = TypesMod::Unchanged),
            choice!(
                "ptRandomFollowEvolutionsRadioButton",
                types_mod = TypesMod::RandomFollowEvolutions
            )
            .enabled_when(r::not_every_level),
            choice!(
                "ptRandomCompletelyRadioButton",
                types_mod = TypesMod::CompletelyRandom
            ),
        ]
    )),
    Row::Item(
        toggle!("ptFollowMegaEvosCheckBox", types_follow_mega_evolutions)
            .shown_when(v::mega_evolutions)
            .enabled_when(r::types_follow_megas),
    ),
    Row::Item(
        toggle!(
            "ptForceDualTypeCheckBox",
            "GUI.ptForceDualTypeCheckBox.toolTipText",
            dual_type_only
        )
        .enabled_when(r::types_randomized),
    ),
];

const ABILITIES: &[Row] = &[
    Row::Radio(radio!(
        abilities_mod,
        [
            choice!(
                "paUnchangedRadioButton",
                abilities_mod = AbilitiesMod::Unchanged
            ),
            choice!(
                "paRandomRadioButton",
                abilities_mod = AbilitiesMod::Randomize
            ),
        ]
    )),
    Row::Item(
        toggle!("paAllowWonderGuardCheckBox", allow_wonder_guard)
            .enabled_when(r::abilities_randomized),
    ),
    Row::Item(
        toggle!("paFollowEvolutionsCheckBox", abilities_follow_evolutions)
            .enabled_when(r::abilities_follow_evolutions),
    ),
    Row::Item(
        toggle!("paFollowMegaEvosCheckBox", abilities_follow_mega_evolutions)
            .shown_when(v::mega_evolutions)
            .enabled_when(r::abilities_follow_megas),
    ),
    Row::Item(
        toggle!(
            "paWeighDuplicatesTogetherCheckBox",
            weigh_duplicate_abilities_together
        )
        .enabled_when(r::abilities_randomized),
    ),
    Row::Item(
        toggle!("paEnsureTwoAbilitiesCheckbox", ensure_two_abilities)
            .enabled_when(r::abilities_randomized),
    ),
    Row::caption("GUI.paBanLabel.text"),
    Row::Item(
        toggle!("paTrappingAbilitiesCheckBox", ban_trapping_abilities)
            .enabled_when(r::abilities_randomized)
            .nested(),
    ),
    Row::Item(
        toggle!("paNegativeAbilitiesCheckBox", ban_negative_abilities)
            .enabled_when(r::abilities_randomized)
            .nested(),
    ),
    Row::Item(
        toggle!("paBadAbilitiesCheckBox", ban_bad_abilities)
            .enabled_when(r::abilities_randomized)
            .nested(),
    ),
];

const EVOLUTIONS: &[Row] = &[
    Row::Radio(radio!(
        evolutions_mod,
        [
            choice!(
                "peUnchangedRadioButton",
                evolutions_mod = EvolutionsMod::Unchanged
            ),
            choice!(
                "peRandomRadioButton",
                evolutions_mod = EvolutionsMod::Random
            ),
            choice!(
                "peRandomEveryLevelRadioButton",
                evolutions_mod = EvolutionsMod::RandomEveryLevel
            )
            .shown_when(v::generation_3_plus),
        ]
    )),
    Row::Item(
        toggle!("peSimilarStrengthCheckBox", evos_similar_strength)
            .enabled_when(r::evolutions_random),
    ),
    Row::Item(
        toggle!("peSameTypingCheckBox", evos_same_typing).enabled_when(r::evolutions_randomized),
    ),
    Row::Item(
        toggle!("peLimitEvolutionsToThreeCheckBox", evos_max_three_stages)
            .enabled_when(r::evolutions_random),
    ),
    Row::Item(
        toggle!("peForceChangeCheckBox", evos_force_change).enabled_when(r::evolutions_randomized),
    ),
    Row::Item(
        toggle!("peAllowAltFormesCheckBox", evos_allow_alt_formes)
            .shown_when(v::generation_7_plus)
            .enabled_when(r::evolutions_randomized),
    ),
    Row::Item(
        toggle!(
            "peChangeImpossibleEvosCheckBox",
            change_impossible_evolutions
        )
        .enabled_when(r::not_every_level),
    ),
    Row::Item(
        toggle!("peMakeEvolutionsEasierCheckBox", make_evolutions_easier)
            .enabled_when(r::not_every_level),
    ),
    Row::Item(
        toggle!("peRemoveTimeBasedEvolutions", remove_time_based_evolutions)
            .enabled_when(r::not_every_level),
    ),
];

const STARTERS: &[Row] = &[
    Row::Radio(radio!(
        starters_mod,
        [
            choice!(
                "spUnchangedRadioButton",
                starters_mod = StartersMod::Unchanged
            ),
            choice!("spCustomRadioButton", starters_mod = StartersMod::Custom),
            choice!(
                "spRandomCompletelyRadioButton",
                starters_mod = StartersMod::CompletelyRandom
            ),
            choice!(
                "spRandomTwoEvosRadioButton",
                starters_mod = StartersMod::RandomWithTwoEvolutions
            )
            .enabled_when(r::not_every_level),
        ]
    )),
    Row::Item(
        pick(Text::Plain("Starter 1"), Pick::Starter(0))
            .enabled_when(r::custom_starters)
            .nested(),
    ),
    Row::Item(
        pick(Text::Plain("Starter 2"), Pick::Starter(1))
            .enabled_when(r::custom_starters)
            .nested(),
    ),
    Row::Item(
        pick(Text::Plain("Starter 3"), Pick::Starter(2))
            .shown_when(v::third_starter)
            .enabled_when(r::custom_starters)
            .nested(),
    ),
    Row::Item(
        toggle!(
            "spRandomizeStarterHeldItemsCheckBox",
            randomize_starters_held_items
        )
        .shown_when(v::starter_held_items),
    ),
    Row::Item(
        toggle!("spBanBadItemsCheckBox", ban_bad_random_starter_held_items)
            .shown_when(v::starter_held_items)
            .enabled_when(r::starter_held_items_randomized)
            .nested(),
    ),
    Row::Item(
        toggle!("spAllowAltFormesCheckBox", allow_starter_alt_formes)
            .shown_when(v::starter_alt_formes)
            .enabled_when(r::starters_randomized),
    ),
];

const STATICS: &[Row] = &[
    Row::Radio(radio!(
        static_pokemon_mod,
        [
            choice!(
                "stpUnchangedRadioButton",
                static_pokemon_mod = StaticPokemonMod::Unchanged
            ),
            choice!(
                "stpSwapLegendariesSwapStandardsRadioButton",
                static_pokemon_mod = StaticPokemonMod::RandomMatching
            )
            .shown_when(v::static_pokemon),
            choice!(
                "stpRandomCompletelyRadioButton",
                static_pokemon_mod = StaticPokemonMod::CompletelyRandom
            )
            .shown_when(v::static_pokemon),
            choice!(
                "stpRandomSimilarStrengthRadioButton",
                static_pokemon_mod = StaticPokemonMod::SimilarStrength
            )
            .shown_when(v::static_pokemon),
        ]
    )),
    Row::Item(
        toggle!("stpRandomize600BSTCheckBox", limit600)
            .shown_when(v::static_pokemon)
            .enabled_when(r::statics_randomized),
    ),
    Row::Item(
        toggle!(
            "stpLimitMainGameLegendariesCheckBox",
            limit_main_game_legendaries
        )
        .shown_when(v::main_game_legendaries)
        .enabled_when(r::statics_similar_strength),
    ),
    Row::Item(
        toggle!("stpAllowAltFormesCheckBox", allow_static_alt_formes)
            .shown_when(v::static_alt_formes)
            .enabled_when(r::statics_randomized),
    ),
    Row::Item(
        toggle!("stpSwapMegaEvosCheckBox", swap_static_mega_evos)
            .shown_when(v::static_mega_swaps)
            .enabled_when(r::statics_randomized),
    ),
    Row::Item(
        toggle!("stpFixMusicAllCheckBox", correct_static_music)
            .shown_when(v::static_music_fix)
            .enabled_when(r::statics_randomized),
    ),
    Row::Item(
        toggle!(
            "stpPercentageLevelModifierCheckBox",
            "GUI.stpPercentageLevelModifierCheckBox.tooltipText",
            static_level_modified
        )
        .shown_when(v::static_pokemon)
        .with(level_modifier!(static_level_modifier)),
    ),
];

const TRADES: &[Row] = &[
    Row::Radio(radio!(
        in_game_trades_mod,
        [
            choice!(
                "igtUnchangedRadioButton",
                in_game_trades_mod = InGameTradesMod::Unchanged
            ),
            choice!(
                "igtRandomizeGivenPokemonOnlyRadioButton",
                in_game_trades_mod = InGameTradesMod::RandomizeGiven
            ),
            choice!(
                "igtRandomizeBothRequestedGivenRadioButton",
                in_game_trades_mod = InGameTradesMod::RandomizeGivenAndRequested
            ),
        ]
    )),
    Row::Item(
        toggle!(
            "igtRandomizeNicknamesCheckBox",
            randomize_in_game_trades_nicknames
        )
        .enabled_when(r::trades_randomized),
    ),
    Row::Item(
        toggle!("igtRandomizeOTsCheckBox", randomize_in_game_trades_ots)
            .shown_when(v::not_generation_1)
            .enabled_when(r::trades_randomized),
    ),
    Row::Item(
        toggle!("igtRandomizeIVsCheckBox", randomize_in_game_trades_ivs)
            .shown_when(v::not_generation_1)
            .enabled_when(r::trades_randomized),
    ),
    Row::Item(
        toggle!("igtRandomizeItemsCheckBox", randomize_in_game_trades_items)
            .shown_when(v::not_generation_1)
            .enabled_when(r::trades_randomized),
    ),
];

const MOVE_DATA: &[Row] = &[
    Row::Item(toggle!(
        "mdRandomizeMovePowerCheckBox",
        randomize_move_powers
    )),
    Row::Item(toggle!(
        "mdRandomizeMoveAccuracyCheckBox",
        randomize_move_accuracies
    )),
    Row::Item(toggle!("mdRandomizeMovePPCheckBox", randomize_move_pps)),
    Row::Item(toggle!(
        "mdRandomizeMoveTypesCheckBox",
        randomize_move_types
    )),
    Row::Item(
        toggle!("mdRandomizeMoveCategoryCheckBox", randomize_move_category)
            .shown_when(v::physical_special_split),
    ),
    Row::Item(
        toggle!("mdUpdateMovesCheckBox", update_moves)
            .shown_when(v::generation_below_8)
            .with(Attachment::Pick(Pick::MoveGeneration)),
    ),
];

const MOVESETS: &[Row] = &[
    Row::Radio(radio!(
        movesets_mod,
        [
            choice!(
                "pmsUnchangedRadioButton",
                movesets_mod = MovesetsMod::Unchanged
            ),
            choice!(
                "pmsRandomPreferringSameTypeRadioButton",
                movesets_mod = MovesetsMod::RandomPreferSameType
            ),
            choice!(
                "pmsRandomCompletelyRadioButton",
                movesets_mod = MovesetsMod::CompletelyRandom
            ),
            choice!(
                "pmsMetronomeOnlyModeRadioButton",
                movesets_mod = MovesetsMod::MetronomeOnly
            ),
        ]
    )),
    Row::Item(
        toggle!(
            "pmsGuaranteedLevel1MovesCheckBox",
            start_with_guaranteed_moves
        )
        .shown_when(v::four_starting_moves)
        .enabled_when(r::movesets_randomized)
        .with(number!(
            guaranteed_move_count,
            2,
            4,
            1,
            NumberFormat::Plain,
            Some("GUI.pmsGuaranteedLevel1MovesSlider.toolTipText")
        )),
    ),
    Row::Item(
        toggle!("pmsReorderDamagingMovesCheckBox", reorder_damaging_moves)
            .enabled_when(r::movesets_randomized),
    ),
    Row::Item(
        toggle!("pmsNoGameBreakingMovesCheckBox", block_broken_moveset_moves)
            .enabled_when(r::movesets_randomized),
    ),
    Row::Item(
        toggle!("pmsForceGoodDamagingCheckBox", movesets_force_good_damaging)
            .enabled_when(r::movesets_randomized)
            .with(good_damaging!(
                movesets_good_damaging_percent,
                "pmsForceGoodDamagingSlider"
            )),
    ),
    Row::Item(
        toggle!("pmsEvolutionMovesCheckBox", evolution_moves_for_all)
            .shown_when(v::generation_7_plus)
            .enabled_when(r::movesets_randomized),
    ),
];

const TRAINERS: &[Row] = &[
    Row::Item(pick(Text::Plain("Trainer Pokemon"), Pick::TrainerMode)),
    Row::Item(
        toggle!(
            "tpRivalCarriesStarterCheckBox",
            rival_carries_starter_throughout
        )
        .enabled_when(r::rival_carries_starter),
    ),
    Row::Item(
        toggle!(
            "tpSimilarStrengthCheckBox",
            trainers_use_pokemon_of_similar_strength
        )
        .enabled_when(r::trainers_randomized),
    ),
    Row::Item(
        toggle!("tpWeightTypesCheckBox", trainers_match_typing_distribution)
            .enabled_when(r::trainers_type_themed),
    ),
    Row::Item(
        toggle!("tpDontUseLegendariesCheckBox", trainers_block_legendaries)
            .enabled_when(r::trainers_randomized),
    ),
    Row::Item(
        toggle!(
            "tpNoEarlyWonderGuardCheckBox",
            trainers_block_early_wonder_guard
        )
        .shown_when(v::generation_3_plus)
        .enabled_when(r::trainers_randomized),
    ),
    Row::Item(
        toggle!(
            "tpAllowAlternateFormesCheckBox",
            allow_trainer_alternate_formes
        )
        .shown_when(v::functional_formes)
        .enabled_when(r::trainers_randomized),
    ),
    Row::Item(
        toggle!("tpSwapMegaEvosCheckBox", swap_trainer_mega_evos)
            .shown_when(v::mega_evolutions)
            .enabled_when(r::trainers_swap_megas),
    ),
    Row::Item(
        toggle!("tpRandomShinyTrainerPokemonCheckBox", shiny_chance)
            .shown_when(v::generation_7_plus)
            .enabled_when(r::trainers_randomized),
    ),
    Row::Item(
        toggle!("tpBetterMovesetsCheckBox", better_trainer_movesets)
            .shown_when(v::generation_3_plus)
            .enabled_when(r::better_trainer_movesets),
    ),
    Row::Item(
        toggle!("tpDoubleBattleModeCheckBox", double_battle_mode)
            .shown_when(v::generation_3_plus)
            .enabled_when(r::trainers_randomized),
    ),
    Row::Item(toggle!(
        "tpRandomizeTrainerNamesCheckBox",
        randomize_trainer_names
    )),
    Row::Item(toggle!(
        "tpRandomizeTrainerClassNamesCheckBox",
        randomize_trainer_class_names
    )),
    Row::Item(
        toggle!(
            "tpForceFullyEvolvedAtCheckBox",
            trainers_force_fully_evolved
        )
        .enabled_when(r::not_every_level)
        .with(number!(
            trainers_force_fully_evolved_level,
            30,
            65,
            1,
            NumberFormat::Level,
            Some("GUI.tpForceFullyEvolvedAtSlider.toolTipText")
        )),
    ),
    Row::Item(
        toggle!("tpPercentageLevelModifierCheckBox", trainers_level_modified)
            .with(level_modifier!(trainers_level_modifier)),
    ),
    Row::Item(
        counted_toggle!(
            "tpEliteFourUniquePokemonCheckBox",
            "GUI.tpEliteFourUniquePokemonCheckBox.toolTipText",
            elite_four_unique_pokemon_number,
            2
        )
        .shown_when(v::generation_3_plus)
        .enabled_when(r::trainers_randomized),
    ),
    Row::caption_when("GUI.tpAddMorePokemonForLabel.text", v::generation_3_plus),
    Row::Item(
        counted_toggle!(
            "tpBossTrainersCheckBox",
            "GUI.tpBossTrainersCheckBox.toolTipText",
            additional_boss_trainer_pokemon,
            5
        )
        .shown_when(v::generation_3_plus)
        .enabled_when(r::trainers_randomized)
        .nested(),
    ),
    Row::Item(
        counted_toggle!(
            "tpImportantTrainersCheckBox",
            "GUI.tpImportantTrainersCheckBox.toolTipText",
            additional_important_trainer_pokemon,
            5
        )
        .shown_when(v::generation_3_plus)
        .enabled_when(r::trainers_randomized)
        .nested(),
    ),
    Row::Item(
        counted_toggle!(
            "tpRegularTrainersCheckBox",
            "GUI.tpRegularTrainersCheckBox.toolTipText",
            additional_regular_trainer_pokemon,
            5
        )
        .shown_when(v::generation_3_plus)
        .enabled_when(r::trainers_randomized)
        .nested(),
    ),
    Row::caption_when("GUI.tpHeldItemsLabel.text", v::generation_3_plus),
    Row::Item(
        toggle!(
            "tpBossTrainersItemsCheckBox",
            randomize_held_items_for_boss_trainer_pokemon
        )
        .shown_when(v::generation_3_plus)
        .enabled_when(r::trainers_randomized)
        .nested(),
    ),
    Row::Item(
        toggle!(
            "tpImportantTrainersItemsCheckBox",
            randomize_held_items_for_important_trainer_pokemon
        )
        .shown_when(v::generation_3_plus)
        .enabled_when(r::trainers_randomized)
        .nested(),
    ),
    Row::Item(
        toggle!(
            "tpRegularTrainersItemsCheckBox",
            randomize_held_items_for_regular_trainer_pokemon
        )
        .shown_when(v::generation_3_plus)
        .enabled_when(r::trainers_randomized)
        .nested(),
    ),
    Row::Item(
        toggle!(
            "tpConsumableItemsOnlyCheckBox",
            "GUI.tpConsumableItemsOnlyCheckBox.tooltip",
            consumable_items_only_for_trainer_pokemon
        )
        .shown_when(v::generation_3_plus)
        .enabled_when(r::trainer_held_items_chosen)
        .nested(),
    ),
    Row::Item(
        toggle!(
            "tpSensibleItemsCheckBox",
            "GUI.tpSensibleItemsCheckBox.tooltip",
            sensible_items_only_for_trainer_pokemon
        )
        .shown_when(v::generation_3_plus)
        .enabled_when(r::trainer_held_items_chosen)
        .nested(),
    ),
    Row::Item(
        toggle!(
            "tpHighestLevelGetsItemCheckBox",
            "GUI.tpHighestLevelGetsItemCheckBox.tooltip",
            highest_level_only_gets_items_for_trainer_pokemon
        )
        .shown_when(v::generation_3_plus)
        .enabled_when(r::trainer_held_items_chosen)
        .nested(),
    ),
];

const TOTEMS: &[Row] = &[
    Row::Radio(radio!(
        totem_pokemon_mod,
        [
            choice!(
                "totpUnchangedRadioButton",
                totem_pokemon_mod = TotemPokemonMod::Unchanged
            ),
            choice!(
                "totpRandomRadioButton",
                totem_pokemon_mod = TotemPokemonMod::Random
            ),
            choice!(
                "totpRandomSimilarStrengthRadioButton",
                totem_pokemon_mod = TotemPokemonMod::SimilarStrength
            ),
        ]
    )),
    Row::Radio(
        radio!(
            ally_pokemon_mod,
            [
                choice!(
                    "totpAllyUnchangedRadioButton",
                    ally_pokemon_mod = AllyPokemonMod::Unchanged
                ),
                choice!(
                    "totpAllyRandomRadioButton",
                    ally_pokemon_mod = AllyPokemonMod::Random
                ),
                choice!(
                    "totpAllyRandomSimilarStrengthRadioButton",
                    ally_pokemon_mod = AllyPokemonMod::SimilarStrength
                ),
            ]
        )
        .titled(Text::Bundle("GUI.totpAllyPanel.title")),
    ),
    Row::Radio(
        radio!(
            aura_mod,
            [
                choice!(
                    "totpAuraUnchangedRadioButton",
                    aura_mod = AuraMod::Unchanged
                ),
                choice!(
                    "totpAuraRandomRadioButton",
                    "GUI.totpAuraRandomRadioButton.toolTipText.",
                    aura_mod = AuraMod::Random
                ),
                choice!(
                    "totpAuraRandomSameStrengthRadioButton",
                    aura_mod = AuraMod::SameStrength
                ),
            ]
        )
        .titled(Text::Bundle("GUI.totpAuraPanel.title")),
    ),
    Row::Item(toggle!(
        "totpRandomizeHeldItemsCheckBox",
        randomize_totem_held_items
    )),
    Row::Item(
        toggle!("totpAllowAltFormesCheckBox", allow_totem_alt_formes)
            .enabled_when(r::totem_alt_formes),
    ),
    Row::Item(
        toggle!("totpPercentageLevelModifierCheckBox", totem_levels_modified)
            .with(level_modifier!(totem_level_modifier)),
    ),
];

const WILD: &[Row] = &[
    Row::Radio(radio!(
        wild_pokemon_mod,
        [
            choice!(
                "wpUnchangedRadioButton",
                wild_pokemon_mod = WildPokemonMod::Unchanged
            ),
            choice!(
                "wpRandomRadioButton",
                wild_pokemon_mod = WildPokemonMod::Random
            ),
            choice!(
                "wpArea1To1RadioButton",
                wild_pokemon_mod = WildPokemonMod::AreaMapping
            ),
            choice!(
                "wpGlobal1To1RadioButton",
                wild_pokemon_mod = WildPokemonMod::GlobalMapping
            ),
        ]
    )),
    Row::Radio(
        radio!(
            wild_pokemon_restriction_mod,
            [
                choice!(
                    "wpARNoneRadioButton",
                    wild_pokemon_restriction_mod = WildPokemonRestrictionMod::None
                )
                .enabled_when(r::wild_randomized),
                choice!(
                    "wpARSimilarStrengthRadioButton",
                    wild_pokemon_restriction_mod = WildPokemonRestrictionMod::SimilarStrength
                )
                .enabled_when(r::wild_randomized),
                choice!(
                    "wpARCatchEmAllModeRadioButton",
                    wild_pokemon_restriction_mod = WildPokemonRestrictionMod::CatchEmAll
                )
                .enabled_when(r::wild_area_rules),
                choice!(
                    "wpARTypeThemeAreasRadioButton",
                    wild_pokemon_restriction_mod = WildPokemonRestrictionMod::TypeThemeAreas
                )
                .enabled_when(r::wild_area_rules),
            ]
        )
        .titled(Text::Bundle("GUI.wpARPanel.title")),
    ),
    Row::Item(
        toggle!(
            "wpUseTimeBasedEncountersCheckBox",
            use_time_based_encounters
        )
        .shown_when(v::time_based_encounters)
        .enabled_when(r::wild_randomized),
    ),
    Row::Item(
        toggle!("wpDontUseLegendariesCheckBox", block_wild_legendaries)
            .enabled_when(r::wild_randomized),
    ),
    Row::Item(
        toggle!("wpSetMinimumCatchRateCheckBox", use_minimum_catch_rate).with(number!(
            minimum_catch_rate_level,
            1,
            5,
            1,
            NumberFormat::Plain,
            Some("GUI.wpSetMinimumCatchRateSlider.toolTipText")
        )),
    ),
    Row::Item(
        toggle!(
            "wpRandomizeHeldItemsCheckBox",
            randomize_wild_pokemon_held_items
        )
        .shown_when(v::not_generation_1),
    ),
    Row::Item(
        toggle!(
            "wpBanBadItemsCheckBox",
            ban_bad_random_wild_pokemon_held_items
        )
        .shown_when(v::not_generation_1)
        .enabled_when(r::wild_held_items)
        .nested(),
    ),
    Row::Item(
        toggle!(
            "wpBalanceShakingGrassPokemonCheckBox",
            balance_shaking_grass
        )
        .shown_when(v::generation_5)
        .enabled_when(r::wild_random),
    ),
    Row::Item(
        toggle!("wpPercentageLevelModifierCheckBox", wild_levels_modified)
            .with(level_modifier!(wild_level_modifier)),
    ),
    Row::Item(
        toggle!("wpAllowAltFormesCheckBox", allow_wild_alt_formes)
            .shown_when(v::wild_alt_formes)
            .enabled_when(r::wild_randomized),
    ),
];

const TM_MOVES: &[Row] = &[
    Row::Radio(radio!(
        tms_mod,
        [
            choice!("tmUnchangedRadioButton", tms_mod = TmsMod::Unchanged)
                .enabled_when(r::not_metronome),
            choice!("tmRandomRadioButton", tms_mod = TmsMod::Random).enabled_when(r::not_metronome),
        ]
    )),
    Row::Item(
        toggle!("tmNoGameBreakingMovesCheckBox", block_broken_tm_moves)
            .enabled_when(r::tm_moves_randomized),
    ),
    Row::Item(
        toggle!("tmKeepFieldMoveTMsCheckBox", keep_field_move_tms)
            .enabled_when(r::tm_moves_randomized),
    ),
    Row::Item(
        toggle!("tmForceGoodDamagingCheckBox", tms_force_good_damaging)
            .enabled_when(r::tm_moves_randomized)
            .with(good_damaging!(
                tms_good_damaging_percent,
                "tmForceGoodDamagingSlider"
            )),
    ),
];

const TM_COMPATIBILITY: &[Row] = &[
    Row::Radio(radio!(
        tms_hms_compatibility_mod,
        [
            choice!(
                "thcUnchangedRadioButton",
                tms_hms_compatibility_mod = TmsHmsCompatibilityMod::Unchanged
            ),
            choice!(
                "thcRandomPreferSameTypeRadioButton",
                tms_hms_compatibility_mod = TmsHmsCompatibilityMod::RandomPreferType
            ),
            choice!(
                "thcRandomCompletelyRadioButton",
                tms_hms_compatibility_mod = TmsHmsCompatibilityMod::CompletelyRandom
            ),
            choice!(
                "thcFullCompatibilityRadioButton",
                tms_hms_compatibility_mod = TmsHmsCompatibilityMod::Full
            ),
        ]
    )),
    Row::Item(
        toggle!("tmLevelupMoveSanityCheckBox", tm_level_up_move_sanity)
            .enabled_when(r::tm_level_up_sanity),
    ),
    Row::Item(
        toggle!("tmFollowEvolutionsCheckBox", tms_follow_evolutions)
            .enabled_when(r::tm_follow_evolutions),
    ),
    Row::Item(
        toggle!("tmFullHMCompatibilityCheckBox", full_hm_compat)
            .shown_when(v::generation_below_7)
            .enabled_when(r::full_hm_compatibility),
    ),
];

const TUTOR_MOVES: &[Row] = &[
    Row::Radio(radio!(
        move_tutor_moves_mod,
        [
            choice!(
                "mtUnchangedRadioButton",
                move_tutor_moves_mod = MoveTutorMovesMod::Unchanged
            )
            .enabled_when(r::tutor_moves_radio),
            choice!(
                "mtRandomRadioButton",
                move_tutor_moves_mod = MoveTutorMovesMod::Random
            )
            .enabled_when(r::tutor_moves_radio),
        ]
    )),
    Row::Item(
        toggle!("mtNoGameBreakingMovesCheckBox", block_broken_tutor_moves)
            .enabled_when(r::tutor_moves_randomized),
    ),
    Row::Item(
        toggle!("mtKeepFieldMoveTutorsCheckBox", keep_field_move_tutors)
            .enabled_when(r::tutor_moves_randomized),
    ),
    Row::Item(
        toggle!("mtForceGoodDamagingCheckBox", tutors_force_good_damaging)
            .enabled_when(r::tutor_moves_randomized)
            .with(good_damaging!(
                tutors_good_damaging_percent,
                "mtForceGoodDamagingSlider"
            )),
    ),
];

const TUTOR_COMPATIBILITY: &[Row] = &[
    Row::Radio(radio!(
        move_tutors_compatibility_mod,
        [
            choice!(
                "mtcUnchangedRadioButton",
                move_tutors_compatibility_mod = MoveTutorsCompatibilityMod::Unchanged
            )
            .enabled_when(r::tutor_compatibility_radio),
            choice!(
                "mtcRandomPreferSameTypeRadioButton",
                move_tutors_compatibility_mod = MoveTutorsCompatibilityMod::RandomPreferType
            )
            .enabled_when(r::tutor_compatibility_radio),
            choice!(
                "mtcRandomCompletelyRadioButton",
                move_tutors_compatibility_mod = MoveTutorsCompatibilityMod::CompletelyRandom
            )
            .enabled_when(r::tutor_compatibility_radio),
            choice!(
                "mtcFullCompatibilityRadioButton",
                move_tutors_compatibility_mod = MoveTutorsCompatibilityMod::Full
            )
            .enabled_when(r::tutor_compatibility_radio),
        ]
    )),
    Row::Item(
        toggle!("mtLevelupMoveSanityCheckBox", tutor_level_up_move_sanity)
            .enabled_when(r::tutor_level_up_sanity),
    ),
    Row::Item(
        toggle!("mtFollowEvolutionsCheckBox", tutor_follow_evolutions)
            .enabled_when(r::tutor_follow_evolutions),
    ),
];

const FIELD_ITEMS: &[Row] = &[
    Row::Radio(radio!(
        field_items_mod,
        [
            choice!(
                "fiUnchangedRadioButton",
                field_items_mod = FieldItemsMod::Unchanged
            ),
            choice!(
                "fiShuffleRadioButton",
                field_items_mod = FieldItemsMod::Shuffle
            ),
            choice!(
                "fiRandomRadioButton",
                field_items_mod = FieldItemsMod::Random
            ),
            choice!(
                "fiRandomEvenDistributionRadioButton",
                field_items_mod = FieldItemsMod::RandomEven
            ),
        ]
    )),
    Row::Item(
        toggle!("fiBanBadItemsCheckBox", ban_bad_random_field_items)
            .enabled_when(r::field_items_randomized),
    ),
];

const SHOP_ITEMS: &[Row] = &[
    Row::Radio(radio!(
        shop_items_mod,
        [
            choice!(
                "shUnchangedRadioButton",
                shop_items_mod = ShopItemsMod::Unchanged
            ),
            choice!(
                "shShuffleRadioButton",
                shop_items_mod = ShopItemsMod::Shuffle
            ),
            choice!("shRandomRadioButton", shop_items_mod = ShopItemsMod::Random),
        ]
    )),
    Row::Item(
        toggle!("shBanBadItemsCheckBox", ban_bad_random_shop_items)
            .enabled_when(r::shop_items_random),
    ),
    Row::Item(
        toggle!("shBanRegularShopItemsCheckBox", ban_regular_shop_items)
            .enabled_when(r::shop_items_random),
    ),
    Row::Item(
        toggle!("shBanOverpoweredShopItemsCheckBox", ban_op_shop_items)
            .enabled_when(r::shop_items_random),
    ),
    Row::Item(
        toggle!("shBalanceShopItemPricesCheckBox", balance_shop_prices)
            .enabled_when(r::shop_items_random),
    ),
    Row::Item(
        toggle!(
            "shGuaranteeEvolutionItemsCheckBox",
            guarantee_evolution_items
        )
        .enabled_when(r::shop_items_random),
    ),
    Row::Item(
        toggle!(
            "shGuaranteeXItemsCheckbox",
            "GUI.shGuaranteeXItemsCheckbox.tooltipText",
            guarantee_x_items
        )
        .enabled_when(r::shop_items_random),
    ),
];

const PICKUP_ITEMS: &[Row] = &[
    Row::Radio(radio!(
        pickup_items_mod,
        [
            choice!(
                "puUnchangedRadioButton",
                pickup_items_mod = PickupItemsMod::Unchanged
            ),
            choice!(
                "puRandomRadioButton",
                pickup_items_mod = PickupItemsMod::Random
            ),
        ]
    )),
    Row::Item(
        toggle!("puBanBadItemsCheckBox", ban_bad_random_pickup_items)
            .enabled_when(r::pickup_items_random),
    ),
];

pub static CARDS: &[Card] = &[
    Card::new(
        "general",
        Section::PokemonTraits,
        Text::Bundle("GUI.generalOptionsPanel.title"),
        "Rules that shape the Pokémon pool for every other option.",
        GENERAL,
    ),
    Card::new(
        "base-statistics",
        Section::PokemonTraits,
        Text::Bundle("GUI.pbsPanel.title"),
        "Shuffle or reroll base stats and standardize EXP curves.",
        BASE_STATISTICS,
    ),
    Card::new(
        "types",
        Section::PokemonTraits,
        Text::Bundle("GUI.ptPanel.title"),
        "Give every species new types.",
        TYPES,
    ),
    Card::new(
        "abilities",
        Section::PokemonTraits,
        Text::Bundle("GUI.paPanel.title"),
        "Swap abilities and keep problem abilities out of the pool.",
        ABILITIES,
    )
    .shown_when(v::generation_3_plus),
    Card::new(
        "evolutions",
        Section::PokemonTraits,
        Text::Bundle("GUI.pePanel.title"),
        "Change what each Pokémon evolves into and how.",
        EVOLUTIONS,
    ),
    Card::new(
        "starters",
        Section::StartersStaticsTrades,
        Text::Bundle("GUI.spPanel.title"),
        "Pick or randomize the starter Pokémon.",
        STARTERS,
    ),
    Card::new(
        "statics",
        Section::StartersStaticsTrades,
        Text::Bundle("GUI.stpPanel.title"),
        "Legendaries, gifts and other fixed encounters.",
        STATICS,
    ),
    Card::new(
        "trades",
        Section::StartersStaticsTrades,
        Text::Bundle("GUI.igtPanel.title"),
        "The Pokémon you give and receive in in-game trades.",
        TRADES,
    ),
    Card::new(
        "move-data",
        Section::MovesAndMovesets,
        Text::Bundle("GUI.mdPanel.title"),
        "Power, accuracy, PP, type and category of every move.",
        MOVE_DATA,
    ),
    Card::new(
        "movesets",
        Section::MovesAndMovesets,
        Text::Bundle("GUI.pmsPanel.title"),
        "The moves each Pokémon learns by leveling up.",
        MOVESETS,
    ),
    Card::new(
        "trainers",
        Section::FoePokemon,
        Text::Bundle("GUI.tpPanel.title"),
        "Trainer teams, their strength and their held items.",
        TRAINERS,
    ),
    Card::new(
        "totems",
        Section::FoePokemon,
        Text::Bundle("GUI.totpPanel.title"),
        "Totem Pokémon, their allies and auras.",
        TOTEMS,
    )
    .shown_when(v::generation_7),
    Card::new(
        "wild",
        Section::WildPokemon,
        Text::Bundle("GUI.wpPanel.title"),
        "Encounter tables, catch rates, levels and held items.",
        WILD,
    ),
    Card::new(
        "tm-moves",
        Section::TmsAndTutors,
        Text::Bundle("GUI.tmMovesPanel.title"),
        "Which move each TM teaches.",
        TM_MOVES,
    ),
    Card::new(
        "tm-compatibility",
        Section::TmsAndTutors,
        Text::Bundle("GUI.thcPanel.title"),
        "Which Pokémon can learn each TM and HM.",
        TM_COMPATIBILITY,
    ),
    Card::new(
        "tutor-moves",
        Section::TmsAndTutors,
        Text::Bundle("GUI.mtMovesPanel.title"),
        "Which move each move tutor teaches.",
        TUTOR_MOVES,
    )
    .shown_when(v::move_tutors)
    .unavailable_note("GUI.mtNoExistLabel.text"),
    Card::new(
        "tutor-compatibility",
        Section::TmsAndTutors,
        Text::Bundle("GUI.mtcPanel.title"),
        "Which Pokémon each move tutor will teach.",
        TUTOR_COMPATIBILITY,
    )
    .shown_when(v::move_tutors)
    .unavailable_note("GUI.mtNoExistLabel.text"),
    Card::new(
        "field-items",
        Section::Items,
        Text::Bundle("GUI.fiPanel.title"),
        "Item balls and hidden items around the world.",
        FIELD_ITEMS,
    ),
    Card::new(
        "shop-items",
        Section::Items,
        Text::Bundle("GUI.shPanel.title"),
        "Stock of the special shops.",
        SHOP_ITEMS,
    )
    .shown_when(v::shop_randomization),
    Card::new(
        "pickup-items",
        Section::Items,
        Text::Bundle("GUI.puPanel.title"),
        "Items found by Pokémon with the Pickup ability.",
        PICKUP_ITEMS,
    )
    .shown_when(v::pickup_items),
    Card::new(
        "misc-tweaks",
        Section::MiscTweaks,
        Text::Bundle("GUI.miscPanel.title"),
        "Optional patches and quality-of-life fixes for the game.",
        &[],
    )
    .unavailable_note("GUI.miscNoneAvailableLabel.text")
    .misc_tweaks(),
];

pub fn cards_in(section: Section) -> impl Iterator<Item = &'static Card> {
    CARDS.iter().filter(move |card| card.section == section)
}
