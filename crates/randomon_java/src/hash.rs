use std::cmp::Ordering;

pub trait JavaHash {
    fn java_hash_code(&self) -> i32;

    fn java_compare_to(&self, _other: &Self) -> Option<Ordering> {
        None
    }
}

pub fn string_hash_code(value: &str) -> i32 {
    value.encode_utf16().fold(0i32, |hash, unit| {
        hash.wrapping_mul(31).wrapping_add(i32::from(unit))
    })
}

pub fn utf16_hash_code(units: &[u16]) -> i32 {
    units.iter().fold(0i32, |hash, unit| {
        hash.wrapping_mul(31).wrapping_add(i32::from(*unit))
    })
}

pub fn string_compare_to(left: &str, right: &str) -> Ordering {
    left.encode_utf16().cmp(right.encode_utf16())
}

pub fn long_hash_code(value: i64) -> i32 {
    (value ^ ((value as u64) >> 32) as i64) as i32
}

pub fn boolean_hash_code(value: bool) -> i32 {
    if value { 1231 } else { 1237 }
}

pub fn double_hash_code(value: f64) -> i32 {
    long_hash_code(double_to_long_bits(value))
}

pub fn double_to_long_bits(value: f64) -> i64 {
    if value.is_nan() {
        0x7FF8_0000_0000_0000
    } else {
        value.to_bits() as i64
    }
}

pub fn spread(hash_code: i32) -> i32 {
    hash_code ^ ((hash_code as u32) >> 16) as i32
}

impl JavaHash for i32 {
    fn java_hash_code(&self) -> i32 {
        *self
    }
    fn java_compare_to(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl JavaHash for i64 {
    fn java_hash_code(&self) -> i32 {
        long_hash_code(*self)
    }
    fn java_compare_to(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl JavaHash for i16 {
    fn java_hash_code(&self) -> i32 {
        i32::from(*self)
    }
    fn java_compare_to(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl JavaHash for i8 {
    fn java_hash_code(&self) -> i32 {
        i32::from(*self)
    }
    fn java_compare_to(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl JavaHash for bool {
    fn java_hash_code(&self) -> i32 {
        boolean_hash_code(*self)
    }
    fn java_compare_to(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl JavaHash for str {
    fn java_hash_code(&self) -> i32 {
        string_hash_code(self)
    }
    fn java_compare_to(&self, other: &Self) -> Option<Ordering> {
        Some(string_compare_to(self, other))
    }
}

impl JavaHash for String {
    fn java_hash_code(&self) -> i32 {
        string_hash_code(self)
    }
    fn java_compare_to(&self, other: &Self) -> Option<Ordering> {
        Some(string_compare_to(self, other))
    }
}

impl<T: JavaHash + ?Sized> JavaHash for &T {
    fn java_hash_code(&self) -> i32 {
        (**self).java_hash_code()
    }
    fn java_compare_to(&self, other: &Self) -> Option<Ordering> {
        (**self).java_compare_to(*other)
    }
}

impl<T: JavaHash + ?Sized> JavaHash for Box<T> {
    fn java_hash_code(&self) -> i32 {
        (**self).java_hash_code()
    }
    fn java_compare_to(&self, other: &Self) -> Option<Ordering> {
        (**self).java_compare_to(other)
    }
}

impl<T: JavaHash + ?Sized> JavaHash for std::rc::Rc<T> {
    fn java_hash_code(&self) -> i32 {
        (**self).java_hash_code()
    }
    fn java_compare_to(&self, other: &Self) -> Option<Ordering> {
        (**self).java_compare_to(other)
    }
}

impl<T: JavaHash + ?Sized> JavaHash for std::sync::Arc<T> {
    fn java_hash_code(&self) -> i32 {
        (**self).java_hash_code()
    }
    fn java_compare_to(&self, other: &Self) -> Option<Ordering> {
        (**self).java_compare_to(other)
    }
}
