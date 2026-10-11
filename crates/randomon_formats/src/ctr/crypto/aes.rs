pub const BLOCK_SIZE: usize = 16;

pub type Block = [u8; BLOCK_SIZE];

const ROUNDS: usize = 10;
const ROUND_KEY_WORDS: usize = 4 * (ROUNDS + 1);
const ROUND_CONSTANTS: [u8; ROUNDS] = [0x01, 0x02, 0x04, 0x08, 0x10, 0x20, 0x40, 0x80, 0x1B, 0x36];

const fn gf_multiply(mut left: u8, mut right: u8) -> u8 {
    let mut product = 0u8;
    while right != 0 {
        if right & 1 != 0 {
            product ^= left;
        }
        let carry = left & 0x80;
        left <<= 1;
        if carry != 0 {
            left ^= 0x1B;
        }
        right >>= 1;
    }
    product
}

const fn gf_inverse(value: u8) -> u8 {
    if value == 0 {
        return 0;
    }
    let mut result = 1u8;
    let mut base = value;
    let mut exponent = 254u32;
    while exponent > 0 {
        if exponent & 1 != 0 {
            result = gf_multiply(result, base);
        }
        base = gf_multiply(base, base);
        exponent >>= 1;
    }
    result
}

const fn build_sbox() -> [u8; 256] {
    let mut table = [0u8; 256];
    let mut index = 0;
    while index < 256 {
        let inverse = gf_inverse(index as u8);
        table[index] = inverse
            ^ inverse.rotate_left(1)
            ^ inverse.rotate_left(2)
            ^ inverse.rotate_left(3)
            ^ inverse.rotate_left(4)
            ^ 0x63;
        index += 1;
    }
    table
}

const fn build_inverse_sbox(sbox: &[u8; 256]) -> [u8; 256] {
    let mut table = [0u8; 256];
    let mut index = 0;
    while index < 256 {
        table[sbox[index] as usize] = index as u8;
        index += 1;
    }
    table
}

const fn build_encrypt_table(sbox: &[u8; 256]) -> [u32; 256] {
    let mut table = [0u32; 256];
    let mut index = 0;
    while index < 256 {
        let substituted = sbox[index];
        table[index] = (gf_multiply(substituted, 2) as u32) << 24
            | (substituted as u32) << 16
            | (substituted as u32) << 8
            | gf_multiply(substituted, 3) as u32;
        index += 1;
    }
    table
}

const SBOX: [u8; 256] = build_sbox();
const INVERSE_SBOX: [u8; 256] = build_inverse_sbox(&SBOX);
const ENCRYPT_TABLE: [u32; 256] = build_encrypt_table(&SBOX);

fn substitute_word(word: u32) -> u32 {
    u32::from_be_bytes(word.to_be_bytes().map(|byte| SBOX[byte as usize]))
}

fn table_column(first: u32, second: u32, third: u32, fourth: u32) -> u32 {
    ENCRYPT_TABLE[(first >> 24) as usize]
        ^ ENCRYPT_TABLE[((second >> 16) & 0xFF) as usize].rotate_right(8)
        ^ ENCRYPT_TABLE[((third >> 8) & 0xFF) as usize].rotate_right(16)
        ^ ENCRYPT_TABLE[(fourth & 0xFF) as usize].rotate_right(24)
}

fn final_column(first: u32, second: u32, third: u32, fourth: u32) -> u32 {
    u32::from_be_bytes([
        SBOX[(first >> 24) as usize],
        SBOX[((second >> 16) & 0xFF) as usize],
        SBOX[((third >> 8) & 0xFF) as usize],
        SBOX[(fourth & 0xFF) as usize],
    ])
}

#[derive(Clone)]
pub struct Aes128 {
    round_keys: [u32; ROUND_KEY_WORDS],
}

impl std::fmt::Debug for Aes128 {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("Aes128(..)")
    }
}

impl Aes128 {
    pub fn new(key: &Block) -> Self {
        let mut round_keys = [0u32; ROUND_KEY_WORDS];
        for (index, chunk) in key.as_chunks::<4>().0.iter().enumerate() {
            round_keys[index] = u32::from_be_bytes(*chunk);
        }
        for index in 4..ROUND_KEY_WORDS {
            let mut word = round_keys[index - 1];
            if index % 4 == 0 {
                word = substitute_word(word.rotate_left(8))
                    ^ (ROUND_CONSTANTS[index / 4 - 1] as u32) << 24;
            }
            round_keys[index] = round_keys[index - 4] ^ word;
        }
        Aes128 { round_keys }
    }

    fn round_key(&self, round: usize) -> &[u32] {
        &self.round_keys[round * 4..round * 4 + 4]
    }

    pub fn encrypt_block(&self, block: &Block) -> Block {
        let key = self.round_key(0);
        let mut state = [0u32; 4];
        for (column, chunk) in block.as_chunks::<4>().0.iter().enumerate() {
            state[column] = u32::from_be_bytes(*chunk) ^ key[column];
        }
        for round in 1..ROUNDS {
            let key = self.round_key(round);
            let [a, b, c, d] = state;
            state = [
                table_column(a, b, c, d) ^ key[0],
                table_column(b, c, d, a) ^ key[1],
                table_column(c, d, a, b) ^ key[2],
                table_column(d, a, b, c) ^ key[3],
            ];
        }
        let key = self.round_key(ROUNDS);
        let [a, b, c, d] = state;
        let columns = [
            final_column(a, b, c, d) ^ key[0],
            final_column(b, c, d, a) ^ key[1],
            final_column(c, d, a, b) ^ key[2],
            final_column(d, a, b, c) ^ key[3],
        ];
        let mut output = [0u8; BLOCK_SIZE];
        for (column, word) in columns.iter().enumerate() {
            output[column * 4..column * 4 + 4].copy_from_slice(&word.to_be_bytes());
        }
        output
    }

    pub fn decrypt_block(&self, block: &Block) -> Block {
        let mut state = *block;
        self.add_round_key(&mut state, ROUNDS);
        for round in (1..ROUNDS).rev() {
            inverse_shift_rows(&mut state);
            inverse_substitute(&mut state);
            self.add_round_key(&mut state, round);
            inverse_mix_columns(&mut state);
        }
        inverse_shift_rows(&mut state);
        inverse_substitute(&mut state);
        self.add_round_key(&mut state, 0);
        state
    }

    fn add_round_key(&self, state: &mut Block, round: usize) {
        for (column, word) in self.round_key(round).iter().enumerate() {
            for (byte, key_byte) in state[column * 4..column * 4 + 4]
                .iter_mut()
                .zip(word.to_be_bytes())
            {
                *byte ^= key_byte;
            }
        }
    }
}

fn inverse_shift_rows(state: &mut Block) {
    let original = *state;
    for row in 0..4 {
        for column in 0..4 {
            state[row + 4 * ((column + row) % 4)] = original[row + 4 * column];
        }
    }
}

fn inverse_substitute(state: &mut Block) {
    for byte in state.iter_mut() {
        *byte = INVERSE_SBOX[*byte as usize];
    }
}

fn inverse_mix_columns(state: &mut Block) {
    for column in state.as_chunks_mut::<4>().0 {
        let [a, b, c, d] = [column[0], column[1], column[2], column[3]];
        column[0] =
            gf_multiply(a, 14) ^ gf_multiply(b, 11) ^ gf_multiply(c, 13) ^ gf_multiply(d, 9);
        column[1] =
            gf_multiply(a, 9) ^ gf_multiply(b, 14) ^ gf_multiply(c, 11) ^ gf_multiply(d, 13);
        column[2] =
            gf_multiply(a, 13) ^ gf_multiply(b, 9) ^ gf_multiply(c, 14) ^ gf_multiply(d, 11);
        column[3] =
            gf_multiply(a, 11) ^ gf_multiply(b, 13) ^ gf_multiply(c, 9) ^ gf_multiply(d, 14);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sequential_block(start: u8, step: u8) -> Block {
        std::array::from_fn(|index| start.wrapping_add(step.wrapping_mul(index as u8)))
    }

    #[test]
    fn matches_fips_197_appendix_c1() {
        let key = sequential_block(0x00, 0x01);
        let plaintext = sequential_block(0x00, 0x11);
        let expected = [
            0x69, 0xC4, 0xE0, 0xD8, 0x6A, 0x7B, 0x04, 0x30, 0xD8, 0xCD, 0xB7, 0x80, 0x70, 0xB4,
            0xC5, 0x5A,
        ];
        let cipher = Aes128::new(&key);
        assert_eq!(cipher.encrypt_block(&plaintext), expected);
        assert_eq!(cipher.decrypt_block(&expected), plaintext);
    }

    #[test]
    fn decrypt_inverts_encrypt_for_many_keys() {
        for seed in 0..32u8 {
            let cipher = Aes128::new(&sequential_block(seed, 37));
            let block = sequential_block(seed.wrapping_mul(7), 91);
            assert_eq!(cipher.decrypt_block(&cipher.encrypt_block(&block)), block);
        }
    }
}
