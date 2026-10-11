use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;

pub const SETTINGS_PREFIX: &str = "BDSP";
pub const SETTINGS_VERSION: u8 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StarterMode {
    Unchanged,
    #[default]
    ThreeStageBasics,
    AnyBasic,
    Any,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WildMode {
    Unchanged,
    PerSlot,
    #[default]
    AreaMapping,
    GlobalMapping,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TrainerMode {
    Unchanged,
    Random,
    #[default]
    SimilarStrength,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LearnsetMode {
    #[default]
    Unchanged,
    Random,
    RandomPreferSameType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TmCompatibilityMode {
    #[default]
    Unchanged,
    Random,
    TypeAware,
    Full,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EvolutionMode {
    #[default]
    Unchanged,
    Random,
    SimilarStrength,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BdspSettings {
    pub starters: StarterMode,
    pub starters_distinct_types: bool,
    pub wild: WildMode,
    pub wild_similar_strength: bool,
    pub wild_keep_legendaries: bool,
    pub wild_allow_legendaries: bool,
    pub wild_generations: [bool; 4],
    pub safari: bool,
    pub underground: bool,
    pub wild_level_percent: i32,
    pub trainers: TrainerMode,
    pub trainer_type_themes: bool,
    pub trainer_rival_carries_starter: bool,
    pub trainer_max_ivs: bool,
    pub trainer_level_percent: i32,
    pub learnsets: LearnsetMode,
    pub tms: bool,
    pub tm_compatibility: TmCompatibilityMode,
    pub abilities: bool,
    pub abilities_follow_evolutions: bool,
    pub evolutions: EvolutionMode,
    pub field_items: bool,
    pub shops: bool,
    pub shops_keep_basics: bool,
    pub hgss_following: bool,
    pub true_size: bool,
    pub remove_weather: bool,
    pub bike_everywhere: bool,
    pub sixty_fps: bool,
    pub speed_multiplier_percent: i32,
}

impl Default for BdspSettings {
    fn default() -> Self {
        Self {
            starters: StarterMode::ThreeStageBasics,
            starters_distinct_types: true,
            wild: WildMode::AreaMapping,
            wild_similar_strength: true,
            wild_keep_legendaries: true,
            wild_allow_legendaries: false,
            wild_generations: [true; 4],
            safari: true,
            underground: true,
            wild_level_percent: 0,
            trainers: TrainerMode::SimilarStrength,
            trainer_type_themes: true,
            trainer_rival_carries_starter: true,
            trainer_max_ivs: false,
            trainer_level_percent: 0,
            learnsets: LearnsetMode::Unchanged,
            tms: false,
            tm_compatibility: TmCompatibilityMode::Unchanged,
            abilities: false,
            abilities_follow_evolutions: true,
            evolutions: EvolutionMode::Unchanged,
            field_items: true,
            shops: false,
            shops_keep_basics: true,
            hgss_following: false,
            true_size: false,
            remove_weather: false,
            bike_everywhere: false,
            sixty_fps: false,
            speed_multiplier_percent: 100,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SettingsError {
    #[error("not a BDSP settings string")]
    WrongPrefix,
    #[error("BDSP settings version {0} is not supported")]
    UnsupportedVersion(u8),
    #[error("BDSP settings string is corrupt: {0}")]
    Corrupt(String),
}

pub const LEVEL_PERCENT_RANGE: std::ops::RangeInclusive<i32> = -50..=500;
pub const SPEED_PERCENT_RANGE: std::ops::RangeInclusive<i32> = 10..=1000;

fn mode_index<T: PartialEq + Copy>(all: &[T], value: T) -> u8 {
    all.iter().position(|&item| item == value).unwrap_or(0) as u8
}

fn mode_from<T: Copy>(all: &[T], index: u8) -> Result<T, SettingsError> {
    all.get(usize::from(index))
        .copied()
        .ok_or_else(|| SettingsError::Corrupt(format!("unknown option value {index}")))
}

pub const STARTER_MODES: [StarterMode; 4] = [
    StarterMode::Unchanged,
    StarterMode::ThreeStageBasics,
    StarterMode::AnyBasic,
    StarterMode::Any,
];
pub const WILD_MODES: [WildMode; 4] = [
    WildMode::Unchanged,
    WildMode::PerSlot,
    WildMode::AreaMapping,
    WildMode::GlobalMapping,
];
pub const TRAINER_MODES: [TrainerMode; 3] = [
    TrainerMode::Unchanged,
    TrainerMode::Random,
    TrainerMode::SimilarStrength,
];
pub const LEARNSET_MODES: [LearnsetMode; 3] = [
    LearnsetMode::Unchanged,
    LearnsetMode::Random,
    LearnsetMode::RandomPreferSameType,
];
pub const TM_COMPATIBILITY_MODES: [TmCompatibilityMode; 4] = [
    TmCompatibilityMode::Unchanged,
    TmCompatibilityMode::Random,
    TmCompatibilityMode::TypeAware,
    TmCompatibilityMode::Full,
];
pub const EVOLUTION_MODES: [EvolutionMode; 3] = [
    EvolutionMode::Unchanged,
    EvolutionMode::Random,
    EvolutionMode::SimilarStrength,
];

impl BdspSettings {
    fn flags(&self) -> Vec<bool> {
        let [gen1, gen2, gen3, gen4] = self.wild_generations;
        vec![
            self.starters_distinct_types,
            self.wild_similar_strength,
            self.wild_keep_legendaries,
            self.wild_allow_legendaries,
            gen1,
            gen2,
            gen3,
            gen4,
            self.safari,
            self.underground,
            self.trainer_type_themes,
            self.trainer_rival_carries_starter,
            self.trainer_max_ivs,
            self.tms,
            self.abilities,
            self.abilities_follow_evolutions,
            self.field_items,
            self.shops,
            self.shops_keep_basics,
            self.hgss_following,
            self.true_size,
            self.remove_weather,
            self.bike_everywhere,
            self.sixty_fps,
        ]
    }

    fn apply_flags(&mut self, flags: &[bool]) {
        let flag = |index: usize| flags.get(index).copied().unwrap_or(false);
        self.starters_distinct_types = flag(0);
        self.wild_similar_strength = flag(1);
        self.wild_keep_legendaries = flag(2);
        self.wild_allow_legendaries = flag(3);
        self.wild_generations = [flag(4), flag(5), flag(6), flag(7)];
        self.safari = flag(8);
        self.underground = flag(9);
        self.trainer_type_themes = flag(10);
        self.trainer_rival_carries_starter = flag(11);
        self.trainer_max_ivs = flag(12);
        self.tms = flag(13);
        self.abilities = flag(14);
        self.abilities_follow_evolutions = flag(15);
        self.field_items = flag(16);
        self.shops = flag(17);
        self.shops_keep_basics = flag(18);
        self.hgss_following = flag(19);
        self.true_size = flag(20);
        self.remove_weather = flag(21);
        self.bike_everywhere = flag(22);
        self.sixty_fps = flag(23);
    }

    pub fn to_settings_string(&self) -> String {
        let mut bytes = vec![
            SETTINGS_VERSION,
            mode_index(&STARTER_MODES, self.starters),
            mode_index(&WILD_MODES, self.wild),
            mode_index(&TRAINER_MODES, self.trainers),
            mode_index(&LEARNSET_MODES, self.learnsets),
            mode_index(&TM_COMPATIBILITY_MODES, self.tm_compatibility),
            mode_index(&EVOLUTION_MODES, self.evolutions),
        ];
        bytes.extend_from_slice(&(self.wild_level_percent as i16).to_be_bytes());
        bytes.extend_from_slice(&(self.trainer_level_percent as i16).to_be_bytes());
        bytes.extend_from_slice(&(self.speed_multiplier_percent as i16).to_be_bytes());
        let flags = self.flags();
        bytes.push(flags.len() as u8);
        for chunk in flags.chunks(8) {
            let byte = chunk
                .iter()
                .enumerate()
                .fold(0u8, |byte, (bit, &set)| byte | (u8::from(set) << bit));
            bytes.push(byte);
        }
        let checksum = bytes.iter().fold(0u8, |sum, &byte| sum.wrapping_add(byte));
        bytes.push(checksum);
        format!("{SETTINGS_PREFIX}{}", URL_SAFE_NO_PAD.encode(bytes))
    }

    pub fn from_settings_string(text: &str) -> Result<Self, SettingsError> {
        let encoded = text
            .trim()
            .strip_prefix(SETTINGS_PREFIX)
            .ok_or(SettingsError::WrongPrefix)?;
        let bytes = URL_SAFE_NO_PAD
            .decode(encoded)
            .map_err(|error| SettingsError::Corrupt(error.to_string()))?;
        let (payload, checksum) = bytes
            .split_last()
            .map(|(last, rest)| (rest, *last))
            .ok_or_else(|| SettingsError::Corrupt("empty".into()))?;
        let expected = payload
            .iter()
            .fold(0u8, |sum, &byte| sum.wrapping_add(byte));
        if expected != checksum {
            return Err(SettingsError::Corrupt("checksum mismatch".into()));
        }
        let version = *payload
            .first()
            .ok_or_else(|| SettingsError::Corrupt("empty".into()))?;
        if version != SETTINGS_VERSION {
            return Err(SettingsError::UnsupportedVersion(version));
        }
        if payload.len() < 14 {
            return Err(SettingsError::Corrupt("too short".into()));
        }
        let number =
            |offset: usize| i32::from(i16::from_be_bytes([payload[offset], payload[offset + 1]]));
        let mut settings = Self {
            starters: mode_from(&STARTER_MODES, payload[1])?,
            wild: mode_from(&WILD_MODES, payload[2])?,
            trainers: mode_from(&TRAINER_MODES, payload[3])?,
            learnsets: mode_from(&LEARNSET_MODES, payload[4])?,
            tm_compatibility: mode_from(&TM_COMPATIBILITY_MODES, payload[5])?,
            evolutions: mode_from(&EVOLUTION_MODES, payload[6])?,
            wild_level_percent: number(7),
            trainer_level_percent: number(9),
            speed_multiplier_percent: number(11),
            ..Self::default()
        };
        let flag_count = usize::from(payload[13]);
        let flag_bytes = &payload[14..];
        if flag_bytes.len() != flag_count.div_ceil(8) {
            return Err(SettingsError::Corrupt("flag length mismatch".into()));
        }
        let flags: Vec<bool> = (0..flag_count)
            .map(|index| flag_bytes[index / 8] & (1 << (index % 8)) != 0)
            .collect();
        settings.apply_flags(&flags);
        settings.validate()?;
        Ok(settings)
    }

    pub fn validate(&self) -> Result<(), SettingsError> {
        if !LEVEL_PERCENT_RANGE.contains(&self.wild_level_percent)
            || !LEVEL_PERCENT_RANGE.contains(&self.trainer_level_percent)
        {
            return Err(SettingsError::Corrupt("level change out of range".into()));
        }
        if !SPEED_PERCENT_RANGE.contains(&self.speed_multiplier_percent) {
            return Err(SettingsError::Corrupt(
                "speed multiplier out of range".into(),
            ));
        }
        Ok(())
    }

    pub fn allowed_generations(&self) -> Vec<u8> {
        let chosen: Vec<u8> = (1..=4)
            .filter(|&generation| self.wild_generations[usize::from(generation - 1)])
            .collect();
        if chosen.is_empty() {
            vec![1, 2, 3, 4]
        } else {
            chosen
        }
    }
}

pub trait ModeLabel: Copy + PartialEq + 'static {
    fn label(self) -> &'static str;
}

impl ModeLabel for StarterMode {
    fn label(self) -> &'static str {
        match self {
            StarterMode::Unchanged => "Unchanged",
            StarterMode::ThreeStageBasics => "Random first stages of three-stage lines",
            StarterMode::AnyBasic => "Random first-stage Pokémon",
            StarterMode::Any => "Completely random",
        }
    }
}

impl ModeLabel for WildMode {
    fn label(self) -> &'static str {
        match self {
            WildMode::Unchanged => "Unchanged",
            WildMode::PerSlot => "Random every slot",
            WildMode::AreaMapping => "Random per area (keeps encounter rates)",
            WildMode::GlobalMapping => "Global 1:1 replacement",
        }
    }
}

impl ModeLabel for TrainerMode {
    fn label(self) -> &'static str {
        match self {
            TrainerMode::Unchanged => "Unchanged",
            TrainerMode::Random => "Random",
            TrainerMode::SimilarStrength => "Random with similar strength",
        }
    }
}

impl ModeLabel for LearnsetMode {
    fn label(self) -> &'static str {
        match self {
            LearnsetMode::Unchanged => "Unchanged",
            LearnsetMode::Random => "Random",
            LearnsetMode::RandomPreferSameType => "Random, favouring the Pokémon's types",
        }
    }
}

impl ModeLabel for TmCompatibilityMode {
    fn label(self) -> &'static str {
        match self {
            TmCompatibilityMode::Unchanged => "Unchanged",
            TmCompatibilityMode::Random => "Random (50%)",
            TmCompatibilityMode::TypeAware => "Random, type-aware",
            TmCompatibilityMode::Full => "Every Pokémon learns every TM",
        }
    }
}

impl ModeLabel for EvolutionMode {
    fn label(self) -> &'static str {
        match self {
            EvolutionMode::Unchanged => "Unchanged",
            EvolutionMode::Random => "Random",
            EvolutionMode::SimilarStrength => "Random with similar strength",
        }
    }
}
