pub fn secure_seed_bytes<const N: usize>() -> [u8; N] {
    let mut bytes = [0u8; N];
    getrandom::fill(&mut bytes).expect("operating system random number generator unavailable");
    bytes
}

pub fn random_i64() -> i64 {
    i64::from_le_bytes(secure_seed_bytes::<8>())
}
