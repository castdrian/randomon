use randomon_data::Type;

const CURSE_PALETTE: i32 = 411;

pub fn type_tm_palette_number(kind: Option<Type>) -> i32 {
    match kind {
        Some(Type::Fighting) => 398,
        Some(Type::Dragon) => 399,
        Some(Type::Water) => 400,
        Some(Type::Psychic) => 401,
        Some(Type::Normal) => 402,
        Some(Type::Poison) => 403,
        Some(Type::Ice) => 404,
        Some(Type::Grass) => 405,
        Some(Type::Fire) => 406,
        Some(Type::Dark) => 407,
        Some(Type::Steel) => 408,
        Some(Type::Electric) => 409,
        Some(Type::Ground) => 410,
        Some(Type::Rock) => 412,
        Some(Type::Flying) => 413,
        Some(Type::Bug) => 610,
        _ => CURSE_PALETTE,
    }
}
