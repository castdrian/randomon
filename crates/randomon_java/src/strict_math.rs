const TWO54: f64 = 1.801_439_850_948_198_4e16;
const LN2_HI: f64 = f64::from_bits(0x3FE6_2E42_FEE0_0000);
const LN2_LO: f64 = f64::from_bits(0x3DEA_39EF_3579_3C76);
const LG1: f64 = f64::from_bits(0x3FE5_5555_5555_5593);
const LG2: f64 = f64::from_bits(0x3FD9_9999_9997_FA04);
const LG3: f64 = f64::from_bits(0x3FD2_4924_9422_9359);
const LG4: f64 = f64::from_bits(0x3FCC_71C5_1D8E_78AF);
const LG5: f64 = f64::from_bits(0x3FC7_4664_96CB_03DE);
const LG6: f64 = f64::from_bits(0x3FC3_9A09_D078_C69F);
const LG7: f64 = f64::from_bits(0x3FC2_F112_DF3E_5244);
const EXP_SIGNIF_BITS: i32 = 0x7FFF_FFFF;
const EXP_BITS: i32 = 0x7FF0_0000;

fn high_word(x: f64) -> i32 {
    (x.to_bits() >> 32) as i32
}

fn low_word(x: f64) -> i32 {
    x.to_bits() as i32
}

fn with_high_word(x: f64, high: i32) -> f64 {
    f64::from_bits((x.to_bits() & 0xFFFF_FFFF) | ((high as u32 as u64) << 32))
}

pub fn log(x: f64) -> f64 {
    let mut x = x;
    let mut hx = high_word(x);
    let lx = low_word(x);
    let mut k: i32 = 0;
    if hx < 0x0010_0000 {
        if ((hx & EXP_SIGNIF_BITS) | lx) == 0 {
            return -TWO54 / 0.0;
        }
        if hx < 0 {
            return f64::NAN;
        }
        k -= 54;
        x *= TWO54;
        hx = high_word(x);
    }
    if hx >= EXP_BITS {
        return x + x;
    }
    k += (hx >> 20) - 1023;
    hx &= 0x000F_FFFF;
    let i = (hx + 0x9_5F64) & 0x10_0000;
    x = with_high_word(x, hx | (i ^ 0x3FF0_0000));
    k += i >> 20;
    let f = x - 1.0;
    if (0x000F_FFFF & (2 + hx)) < 3 {
        return log_near_power_of_two(f, k);
    }
    let s = f / (2.0 + f);
    let dk = k as f64;
    let z = s * s;
    let mut i = hx - 0x6_147A;
    let w = z * z;
    let j = 0x6B851 - hx;
    let t1 = w * (LG2 + w * (LG4 + w * LG6));
    let t2 = z * (LG1 + w * (LG3 + w * (LG5 + w * LG7)));
    i |= j;
    let r = t2 + t1;
    if i > 0 {
        let hfsq = 0.5 * f * f;
        if k == 0 {
            f - (hfsq - s * (hfsq + r))
        } else {
            dk * LN2_HI - ((hfsq - (s * (hfsq + r) + dk * LN2_LO)) - f)
        }
    } else if k == 0 {
        f - s * (f - r)
    } else {
        dk * LN2_HI - ((s * (f - r) - dk * LN2_LO) - f)
    }
}

fn log_near_power_of_two(f: f64, k: i32) -> f64 {
    if f == 0.0 {
        if k == 0 {
            return 0.0;
        }
        let dk = k as f64;
        return dk * LN2_HI + dk * LN2_LO;
    }
    let r = f * f * (0.5 - 0.333_333_333_333_333_3 * f);
    if k == 0 {
        f - r
    } else {
        let dk = k as f64;
        dk * LN2_HI - ((r - dk * LN2_LO) - f)
    }
}

pub fn sqrt(x: f64) -> f64 {
    x.sqrt()
}
