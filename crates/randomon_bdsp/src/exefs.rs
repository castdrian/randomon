use randomon_core::Ips32Patch;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Game {
    BrilliantDiamond,
    ShiningPearl,
}

impl Game {
    pub const ALL: [Game; 2] = [Game::BrilliantDiamond, Game::ShiningPearl];

    pub fn title_id(self) -> &'static str {
        match self {
            Game::BrilliantDiamond => "0100000011D90000",
            Game::ShiningPearl => "010018E011D92000",
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Game::BrilliantDiamond => "Pokémon Brilliant Diamond",
            Game::ShiningPearl => "Pokémon Shining Pearl",
        }
    }

    pub fn rom_name(self) -> &'static str {
        match self {
            Game::BrilliantDiamond => "Pokemon Brilliant Diamond",
            Game::ShiningPearl => "Pokemon Shining Pearl",
        }
    }

    pub fn from_folders(romfs: &std::path::Path) -> Option<Game> {
        romfs.ancestors().find_map(|folder| {
            folder
                .file_name()
                .and_then(|name| name.to_str())
                .and_then(Game::from_title_id)
        })
    }

    pub fn from_title_id(text: &str) -> Option<Game> {
        Game::ALL
            .into_iter()
            .find(|game| game.title_id().eq_ignore_ascii_case(text))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StarterOffsets {
    pub game: Game,
    pub version: &'static str,
    pub build_id: &'static str,
    pub default_poke_no: u32,
    pub rival_poke_no: u32,
}

pub const STARTER_OFFSETS: [StarterOffsets; 14] = [
    StarterOffsets {
        game: Game::BrilliantDiamond,
        version: "1.0.0",
        build_id: "F87FC6075104EC4D9642A4AA6BB22216",
        default_poke_no: 0x1BB_BBB0,
        rival_poke_no: 0x1BB_BF30,
    },
    StarterOffsets {
        game: Game::BrilliantDiamond,
        version: "1.1.0",
        build_id: "EA058A067CBD6943A6CF65B4588B6098",
        default_poke_no: 0x238_C6B0,
        rival_poke_no: 0x238_CA30,
    },
    StarterOffsets {
        game: Game::BrilliantDiamond,
        version: "1.1.1",
        build_id: "D9E96FB92878E3458AAE7E8D31AB32A9",
        default_poke_no: 0x1FE_A960,
        rival_poke_no: 0x1FE_ACE0,
    },
    StarterOffsets {
        game: Game::BrilliantDiamond,
        version: "1.1.2",
        build_id: "1B5215DF918BA04BB3894852387F82FF",
        default_poke_no: 0x238_E470,
        rival_poke_no: 0x238_E7F0,
    },
    StarterOffsets {
        game: Game::BrilliantDiamond,
        version: "1.1.3",
        build_id: "BC259F7EE8E79A4995CDC79E69CAC6CD",
        default_poke_no: 0x237_CCF0,
        rival_poke_no: 0x237_D070,
    },
    StarterOffsets {
        game: Game::BrilliantDiamond,
        version: "1.2.0",
        build_id: "35B9D8779B1951418FB609945CCC8C7E",
        default_poke_no: 0x233_4E30,
        rival_poke_no: 0x233_51B0,
    },
    StarterOffsets {
        game: Game::BrilliantDiamond,
        version: "1.3.0",
        build_id: "94CEAE325C205C4B9D6F7235552F28FD",
        default_poke_no: 0x2CF_13D0,
        rival_poke_no: 0x2CF_1750,
    },
    StarterOffsets {
        game: Game::ShiningPearl,
        version: "1.0.0",
        build_id: "1284AB14C1477243AE2A3550EF828709",
        default_poke_no: 0x1BB_BBE0,
        rival_poke_no: 0x1BB_BF60,
    },
    StarterOffsets {
        game: Game::ShiningPearl,
        version: "1.1.0",
        build_id: "609FAC97880FA04CA6A8626D54A2BAB2",
        default_poke_no: 0x238_C6D0,
        rival_poke_no: 0x238_CA50,
    },
    StarterOffsets {
        game: Game::ShiningPearl,
        version: "1.1.1",
        build_id: "3C70CAE153DF0B4F8A7B24C60FD8D0E7",
        default_poke_no: 0x238_DD00,
        rival_poke_no: 0x238_E080,
    },
    StarterOffsets {
        game: Game::ShiningPearl,
        version: "1.1.2",
        build_id: "5D3A3B56321FFD4CB5B5AEDC62550AFB",
        default_poke_no: 0x238_E490,
        rival_poke_no: 0x238_E810,
    },
    StarterOffsets {
        game: Game::ShiningPearl,
        version: "1.1.3",
        build_id: "046D130F0873314A81D795C157E44A2F",
        default_poke_no: 0x237_CD10,
        rival_poke_no: 0x237_D090,
    },
    StarterOffsets {
        game: Game::ShiningPearl,
        version: "1.2.0",
        build_id: "D75246EC33C2F64BB1D8C893D4823FFC",
        default_poke_no: 0x233_4E50,
        rival_poke_no: 0x233_51D0,
    },
    StarterOffsets {
        game: Game::ShiningPearl,
        version: "1.3.0",
        build_id: "38F59CBDA2EB9C44B72F94C4D25935A2",
        default_poke_no: 0x238_C8E0,
        rival_poke_no: 0x238_CC60,
    },
];

const NSO_HEADER_SHIFT: u32 = 0x100;
const FIRST_STARTER_OFFSET: u32 = 0xC8;
const SECOND_STARTER_DELTA: u32 = 0x8C;
const THIRD_STARTER_DELTA: u32 = 0x4;

pub fn movz_w(register: u8, immediate: u16) -> [u8; 4] {
    let instruction = 0x5280_0000u32 | (u32::from(immediate) << 5) | u32::from(register & 0x1f);
    instruction.to_le_bytes()
}

pub fn starter_patch(offsets: &StarterOffsets, starters: [u16; 3]) -> Ips32Patch {
    let [grass, fire, water] = starters;
    let player_first = offsets.default_poke_no + FIRST_STARTER_OFFSET;
    let player_second = player_first + SECOND_STARTER_DELTA;
    let player_third = player_second + THIRD_STARTER_DELTA;
    let rival_first = offsets.rival_poke_no + FIRST_STARTER_OFFSET;
    let rival_second = rival_first + SECOND_STARTER_DELTA;
    let rival_third = rival_second + THIRD_STARTER_DELTA;
    let mut patch = Ips32Patch::default();
    patch.add(player_first + NSO_HEADER_SHIFT, &movz_w(0, grass));
    patch.add(player_second + NSO_HEADER_SHIFT, &movz_w(8, fire));
    patch.add(player_third + NSO_HEADER_SHIFT, &movz_w(9, water));
    patch.add(rival_first + NSO_HEADER_SHIFT, &movz_w(0, fire));
    patch.add(rival_second + NSO_HEADER_SHIFT, &movz_w(8, water));
    patch.add(rival_third + NSO_HEADER_SHIFT, &movz_w(8, grass));
    patch
}

pub fn starter_patches(starters: [u16; 3]) -> Vec<(String, Vec<u8>)> {
    STARTER_OFFSETS
        .iter()
        .map(|offsets| {
            (
                offsets.build_id.to_owned(),
                starter_patch(offsets, starters).to_bytes(),
            )
        })
        .collect()
}
