const DOUBLE_SIGNIFICAND_WIDTH: i64 = 53;
const DOUBLE_EXP_BIAS: i64 = 1023;
const DOUBLE_EXP_BIT_MASK: i64 = 0x7FF0_0000_0000_0000;
const DOUBLE_SIGNIF_BIT_MASK: i64 = 0x000F_FFFF_FFFF_FFFF;
const FLOAT_SIGNIFICAND_WIDTH: i32 = 24;
const FLOAT_EXP_BIAS: i32 = 127;
const FLOAT_EXP_BIT_MASK: i32 = 0x7F80_0000;
const FLOAT_SIGNIF_BIT_MASK: i32 = 0x007F_FFFF;

pub fn round_double(value: f64) -> i64 {
    let long_bits = value.to_bits() as i64;
    let biased_exp = (long_bits & DOUBLE_EXP_BIT_MASK) >> (DOUBLE_SIGNIFICAND_WIDTH - 1);
    let shift = (DOUBLE_SIGNIFICAND_WIDTH - 2 + DOUBLE_EXP_BIAS) - biased_exp;
    if (shift & -64) == 0 {
        let mut r = (long_bits & DOUBLE_SIGNIF_BIT_MASK) | (DOUBLE_SIGNIF_BIT_MASK + 1);
        if long_bits < 0 {
            r = -r;
        }
        ((r >> shift) + 1) >> 1
    } else {
        double_to_long(value)
    }
}

pub fn round_float(value: f32) -> i32 {
    let int_bits = value.to_bits() as i32;
    let biased_exp = (int_bits & FLOAT_EXP_BIT_MASK) >> (FLOAT_SIGNIFICAND_WIDTH - 1);
    let shift = (FLOAT_SIGNIFICAND_WIDTH - 2 + FLOAT_EXP_BIAS) - biased_exp;
    if (shift & -32) == 0 {
        let mut r = (int_bits & FLOAT_SIGNIF_BIT_MASK) | (FLOAT_SIGNIF_BIT_MASK + 1);
        if int_bits < 0 {
            r = -r;
        }
        ((r >> shift) + 1) >> 1
    } else {
        float_to_int(value)
    }
}

pub fn double_to_int(value: f64) -> i32 {
    value as i32
}

pub fn double_to_long(value: f64) -> i64 {
    value as i64
}

pub fn float_to_int(value: f32) -> i32 {
    value as i32
}

pub fn float_to_long(value: f32) -> i64 {
    value as i64
}

pub fn int_div(dividend: i32, divisor: i32) -> i32 {
    assert!(divisor != 0, "/ by zero");
    dividend.wrapping_div(divisor)
}

pub fn int_rem(dividend: i32, divisor: i32) -> i32 {
    assert!(divisor != 0, "/ by zero");
    dividend.wrapping_rem(divisor)
}

pub fn long_div(dividend: i64, divisor: i64) -> i64 {
    assert!(divisor != 0, "/ by zero");
    dividend.wrapping_div(divisor)
}

pub fn long_rem(dividend: i64, divisor: i64) -> i64 {
    assert!(divisor != 0, "/ by zero");
    dividend.wrapping_rem(divisor)
}

pub fn floor_div(dividend: i32, divisor: i32) -> i32 {
    let quotient = int_div(dividend, divisor);
    if (dividend ^ divisor) < 0 && quotient.wrapping_mul(divisor) != dividend {
        quotient - 1
    } else {
        quotient
    }
}

pub fn floor_mod(dividend: i32, divisor: i32) -> i32 {
    dividend.wrapping_sub(floor_div(dividend, divisor).wrapping_mul(divisor))
}

pub fn int_shift_left(value: i32, distance: i32) -> i32 {
    value.wrapping_shl(distance as u32)
}

pub fn int_shift_right(value: i32, distance: i32) -> i32 {
    value.wrapping_shr(distance as u32)
}

pub fn int_unsigned_shift_right(value: i32, distance: i32) -> i32 {
    (value as u32).wrapping_shr(distance as u32) as i32
}

pub fn long_shift_left(value: i64, distance: i32) -> i64 {
    value.wrapping_shl(distance as u32)
}

pub fn long_shift_right(value: i64, distance: i32) -> i64 {
    value.wrapping_shr(distance as u32)
}

pub fn long_unsigned_shift_right(value: i64, distance: i32) -> i64 {
    (value as u64).wrapping_shr(distance as u32) as i64
}

pub fn int_abs(value: i32) -> i32 {
    value.wrapping_abs()
}

pub fn byte_to_unsigned(value: i8) -> i32 {
    i32::from(value as u8)
}
