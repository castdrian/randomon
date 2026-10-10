mod collections;
mod entropy;
mod enum_map;
mod floating_decimal;
mod formatter;
mod hash;
mod hash_map;
mod hash_set;
pub mod math;
mod random;
mod random_source;
pub mod strict_math;

pub use collections::shuffle;
pub use enum_map::{JavaEnum, JavaEnumMap, JavaEnumSet};
pub use floating_decimal::{
    BinaryToAscii, DecimalDigits, DigitMode, double_to_decimal, double_to_string, float_to_decimal,
    float_to_string,
};
pub use formatter::{
    FormatArg, FormatError, format_fixed, java_format, java_format_with_line_separator,
};
pub use hash::{
    JavaHash, boolean_hash_code, double_hash_code, double_to_long_bits, long_hash_code, spread,
    string_compare_to, string_hash_code, utf16_hash_code,
};
pub use hash_map::{
    Iter as JavaHashMapIter, JavaHashMap, jdk8_copy_constructor_capacity, table_size_for,
};
pub use hash_set::{JavaHashSet, jdk8_hash_set_copy_capacity};
pub use random::{JavaRandom, JavaRng, next_bytes_from_ints};
pub use random_source::{
    CosmeticRandomSourceInstance, RandomSource, RandomSourceInstance,
    SharedCosmeticRandomSourceInstance, SharedRandomSource, SharedRandomSourceInstance,
};
