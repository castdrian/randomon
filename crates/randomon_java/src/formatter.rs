use std::fmt;

use crate::floating_decimal::{
    BinaryToAscii, DigitMode, double_to_decimal, double_to_string, float_to_string,
};

#[derive(Debug, Clone, PartialEq)]
pub enum FormatArg {
    Null,
    Byte(i8),
    Short(i16),
    Int(i32),
    Long(i64),
    Float(f32),
    Double(f64),
    Char(char),
    Bool(bool),
    Str(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FormatError {
    UnknownConversion(char),
    UnsupportedConversion(char),
    MissingArgument(String),
    IllegalConversion {
        conversion: char,
        argument: &'static str,
    },
    MissingWidth(String),
    IllegalPrecision(String),
    IllegalFlags(String),
    UnterminatedSpecifier,
}

impl fmt::Display for FormatError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownConversion(c) => write!(formatter, "Conversion = '{c}'"),
            Self::UnsupportedConversion(c) => {
                write!(formatter, "conversion '{c}' is not supported")
            }
            Self::MissingArgument(spec) => write!(formatter, "Format specifier '{spec}'"),
            Self::IllegalConversion {
                conversion,
                argument,
            } => write!(formatter, "{conversion} != {argument}"),
            Self::MissingWidth(spec) => write!(formatter, "{spec}"),
            Self::IllegalPrecision(spec) => write!(formatter, "precision in {spec}"),
            Self::IllegalFlags(spec) => write!(formatter, "Flags = '{spec}'"),
            Self::UnterminatedSpecifier => write!(formatter, "Format specifier '%'"),
        }
    }
}

impl std::error::Error for FormatError {}

impl FormatArg {
    fn type_name(&self) -> &'static str {
        match self {
            Self::Null => "null",
            Self::Byte(_) => "java.lang.Byte",
            Self::Short(_) => "java.lang.Short",
            Self::Int(_) => "java.lang.Integer",
            Self::Long(_) => "java.lang.Long",
            Self::Float(_) => "java.lang.Float",
            Self::Double(_) => "java.lang.Double",
            Self::Char(_) => "java.lang.Character",
            Self::Bool(_) => "java.lang.Boolean",
            Self::Str(_) => "java.lang.String",
        }
    }

    pub fn java_to_string(&self) -> String {
        match self {
            Self::Null => "null".to_owned(),
            Self::Byte(value) => value.to_string(),
            Self::Short(value) => value.to_string(),
            Self::Int(value) => value.to_string(),
            Self::Long(value) => value.to_string(),
            Self::Float(value) => float_to_string(*value),
            Self::Double(value) => double_to_string(*value),
            Self::Char(value) => value.to_string(),
            Self::Bool(value) => value.to_string(),
            Self::Str(value) => value.clone(),
        }
    }
}

macro_rules! format_arg_from {
    ($($source:ty => $variant:ident),* $(,)?) => {
        $(impl From<$source> for FormatArg {
            fn from(value: $source) -> Self {
                Self::$variant(value)
            }
        })*
    };
}

format_arg_from!(i8 => Byte, i16 => Short, i32 => Int, i64 => Long, f32 => Float, f64 => Double, char => Char, bool => Bool, String => Str);

impl From<&str> for FormatArg {
    fn from(value: &str) -> Self {
        Self::Str(value.to_owned())
    }
}

impl From<&String> for FormatArg {
    fn from(value: &String) -> Self {
        Self::Str(value.clone())
    }
}

impl<T: Into<FormatArg>> From<Option<T>> for FormatArg {
    fn from(value: Option<T>) -> Self {
        value.map_or(Self::Null, Into::into)
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct Flags {
    left_justify: bool,
    alternate: bool,
    plus: bool,
    leading_space: bool,
    zero_pad: bool,
    group: bool,
    parentheses: bool,
    previous: bool,
    uppercase: bool,
}

#[derive(Debug, Clone)]
struct Specifier {
    text: String,
    index: Option<usize>,
    flags: Flags,
    width: Option<usize>,
    precision: Option<usize>,
    conversion: char,
}

pub fn java_format(format: &str, args: &[FormatArg]) -> Result<String, FormatError> {
    java_format_with_line_separator(format, args, "\n")
}

pub fn java_format_with_line_separator(
    format: &str,
    args: &[FormatArg],
    line_separator: &str,
) -> Result<String, FormatError> {
    let chars: Vec<char> = format.chars().collect();
    let mut out = String::with_capacity(format.len() + 16);
    let mut position = 0;
    let mut ordinary_index = 0;
    let mut last_index: Option<usize> = None;
    while position < chars.len() {
        if chars[position] != '%' {
            out.push(chars[position]);
            position += 1;
            continue;
        }
        let (specifier, next_position) = parse_specifier(&chars, position)?;
        position = next_position;
        match specifier.conversion {
            '%' => {
                reject_precision(&specifier)?;
                append_justified(&mut out, "%".to_owned(), &specifier);
            }
            'n' => out.push_str(line_separator),
            _ => {
                let argument_index = if specifier.flags.previous {
                    last_index
                        .ok_or_else(|| FormatError::MissingArgument(specifier.text.clone()))?
                } else if let Some(explicit) = specifier.index {
                    explicit - 1
                } else {
                    ordinary_index += 1;
                    ordinary_index - 1
                };
                last_index = Some(argument_index);
                let argument = args
                    .get(argument_index)
                    .ok_or_else(|| FormatError::MissingArgument(specifier.text.clone()))?;
                let formatted = format_argument(&specifier, argument)?;
                append_justified(&mut out, formatted, &specifier);
            }
        }
    }
    Ok(out)
}

fn parse_digits(chars: &[char], mut position: usize) -> (Option<usize>, usize) {
    let start = position;
    while position < chars.len() && chars[position].is_ascii_digit() {
        position += 1;
    }
    if position == start {
        return (None, position);
    }
    let digits: String = chars[start..position].iter().collect();
    (digits.parse().ok(), position)
}

fn parse_specifier(chars: &[char], start: usize) -> Result<(Specifier, usize), FormatError> {
    let mut position = start + 1;
    let mut index = None;
    let (leading_number, after_number) = parse_digits(chars, position);
    if leading_number.is_some() && chars.get(after_number) == Some(&'$') {
        index = leading_number;
        position = after_number + 1;
    }
    let mut flags = Flags::default();
    while let Some(flag) = chars.get(position) {
        match flag {
            '-' => flags.left_justify = true,
            '#' => flags.alternate = true,
            '+' => flags.plus = true,
            ' ' => flags.leading_space = true,
            '0' => flags.zero_pad = true,
            ',' => flags.group = true,
            '(' => flags.parentheses = true,
            '<' => flags.previous = true,
            _ => break,
        }
        position += 1;
    }
    let (width, after_width) = parse_digits(chars, position);
    position = after_width;
    let mut precision = None;
    if chars.get(position) == Some(&'.') {
        let (parsed, after_precision) = parse_digits(chars, position + 1);
        if parsed.is_none() {
            return Err(FormatError::UnterminatedSpecifier);
        }
        precision = parsed;
        position = after_precision;
    }
    let Some(&raw_conversion) = chars.get(position) else {
        return Err(FormatError::UnterminatedSpecifier);
    };
    position += 1;
    if raw_conversion.is_ascii_uppercase() {
        flags.uppercase = true;
    }
    let conversion = raw_conversion.to_ascii_lowercase();
    let text: String = chars[start..position].iter().collect();
    let specifier = Specifier {
        text,
        index,
        flags,
        width,
        precision,
        conversion: if raw_conversion == '%' {
            '%'
        } else {
            conversion
        },
    };
    validate_specifier(&specifier)?;
    Ok((specifier, position))
}

fn validate_specifier(specifier: &Specifier) -> Result<(), FormatError> {
    let flags = specifier.flags;
    if (flags.left_justify || flags.zero_pad)
        && specifier.width.is_none()
        && specifier.conversion != 'n'
    {
        return Err(FormatError::MissingWidth(specifier.text.clone()));
    }
    if flags.left_justify && flags.zero_pad {
        return Err(FormatError::IllegalFlags(specifier.text.clone()));
    }
    if flags.plus && flags.leading_space {
        return Err(FormatError::IllegalFlags(specifier.text.clone()));
    }
    Ok(())
}

fn reject_precision(specifier: &Specifier) -> Result<(), FormatError> {
    if specifier.precision.is_some() {
        Err(FormatError::IllegalPrecision(specifier.text.clone()))
    } else {
        Ok(())
    }
}

fn utf16_length(text: &str) -> usize {
    text.encode_utf16().count()
}

fn append_justified(out: &mut String, text: String, specifier: &Specifier) {
    let Some(width) = specifier.width else {
        out.push_str(&text);
        return;
    };
    let padding = width.saturating_sub(utf16_length(&text));
    if specifier.flags.left_justify {
        out.push_str(&text);
        out.extend(std::iter::repeat_n(' ', padding));
    } else {
        out.extend(std::iter::repeat_n(' ', padding));
        out.push_str(&text);
    }
}

fn format_argument(specifier: &Specifier, argument: &FormatArg) -> Result<String, FormatError> {
    match specifier.conversion {
        's' => Ok(format_string(specifier, argument)),
        'b' => Ok(format_boolean(specifier, argument)),
        'c' => format_character(specifier, argument),
        'd' => format_decimal_integer(specifier, argument),
        'x' | 'o' => format_unsigned_radix(specifier, argument),
        'f' => format_decimal_float(specifier, argument),
        'h' => Ok(apply_precision_and_case(
            specifier,
            format!("{:x}", java_hash_of_argument(argument)),
        )),
        'e' | 'g' | 'a' | 't' => Err(FormatError::UnsupportedConversion(specifier.conversion)),
        other => Err(FormatError::UnknownConversion(other)),
    }
}

fn java_hash_of_argument(argument: &FormatArg) -> u32 {
    use crate::hash::{JavaHash, boolean_hash_code, double_hash_code, long_hash_code};
    let hash = match argument {
        FormatArg::Null => return 0,
        FormatArg::Byte(value) => i32::from(*value),
        FormatArg::Short(value) => i32::from(*value),
        FormatArg::Int(value) => *value,
        FormatArg::Long(value) => long_hash_code(*value),
        FormatArg::Float(value) => value.to_bits() as i32,
        FormatArg::Double(value) => double_hash_code(*value),
        FormatArg::Char(value) => *value as i32,
        FormatArg::Bool(value) => boolean_hash_code(*value),
        FormatArg::Str(value) => value.java_hash_code(),
    };
    hash as u32
}

fn apply_precision_and_case(specifier: &Specifier, text: String) -> String {
    let truncated = match specifier.precision {
        Some(precision) if precision < utf16_length(&text) => {
            let units: Vec<u16> = text.encode_utf16().take(precision).collect();
            String::from_utf16_lossy(&units)
        }
        _ => text,
    };
    if specifier.flags.uppercase {
        truncated.to_uppercase()
    } else {
        truncated
    }
}

fn format_string(specifier: &Specifier, argument: &FormatArg) -> String {
    apply_precision_and_case(specifier, argument.java_to_string())
}

fn format_boolean(specifier: &Specifier, argument: &FormatArg) -> String {
    let value = match argument {
        FormatArg::Null => false,
        FormatArg::Bool(value) => *value,
        _ => true,
    };
    apply_precision_and_case(specifier, value.to_string())
}

fn format_character(specifier: &Specifier, argument: &FormatArg) -> Result<String, FormatError> {
    reject_precision(specifier)?;
    let character = match argument {
        FormatArg::Null => return Ok("null".to_owned()),
        FormatArg::Char(value) => Some(*value),
        FormatArg::Byte(value) => char::from_u32(u32::from(*value as u8)),
        FormatArg::Short(value) => char::from_u32(u32::from(*value as u16)),
        FormatArg::Int(value) => char::from_u32(*value as u32),
        other => {
            return Err(FormatError::IllegalConversion {
                conversion: 'c',
                argument: other.type_name(),
            });
        }
    };
    let text = character.map(String::from).unwrap_or_default();
    Ok(if specifier.flags.uppercase {
        text.to_uppercase()
    } else {
        text
    })
}

fn integral_value(
    specifier: &Specifier,
    argument: &FormatArg,
) -> Result<Option<(i64, u32)>, FormatError> {
    match argument {
        FormatArg::Null => Ok(None),
        FormatArg::Byte(value) => Ok(Some((i64::from(*value), 8))),
        FormatArg::Short(value) => Ok(Some((i64::from(*value), 16))),
        FormatArg::Int(value) => Ok(Some((i64::from(*value), 32))),
        FormatArg::Long(value) => Ok(Some((*value, 64))),
        other => Err(FormatError::IllegalConversion {
            conversion: specifier.conversion,
            argument: other.type_name(),
        }),
    }
}

fn leading_sign(out: &mut String, negative: bool, flags: Flags) {
    if !negative {
        if flags.plus {
            out.push('+');
        } else if flags.leading_space {
            out.push(' ');
        }
    } else if flags.parentheses {
        out.push('(');
    } else {
        out.push('-');
    }
}

fn trailing_sign(out: &mut String, negative: bool, flags: Flags) {
    if negative && flags.parentheses {
        out.push(')');
    }
}

fn adjust_width(width: Option<usize>, flags: Flags, negative: bool) -> Option<usize> {
    width.map(|width| {
        if negative && flags.parentheses {
            width.saturating_sub(1)
        } else {
            width
        }
    })
}

fn localized_magnitude(out: &mut String, value: &str, flags: Flags, width: Option<usize>) {
    let begin = out.len();
    let chars: Vec<char> = value.chars().collect();
    let dot = chars.iter().position(|c| *c == '.').unwrap_or(chars.len());
    let mut grouping = flags.group;
    for (j, c) in chars.iter().enumerate() {
        if j == dot {
            out.push('.');
            grouping = false;
            continue;
        }
        out.push(*c);
        if grouping && j + 1 != dot && (dot - j) % 3 == 1 {
            out.push(',');
        }
    }
    if let (Some(width), true) = (width, flags.zero_pad) {
        let current = utf16_length(out);
        if current < width {
            out.insert_str(begin, &"0".repeat(width - current));
        }
    }
}

fn format_decimal_integer(
    specifier: &Specifier,
    argument: &FormatArg,
) -> Result<String, FormatError> {
    reject_precision(specifier)?;
    if specifier.flags.alternate {
        return Err(FormatError::IllegalFlags(specifier.text.clone()));
    }
    let Some((value, _)) = integral_value(specifier, argument)? else {
        return Ok("null".to_owned());
    };
    let negative = value < 0;
    let magnitude = value.unsigned_abs().to_string();
    let mut out = String::new();
    leading_sign(&mut out, negative, specifier.flags);
    let width = adjust_width(specifier.width, specifier.flags, negative);
    localized_magnitude(&mut out, &magnitude, specifier.flags, width);
    trailing_sign(&mut out, negative, specifier.flags);
    Ok(out)
}

fn format_unsigned_radix(
    specifier: &Specifier,
    argument: &FormatArg,
) -> Result<String, FormatError> {
    reject_precision(specifier)?;
    let flags = specifier.flags;
    if flags.parentheses || flags.leading_space || flags.plus || flags.group {
        return Err(FormatError::IllegalFlags(specifier.text.clone()));
    }
    let Some((value, bits)) = integral_value(specifier, argument)? else {
        return Ok("null".to_owned());
    };
    let unsigned = if bits == 64 {
        value as u64
    } else {
        (value as u64) & ((1u64 << bits) - 1)
    };
    let (digits, prefix) = if specifier.conversion == 'x' {
        (
            format!("{unsigned:x}"),
            if flags.uppercase { "0X" } else { "0x" },
        )
    } else {
        (format!("{unsigned:o}"), "0")
    };
    let mut out = String::new();
    let length = digits.len() + if flags.alternate { prefix.len() } else { 0 };
    if flags.alternate {
        out.push_str(prefix);
    }
    if let (true, Some(width)) = (flags.zero_pad, specifier.width) {
        out.extend(std::iter::repeat_n('0', width.saturating_sub(length)));
    }
    if flags.uppercase {
        out.push_str(&digits.to_uppercase());
    } else {
        out.push_str(&digits);
    }
    Ok(out)
}

fn format_decimal_float(
    specifier: &Specifier,
    argument: &FormatArg,
) -> Result<String, FormatError> {
    let value = match argument {
        FormatArg::Null => return Ok("null".to_owned()),
        FormatArg::Double(value) => *value,
        FormatArg::Float(value) => f64::from(*value),
        other => {
            return Err(FormatError::IllegalConversion {
                conversion: 'f',
                argument: other.type_name(),
            });
        }
    };
    if value.is_nan() {
        return Ok(if specifier.flags.uppercase {
            "NAN"
        } else {
            "NaN"
        }
        .to_owned());
    }
    let flags = specifier.flags;
    let negative = value < 0.0 || (value == 0.0 && value.is_sign_negative());
    let magnitude_value = value.abs();
    let mut out = String::new();
    leading_sign(&mut out, negative, flags);
    if magnitude_value.is_infinite() {
        out.push_str(if flags.uppercase {
            "INFINITY"
        } else {
            "Infinity"
        });
    } else {
        let precision = specifier.precision.unwrap_or(6);
        let mut mantissa = add_zeros(
            decimal_float_mantissa(magnitude_value, precision),
            precision,
        );
        if flags.alternate && precision == 0 {
            mantissa.push('.');
        }
        let width = adjust_width(specifier.width, flags, negative);
        localized_magnitude(&mut out, &mantissa, flags, width);
    }
    trailing_sign(&mut out, negative, flags);
    Ok(out)
}

pub fn format_fixed(value: f64, precision: usize) -> String {
    let specifier = Specifier {
        text: String::new(),
        index: None,
        flags: Flags::default(),
        width: None,
        precision: Some(precision),
        conversion: 'f',
    };
    format_decimal_float(&specifier, &FormatArg::Double(value)).unwrap_or_default()
}

fn decimal_float_mantissa(value: f64, precision: usize) -> String {
    let BinaryToAscii::Finite(decimal) = double_to_decimal(value, DigitMode::Formatter) else {
        unreachable!("finite magnitude expected");
    };
    let mut digits = decimal.digits;
    let n_digits = digits.len() as i32;
    let precision = precision as i32;
    let exponent = apply_precision(
        decimal.decimal_exponent,
        &mut digits,
        decimal.decimal_exponent.saturating_add(precision),
    );
    fill_decimal(precision, &digits, n_digits, exponent)
}

fn apply_precision(dec_exp: i32, digits: &mut [u8], prec: i32) -> i32 {
    let n_digits = digits.len() as i32;
    if prec >= n_digits || prec < 0 {
        return dec_exp;
    }
    if prec == 0 {
        if digits[0] >= b'5' {
            digits[0] = b'1';
            digits[1..].fill(b'0');
            return dec_exp + 1;
        }
        digits.fill(b'0');
        return dec_exp;
    }
    let prec = prec as usize;
    if digits[prec] >= b'5' {
        let mut i = prec - 1;
        let mut q = digits[i];
        if q == b'9' {
            while q == b'9' && i > 0 {
                i -= 1;
                q = digits[i];
            }
            if q == b'9' {
                digits[0] = b'1';
                digits[1..].fill(b'0');
                return dec_exp + 1;
            }
        }
        digits[i] = q + 1;
        digits[i + 1..].fill(b'0');
    } else {
        digits[prec..].fill(b'0');
    }
    dec_exp
}

fn digits_to_string(digits: &[u8]) -> String {
    digits.iter().map(|digit| *digit as char).collect()
}

fn fill_decimal(precision: i32, digits: &[u8], n_digits: i32, exp: i32) -> String {
    if exp > 0 {
        if n_digits < exp {
            let mut mantissa = digits_to_string(&digits[..n_digits as usize]);
            mantissa.extend(std::iter::repeat_n('0', (exp - n_digits) as usize));
            return mantissa;
        }
        let t = (n_digits - exp).min(precision);
        let mut mantissa = digits_to_string(&digits[..exp as usize]);
        if t > 0 {
            mantissa.push('.');
            mantissa.push_str(&digits_to_string(&digits[exp as usize..(exp + t) as usize]));
        }
        return mantissa;
    }
    let zeros = (-exp).min(precision).max(0);
    let t = n_digits.min(precision + exp).max(0);
    if zeros > 0 || t > 0 {
        let mut mantissa = String::from("0.");
        mantissa.extend(std::iter::repeat_n('0', zeros as usize));
        mantissa.push_str(&digits_to_string(&digits[..t as usize]));
        return mantissa;
    }
    "0".to_owned()
}

fn add_zeros(mantissa: String, precision: usize) -> String {
    let dot = mantissa.find('.');
    let existing = match dot {
        Some(position) => mantissa.len() - position - 1,
        None => 0,
    };
    if existing == precision {
        return mantissa;
    }
    let mut padded = mantissa;
    if dot.is_none() {
        padded.push('.');
    }
    padded.extend(std::iter::repeat_n('0', precision - existing));
    padded
}
