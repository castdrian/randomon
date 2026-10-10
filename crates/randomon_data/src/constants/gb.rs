use crate::types::Type;

pub const MIN_ROM_SIZE: usize = 0x80000;
pub const MAX_ROM_SIZE: usize = 0x200000;
pub const JP_FLAG_OFFSET: usize = 0x14A;
pub const VERSION_OFFSET: usize = 0x14C;
pub const CRC_OFFSET: usize = 0x14E;
pub const ROM_SIG_OFFSET: usize = 0x134;
pub const IS_GBC_OFFSET: usize = 0x143;
pub const ROM_CODE_OFFSET: usize = 0x13F;
pub const STRING_TERMINATOR: u8 = 0x50;
pub const STRING_PRINTED_TEXT_END: u8 = 0x57;
pub const STRING_PRINTED_TEXT_PROMPT_END: u8 = 0x58;
pub const BANK_SIZE: i32 = 0x4000;
pub const Z80_JUMP: u8 = 0xC3;
pub const Z80_NOP: u8 = 0x00;
pub const Z80_XOR_A: u8 = 0xAF;
pub const Z80_LD_A: u8 = 0x3E;
pub const Z80_LD_A_TO_FAR: u8 = 0xEA;
pub const Z80_RET: u8 = 0xC9;
pub const Z80_JUMP_RELATIVE: u8 = 0x18;

pub fn is_physical_type(kind: Option<Type>) -> bool {
    matches!(
        kind,
        Some(
            Type::Normal
                | Type::Fighting
                | Type::Poison
                | Type::Ground
                | Type::Flying
                | Type::Bug
                | Type::Rock
                | Type::Ghost
                | Type::Steel
        )
    )
}
