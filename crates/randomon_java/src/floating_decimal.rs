use std::cmp::Ordering;

const EXP_SHIFT: i32 = 52;
const FRACT_HOB: i64 = 1 << EXP_SHIFT;
const EXP_ONE: i64 = 0x3FF0_0000_0000_0000;
const SIGNIF_BIT_MASK: i64 = 0x000F_FFFF_FFFF_FFFF;
const EXP_BIT_MASK: i64 = 0x7FF0_0000_0000_0000;
const SIGN_BIT_MASK: i64 = i64::MIN;
const EXP_BIAS: i32 = 1023;
#[allow(clippy::approx_constant)]
const ESTIMATE_LOG10_OF_TWO: f64 = 0.301_029_995_663_981;
const MAX_SMALL_BIN_EXP: i32 = 62;
const MIN_SMALL_BIN_EXP: i32 = -(63 / 3);
const SINGLE_EXP_SHIFT: i32 = 23;
const SINGLE_FRACT_HOB: i32 = 1 << SINGLE_EXP_SHIFT;
const SINGLE_EXP_BIAS: i32 = 127;
const SINGLE_SIGNIF_BIT_MASK: i32 = 0x007F_FFFF;
const SINGLE_EXP_BIT_MASK: i32 = 0x7F80_0000;

const INSIGNIFICANT_DIGITS_NUMBER: [i32; 64] = [
    0, 0, 0, 0, 1, 1, 1, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 5, 5, 5, 6, 6, 6, 6, 7, 7, 7, 8, 8, 8, 9, 9,
    9, 9, 10, 10, 10, 11, 11, 11, 12, 12, 12, 12, 13, 13, 13, 14, 14, 14, 15, 15, 15, 15, 16, 16,
    16, 17, 17, 17, 18, 18, 18, 19,
];

const N_5_BITS: [i32; 28] = [
    0, 3, 5, 7, 10, 12, 14, 17, 19, 21, 24, 26, 28, 31, 33, 35, 38, 40, 42, 45, 47, 49, 52, 54, 56,
    59, 61, 63,
];

const fn small_five_powers() -> [i32; 14] {
    let mut powers = [1i32; 14];
    let mut i = 1;
    while i < 14 {
        powers[i] = powers[i - 1] * 5;
        i += 1;
    }
    powers
}

const fn long_five_powers() -> [i64; 27] {
    let mut powers = [1i64; 27];
    let mut i = 1;
    while i < 27 {
        powers[i] = powers[i - 1] * 5;
        i += 1;
    }
    powers
}

const SMALL_5_POW: [i32; 14] = small_five_powers();
const LONG_5_POW: [i64; 27] = long_five_powers();

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecimalDigits {
    pub is_negative: bool,
    pub digits: Vec<u8>,
    pub decimal_exponent: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BinaryToAscii {
    NotANumber,
    Infinite { is_negative: bool },
    Finite(DecimalDigits),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DigitMode {
    Compatible,
    Formatter,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct BigNat {
    limbs: Vec<u32>,
}

impl BigNat {
    fn from_u64(value: u64) -> Self {
        let mut number = Self {
            limbs: vec![value as u32, (value >> 32) as u32],
        };
        number.trim();
        number
    }

    fn trim(&mut self) {
        while self.limbs.last() == Some(&0) {
            self.limbs.pop();
        }
    }

    fn is_zero(&self) -> bool {
        self.limbs.is_empty()
    }

    fn mul_small(&mut self, factor: u32) {
        let mut carry = 0u64;
        for limb in &mut self.limbs {
            let product = u64::from(*limb) * u64::from(factor) + carry;
            *limb = product as u32;
            carry = product >> 32;
        }
        if carry != 0 {
            self.limbs.push(carry as u32);
        }
        self.trim();
    }

    fn mul_pow5(&mut self, exponent: i32) {
        for _ in 0..exponent {
            self.mul_small(5);
        }
    }

    fn shl(&mut self, bits: i32) {
        if self.is_zero() || bits == 0 {
            return;
        }
        let whole = (bits / 32) as usize;
        let partial = (bits % 32) as u32;
        let mut shifted = vec![0u32; whole];
        let mut carry = 0u32;
        for limb in &self.limbs {
            if partial == 0 {
                shifted.push(*limb);
            } else {
                shifted.push((limb << partial) | carry);
                carry = limb >> (32 - partial);
            }
        }
        if carry != 0 {
            shifted.push(carry);
        }
        self.limbs = shifted;
        self.trim();
    }

    fn value_of_mul_pow52(value: u64, p5: i32, p2: i32) -> Self {
        let mut number = Self::from_u64(value);
        number.mul_pow5(p5);
        number.shl(p2);
        number
    }

    fn value_of_pow52(p5: i32, p2: i32) -> Self {
        Self::value_of_mul_pow52(1, p5, p2)
    }

    fn sub_assign(&mut self, other: &Self) {
        let mut borrow = 0i64;
        for (index, limb) in self.limbs.iter_mut().enumerate() {
            let subtrahend = i64::from(other.limbs.get(index).copied().unwrap_or(0)) + borrow;
            let mut difference = i64::from(*limb) - subtrahend;
            borrow = 0;
            if difference < 0 {
                difference += 1 << 32;
                borrow = 1;
            }
            *limb = difference as u32;
        }
        self.trim();
    }

    fn add(&self, other: &Self) -> Self {
        let length = self.limbs.len().max(other.limbs.len());
        let mut limbs = Vec::with_capacity(length + 1);
        let mut carry = 0u64;
        for index in 0..length {
            let sum = u64::from(self.limbs.get(index).copied().unwrap_or(0))
                + u64::from(other.limbs.get(index).copied().unwrap_or(0))
                + carry;
            limbs.push(sum as u32);
            carry = sum >> 32;
        }
        if carry != 0 {
            limbs.push(carry as u32);
        }
        let mut number = Self { limbs };
        number.trim();
        number
    }

    fn quo_rem_iteration(&mut self, divisor: &Self) -> i32 {
        let mut quotient = 0;
        while Ord::cmp(&*self, divisor) != Ordering::Less {
            self.sub_assign(divisor);
            quotient += 1;
        }
        self.mul_small(10);
        quotient
    }
}

impl PartialOrd for BigNat {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for BigNat {
    fn cmp(&self, other: &Self) -> Ordering {
        self.limbs
            .len()
            .cmp(&other.limbs.len())
            .then_with(|| self.limbs.iter().rev().cmp(other.limbs.iter().rev()))
    }
}

struct DigitBuffer {
    digits: Vec<u8>,
    decimal_exponent: i32,
}

impl DigitBuffer {
    fn round_up(&mut self) {
        let mut i = self.digits.len() - 1;
        let mut q = self.digits[i];
        if q == b'9' {
            while q == b'9' && i > 0 {
                self.digits[i] = b'0';
                i -= 1;
                q = self.digits[i];
            }
            if q == b'9' {
                self.decimal_exponent += 1;
                self.digits[0] = b'1';
                return;
            }
        }
        self.digits[i] = q + 1;
    }
}

fn insignificant_digits_for_pow2(p2: i32) -> i32 {
    if p2 > 1 && (p2 as usize) < INSIGNIFICANT_DIGITS_NUMBER.len() {
        INSIGNIFICANT_DIGITS_NUMBER[p2 as usize]
    } else {
        0
    }
}

fn five_bits(power: i32) -> i32 {
    if (power as usize) < N_5_BITS.len() {
        N_5_BITS[power as usize]
    } else {
        power * 3
    }
}

fn estimate_dec_exp(fract_bits: i64, bin_exp: i32) -> i32 {
    let d2 = f64::from_bits((EXP_ONE | (fract_bits & SIGNIF_BIT_MASK)) as u64);
    let d = (d2 - 1.5) * 0.289_529_654 + 0.176_091_259 + bin_exp as f64 * ESTIMATE_LOG10_OF_TWO;
    let d_bits = d.to_bits() as i64;
    let exponent = ((d_bits & EXP_BIT_MASK) >> EXP_SHIFT) as i32 - EXP_BIAS;
    let is_negative = (d_bits & SIGN_BIT_MASK) != 0;
    if (0..52).contains(&exponent) {
        let mask = SIGNIF_BIT_MASK >> exponent;
        let r = (((d_bits & SIGNIF_BIT_MASK) | FRACT_HOB) >> (EXP_SHIFT - exponent)) as i32;
        if is_negative {
            if (mask & d_bits) == 0 { -r } else { -r - 1 }
        } else {
            r
        }
    } else if exponent < 0 {
        if (d_bits & !SIGN_BIT_MASK) == 0 {
            0
        } else if is_negative {
            -1
        } else {
            0
        }
    } else {
        d as i32
    }
}

fn develop_long_digits(mut lvalue: i64, insignificant_digits: i32) -> DigitBuffer {
    let mut dec_exponent = 0;
    if insignificant_digits != 0 {
        let pow10 = LONG_5_POW[insignificant_digits as usize] << insignificant_digits;
        let residue = lvalue % pow10;
        lvalue /= pow10;
        dec_exponent += insignificant_digits;
        if residue >= (pow10 >> 1) {
            lvalue += 1;
        }
    }
    let mut reversed = Vec::with_capacity(20);
    let mut c = lvalue % 10;
    lvalue /= 10;
    while c == 0 {
        dec_exponent += 1;
        c = lvalue % 10;
        lvalue /= 10;
    }
    while lvalue != 0 {
        reversed.push(c as u8 + b'0');
        dec_exponent += 1;
        c = lvalue % 10;
        lvalue /= 10;
    }
    reversed.push(c as u8 + b'0');
    reversed.reverse();
    DigitBuffer {
        digits: reversed,
        decimal_exponent: dec_exponent + 1,
    }
}

struct Scaled {
    fract_bits: i64,
    b2: i32,
    b5: i32,
    s2: i32,
    s5: i32,
    m2: i32,
    m5: i32,
    n_fract_bits: i32,
}

struct DigitLoopResult {
    digits: Vec<u8>,
    dec_exp: i32,
    low: bool,
    high: bool,
    low_digit_difference: i64,
}

fn forces_extra_digit(mode: DigitMode, dec_exp: i32) -> bool {
    mode == DigitMode::Formatter || !(-3..8).contains(&dec_exp)
}

fn int_digit_loop(scaled: &Scaled, mut dec_exp: i32, mode: DigitMode) -> DigitLoopResult {
    let mut b =
        (scaled.fract_bits as i32).wrapping_mul(SMALL_5_POW[scaled.b5 as usize]) << scaled.b2;
    let s = SMALL_5_POW[scaled.s5 as usize] << scaled.s2;
    let mut m = SMALL_5_POW[scaled.m5 as usize] << scaled.m2;
    let tens = s.wrapping_mul(10);
    let mut digits = Vec::with_capacity(20);
    let mut q = b / s;
    b = 10 * (b % s);
    m = m.wrapping_mul(10);
    let mut low = b < m;
    let mut high = b.wrapping_add(m) > tens;
    if q == 0 && !high {
        dec_exp -= 1;
    } else {
        digits.push(b'0' + q as u8);
    }
    if forces_extra_digit(mode, dec_exp) {
        high = false;
        low = false;
    }
    while !low && !high {
        q = b / s;
        b = 10 * (b % s);
        m = m.wrapping_mul(10);
        if m > 0 {
            low = b < m;
            high = b.wrapping_add(m) > tens;
        } else {
            low = true;
            high = true;
        }
        digits.push(b'0' + q as u8);
    }
    DigitLoopResult {
        digits,
        dec_exp,
        low,
        high,
        low_digit_difference: i64::from((b << 1).wrapping_sub(tens)),
    }
}

fn long_digit_loop(scaled: &Scaled, mut dec_exp: i32, mode: DigitMode) -> DigitLoopResult {
    let mut b = scaled
        .fract_bits
        .wrapping_mul(LONG_5_POW[scaled.b5 as usize])
        << scaled.b2;
    let s = LONG_5_POW[scaled.s5 as usize] << scaled.s2;
    let mut m = LONG_5_POW[scaled.m5 as usize] << scaled.m2;
    let tens = s.wrapping_mul(10);
    let mut digits = Vec::with_capacity(20);
    let mut q = (b / s) as i32;
    b = 10 * (b % s);
    m = m.wrapping_mul(10);
    let mut low = b < m;
    let mut high = b.wrapping_add(m) > tens;
    if q == 0 && !high {
        dec_exp -= 1;
    } else {
        digits.push(b'0' + q as u8);
    }
    if forces_extra_digit(mode, dec_exp) {
        high = false;
        low = false;
    }
    while !low && !high {
        q = (b / s) as i32;
        b = 10 * (b % s);
        m = m.wrapping_mul(10);
        if m > 0 {
            low = b < m;
            high = b.wrapping_add(m) > tens;
        } else {
            low = true;
            high = true;
        }
        digits.push(b'0' + q as u8);
    }
    DigitLoopResult {
        digits,
        dec_exp,
        low,
        high,
        low_digit_difference: (b << 1).wrapping_sub(tens),
    }
}

fn big_digit_loop(scaled: &Scaled, mut dec_exp: i32, mode: DigitMode) -> DigitLoopResult {
    let s_val = BigNat::value_of_pow52(scaled.s5, scaled.s2);
    let mut b_val = BigNat::value_of_mul_pow52(scaled.fract_bits as u64, scaled.b5, scaled.b2);
    let mut m_val = BigNat::value_of_pow52(scaled.m5 + 1, scaled.m2 + 1);
    let ten_s_val = BigNat::value_of_pow52(scaled.s5 + 1, scaled.s2 + 1);
    let mut digits = Vec::with_capacity(20);
    let mut q = b_val.quo_rem_iteration(&s_val);
    let mut low = b_val < m_val;
    let mut high = ten_s_val <= b_val.add(&m_val);
    if q == 0 && !high {
        dec_exp -= 1;
    } else {
        digits.push(b'0' + q as u8);
    }
    if forces_extra_digit(mode, dec_exp) {
        high = false;
        low = false;
    }
    while !low && !high {
        q = b_val.quo_rem_iteration(&s_val);
        m_val.mul_small(10);
        low = b_val < m_val;
        high = ten_s_val <= b_val.add(&m_val);
        digits.push(b'0' + q as u8);
    }
    let low_digit_difference = if high && low {
        b_val.shl(1);
        match b_val.cmp(&ten_s_val) {
            Ordering::Less => -1,
            Ordering::Equal => 0,
            Ordering::Greater => 1,
        }
    } else {
        0
    };
    DigitLoopResult {
        digits,
        dec_exp,
        low,
        high,
        low_digit_difference,
    }
}

fn dtoa(bin_exp: i32, fract_bits: i64, n_significant_bits: i32, mode: DigitMode) -> DigitBuffer {
    let tail_zeros = fract_bits.trailing_zeros() as i32;
    let n_fract_bits = EXP_SHIFT + 1 - tail_zeros;
    let n_tiny_bits = (n_fract_bits - bin_exp - 1).max(0);
    if (MIN_SMALL_BIN_EXP..=MAX_SMALL_BIN_EXP).contains(&bin_exp)
        && (n_tiny_bits as usize) < LONG_5_POW.len()
        && n_fract_bits + N_5_BITS[n_tiny_bits as usize] < 64
        && n_tiny_bits == 0
    {
        let insignificant = if bin_exp > n_significant_bits {
            insignificant_digits_for_pow2(bin_exp - n_significant_bits - 1)
        } else {
            0
        };
        let aligned = if bin_exp >= EXP_SHIFT {
            fract_bits << (bin_exp - EXP_SHIFT)
        } else {
            ((fract_bits as u64) >> (EXP_SHIFT - bin_exp)) as i64
        };
        return develop_long_digits(aligned, insignificant);
    }
    let dec_exp = estimate_dec_exp(fract_bits, bin_exp);
    let b5 = (-dec_exp).max(0);
    let mut b2 = b5 + n_tiny_bits + bin_exp;
    let s5 = dec_exp.max(0);
    let mut s2 = s5 + n_tiny_bits;
    let m5 = b5;
    let mut m2 = b2 - n_significant_bits;
    let shifted_fract_bits = ((fract_bits as u64) >> tail_zeros) as i64;
    b2 -= n_fract_bits - 1;
    let common2factor = b2.min(s2);
    b2 -= common2factor;
    s2 -= common2factor;
    m2 -= common2factor;
    if n_fract_bits == 1 {
        m2 -= 1;
    }
    if m2 < 0 {
        b2 -= m2;
        s2 -= m2;
        m2 = 0;
    }
    let scaled = Scaled {
        fract_bits: shifted_fract_bits,
        b2,
        b5,
        s2,
        s5,
        m2,
        m5,
        n_fract_bits,
    };
    let b_bits = scaled.n_fract_bits + b2 + five_bits(b5);
    let ten_s_bits = s2 + 1 + five_bits(s5 + 1);
    let result = if b_bits < 64 && ten_s_bits < 64 {
        if b_bits < 32 && ten_s_bits < 32 {
            int_digit_loop(&scaled, dec_exp, mode)
        } else {
            long_digit_loop(&scaled, dec_exp, mode)
        }
    } else {
        big_digit_loop(&scaled, dec_exp, mode)
    };
    finish_rounding(result)
}

fn finish_rounding(result: DigitLoopResult) -> DigitBuffer {
    let mut buffer = DigitBuffer {
        digits: result.digits,
        decimal_exponent: result.dec_exp + 1,
    };
    let last_digit_is_odd = buffer.digits.last().is_some_and(|digit| digit & 1 != 0);
    let should_round_up = result.high
        && (!result.low
            || result.low_digit_difference > 0
            || (result.low_digit_difference == 0 && last_digit_is_odd));
    if should_round_up {
        buffer.round_up();
    }
    buffer
}

fn finite(is_negative: bool, buffer: DigitBuffer) -> BinaryToAscii {
    BinaryToAscii::Finite(DecimalDigits {
        is_negative,
        digits: buffer.digits,
        decimal_exponent: buffer.decimal_exponent,
    })
}

fn zero(is_negative: bool) -> BinaryToAscii {
    BinaryToAscii::Finite(DecimalDigits {
        is_negative,
        digits: vec![b'0'],
        decimal_exponent: 0,
    })
}

pub fn double_to_decimal(value: f64, mode: DigitMode) -> BinaryToAscii {
    let d_bits = value.to_bits() as i64;
    let is_negative = (d_bits & SIGN_BIT_MASK) != 0;
    let mut fract_bits = d_bits & SIGNIF_BIT_MASK;
    let mut bin_exp = ((d_bits & EXP_BIT_MASK) >> EXP_SHIFT) as i32;
    if bin_exp == (EXP_BIT_MASK >> EXP_SHIFT) as i32 {
        return if fract_bits == 0 {
            BinaryToAscii::Infinite { is_negative }
        } else {
            BinaryToAscii::NotANumber
        };
    }
    let n_significant_bits;
    if bin_exp == 0 {
        if fract_bits == 0 {
            return zero(is_negative);
        }
        let leading_zeros = fract_bits.leading_zeros() as i32;
        let shift = leading_zeros - (63 - EXP_SHIFT);
        fract_bits <<= shift;
        bin_exp = 1 - shift;
        n_significant_bits = 64 - leading_zeros;
    } else {
        fract_bits |= FRACT_HOB;
        n_significant_bits = EXP_SHIFT + 1;
    }
    bin_exp -= EXP_BIAS;
    finite(
        is_negative,
        dtoa(bin_exp, fract_bits, n_significant_bits, mode),
    )
}

pub fn float_to_decimal(value: f32) -> BinaryToAscii {
    let f_bits = value.to_bits() as i32;
    let is_negative = f_bits < 0;
    let mut fract_bits = f_bits & SINGLE_SIGNIF_BIT_MASK;
    let mut bin_exp = (f_bits & SINGLE_EXP_BIT_MASK) >> SINGLE_EXP_SHIFT;
    if bin_exp == SINGLE_EXP_BIT_MASK >> SINGLE_EXP_SHIFT {
        return if fract_bits == 0 {
            BinaryToAscii::Infinite { is_negative }
        } else {
            BinaryToAscii::NotANumber
        };
    }
    let n_significant_bits;
    if bin_exp == 0 {
        if fract_bits == 0 {
            return zero(is_negative);
        }
        let leading_zeros = fract_bits.leading_zeros() as i32;
        let shift = leading_zeros - (31 - SINGLE_EXP_SHIFT);
        fract_bits <<= shift;
        bin_exp = 1 - shift;
        n_significant_bits = 32 - leading_zeros;
    } else {
        fract_bits |= SINGLE_FRACT_HOB;
        n_significant_bits = SINGLE_EXP_SHIFT + 1;
    }
    bin_exp -= SINGLE_EXP_BIAS;
    let widened = i64::from(fract_bits) << (EXP_SHIFT - SINGLE_EXP_SHIFT);
    finite(
        is_negative,
        dtoa(bin_exp, widened, n_significant_bits, DigitMode::Compatible),
    )
}

fn java_format_string(conversion: BinaryToAscii) -> String {
    match conversion {
        BinaryToAscii::NotANumber => "NaN".to_owned(),
        BinaryToAscii::Infinite { is_negative: true } => "-Infinity".to_owned(),
        BinaryToAscii::Infinite { is_negative: false } => "Infinity".to_owned(),
        BinaryToAscii::Finite(decimal) => format_finite(&decimal),
    }
}

fn format_finite(decimal: &DecimalDigits) -> String {
    let digits = &decimal.digits;
    let n_digits = digits.len() as i32;
    let exponent = decimal.decimal_exponent;
    let mut out = String::with_capacity(26);
    if decimal.is_negative {
        out.push('-');
    }
    if exponent > 0 && exponent < 8 {
        let integer_length = n_digits.min(exponent);
        push_digits(&mut out, &digits[..integer_length as usize]);
        if integer_length < exponent {
            out.extend(std::iter::repeat_n(
                '0',
                (exponent - integer_length) as usize,
            ));
            out.push_str(".0");
        } else {
            out.push('.');
            if integer_length < n_digits {
                push_digits(&mut out, &digits[integer_length as usize..]);
            } else {
                out.push('0');
            }
        }
    } else if exponent <= 0 && exponent > -3 {
        out.push_str("0.");
        out.extend(std::iter::repeat_n('0', (-exponent) as usize));
        push_digits(&mut out, digits);
    } else {
        out.push(digits[0] as char);
        out.push('.');
        if n_digits > 1 {
            push_digits(&mut out, &digits[1..]);
        } else {
            out.push('0');
        }
        out.push('E');
        let printed_exponent = if exponent <= 0 {
            out.push('-');
            -exponent + 1
        } else {
            exponent - 1
        };
        out.push_str(&printed_exponent.to_string());
    }
    out
}

fn push_digits(out: &mut String, digits: &[u8]) {
    out.extend(digits.iter().map(|digit| *digit as char));
}

pub fn double_to_string(value: f64) -> String {
    java_format_string(double_to_decimal(value, DigitMode::Compatible))
}

pub fn float_to_string(value: f32) -> String {
    java_format_string(float_to_decimal(value))
}
