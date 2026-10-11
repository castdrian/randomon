mod common;

use std::fs::File;
use std::io::{BufReader, Cursor};
use std::path::PathBuf;

use common::vector;
use randomon_formats::FormatError;
use randomon_formats::ctr::crypto::{
    Aes128, CryptoError, KeyLookup, KeyStore, discover_keys, scramble_normal_key, set_aes_keys_path,
};
use randomon_formats::ctr::{Ncch, container};

const PRIMARY_SLOT: u8 = 0x2C;
const SEVEN_X_SLOT: u8 = 0x25;
const COMMON_SLOT: u8 = 0x3D;

fn pseudo_random_block(seed: u64) -> [u8; 16] {
    let mut state = seed;
    std::array::from_fn(|_| {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (state >> 33) as u8
    })
}

fn hex(block: &[u8; 16]) -> String {
    block.iter().map(|byte| format!("{byte:02X}")).collect()
}

struct TestKeys {
    primary_x: [u8; 16],
    seven_x: [u8; 16],
    common_x: [u8; 16],
    common_y: [u8; 16],
}

impl TestKeys {
    fn generate(seed: u64) -> Self {
        TestKeys {
            primary_x: pseudo_random_block(seed),
            seven_x: pseudo_random_block(seed + 1),
            common_x: pseudo_random_block(seed + 2),
            common_y: pseudo_random_block(seed + 3),
        }
    }

    fn store(&self) -> KeyStore {
        KeyStore::from_aes_keys_text(&format!(
            "slot0x{PRIMARY_SLOT:02X}KeyX={}\nslot0x{SEVEN_X_SLOT:02X}KeyX={}\nslot0x{COMMON_SLOT:02X}KeyX={}\ncommon0={}\n",
            hex(&self.primary_x),
            hex(&self.seven_x),
            hex(&self.common_x),
            hex(&self.common_y)
        ))
    }

    fn common_key(&self) -> [u8; 16] {
        scramble_normal_key(&self.common_x, &self.common_y)
    }
}

fn u32_le(bytes: &[u8], offset: usize) -> u64 {
    u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap()) as u64
}

fn xor_ctr(data: &mut [u8], start: usize, end: usize, origin: usize, key: &[u8; 16], ctr: u128) {
    let cipher = Aes128::new(key);
    for (position, byte) in data.iter_mut().enumerate().take(end).skip(start) {
        let relative = position - origin;
        let block = cipher.encrypt_block(&ctr.wrapping_add((relative / 16) as u128).to_be_bytes());
        *byte ^= block[relative % 16];
    }
}

struct EncryptionPlan {
    method: u8,
    fixed_key: bool,
}

fn encrypt_ncch_in_place(image: &mut [u8], ncch: usize, keys: &TestKeys, plan: &EncryptionPlan) {
    let key_y: [u8; 16] = pseudo_random_block(0xC0FFEE);
    image[ncch..ncch + 16].copy_from_slice(&key_y);
    let (primary, secondary) = if plan.fixed_key {
        ([0u8; 16], [0u8; 16])
    } else {
        let secondary_x = match plan.method {
            0x00 => keys.primary_x,
            0x01 => keys.seven_x,
            other => panic!("unsupported test method {other}"),
        };
        (
            scramble_normal_key(&keys.primary_x, &key_y),
            scramble_normal_key(&secondary_x, &key_y),
        )
    };
    let header = image[ncch..ncch + 0x200].to_vec();
    let partition = u64::from_le_bytes(header[0x108..0x110].try_into().unwrap()) as u128;
    let counter = |section: u128| partition << 64 | section << 56;
    let media = |field: usize| ncch + (u32_le(&header, field) * 0x200) as usize;
    let exefs = media(0x1A0);
    let exefs_size = (u32_le(&header, 0x1A4) * 0x200) as usize;
    let romfs = media(0x1B0);
    let romfs_size = (u32_le(&header, 0x1B4) * 0x200) as usize;
    if u32_le(&header, 0x180) > 0 {
        let start = ncch + 0x200;
        xor_ctr(image, start, start + 0x800, start, &primary, counter(1));
    }
    let exefs_header = image[exefs..exefs + 0x200].to_vec();
    for slot in 0..10 {
        let entry = &exefs_header[slot * 16..slot * 16 + 16];
        let name: Vec<u8> = entry[..8].iter().copied().take_while(|&b| b != 0).collect();
        let size = u32_le(entry, 12) as usize;
        if name.is_empty() || size == 0 {
            continue;
        }
        let start = exefs + 0x200 + u32_le(entry, 8) as usize;
        let key = if name == b"icon" || name == b"banner" {
            &primary
        } else {
            &secondary
        };
        xor_ctr(image, start, start + size, exefs, key, counter(2));
    }
    assert!(exefs_size >= 0x200);
    xor_ctr(image, exefs, exefs + 0x200, exefs, &primary, counter(2));
    xor_ctr(
        image,
        romfs,
        romfs + romfs_size,
        romfs,
        &secondary,
        counter(3),
    );
    image[ncch + 0x18B] = plan.method;
    image[ncch + 0x18F] = if plan.fixed_key { 0x01 } else { 0x00 };
}

fn identity(bytes: &[u8]) -> (Option<String>, Option<String>) {
    let mut cursor = Cursor::new(bytes.to_vec());
    (
        container::product_code(&mut cursor).unwrap(),
        container::title_id(&mut cursor).unwrap(),
    )
}

fn open(bytes: &[u8], store: &KeyStore) -> randomon_formats::Result<Ncch<Cursor<Vec<u8>>>> {
    let (product_code, title_id) = identity(bytes);
    Ncch::open_with_keys(
        Cursor::new(bytes.to_vec()),
        product_code,
        title_id,
        KeyLookup::Using(store),
    )
}

fn plaintext_case(name: &str) -> (Vec<u8>, usize) {
    let cases = vector("ncch");
    let case = cases
        .get("cases")
        .array()
        .iter()
        .find(|case| case.get("name").str() == name)
        .unwrap();
    (
        case.get("input").bytes(),
        case.get("cxi_offset").i64() as usize,
    )
}

fn assert_same_contents(plain: &mut Ncch<Cursor<Vec<u8>>>, decrypted: &mut Ncch<Cursor<Vec<u8>>>) {
    assert!(decrypted.was_decrypted_on_load());
    assert!(decrypted.is_decrypted().unwrap());
    assert_eq!(plain.get_code().unwrap(), decrypted.get_code().unwrap());
    assert_eq!(
        plain.smdh().map(|smdh| smdh.bytes().to_vec()),
        decrypted.smdh().map(|smdh| smdh.bytes().to_vec())
    );
    let paths: Vec<String> = plain
        .romfs_paths()
        .iter()
        .map(|path| path.to_string())
        .collect();
    assert!(!paths.is_empty());
    assert_eq!(
        paths,
        decrypted
            .romfs_paths()
            .iter()
            .map(|path| path.to_string())
            .collect::<Vec<_>>()
    );
    for path in &paths {
        assert_eq!(
            plain.get_file(path).unwrap(),
            decrypted.get_file(path).unwrap(),
            "{path}"
        );
    }
}

fn crypto_error(result: randomon_formats::Result<Ncch<Cursor<Vec<u8>>>>) -> CryptoError {
    match result {
        Err(FormatError::Crypto(error)) => error,
        Err(other) => panic!("expected a crypto error, got {other}"),
        Ok(_) => panic!("expected a crypto error, but the ROM opened"),
    }
}

fn round_trip(case: &str, plan: EncryptionPlan) {
    let keys = TestKeys::generate(17);
    let (plain_bytes, ncch) = plaintext_case(case);
    let mut encrypted = plain_bytes.clone();
    encrypt_ncch_in_place(&mut encrypted, ncch, &keys, &plan);
    assert_ne!(encrypted, plain_bytes);

    let mut plain = open(&plain_bytes, &KeyStore::default()).unwrap();
    assert!(!plain.was_decrypted_on_load());
    let mut decrypted = open(&encrypted, &keys.store()).unwrap();
    assert_same_contents(&mut plain, &mut decrypted);

    let replacement = vec![0x5Au8; 300];
    let target = plain.romfs_paths()[0].to_owned();
    plain.write_file(&target, &replacement).unwrap();
    decrypted.write_file(&target, &replacement).unwrap();
    let saved_plain = plain
        .save_as_ncch(Cursor::new(Vec::new()), "XY", 9)
        .unwrap()
        .into_inner();
    let saved_decrypted = decrypted
        .save_as_ncch(Cursor::new(Vec::new()), "XY", 9)
        .unwrap()
        .into_inner();
    assert_eq!(saved_plain, saved_decrypted);
    assert_eq!(saved_decrypted[0x18B], 0);
    assert_eq!(saved_decrypted[0x18F] & 0x04, 0x04);
    let mut reopened = open(&saved_decrypted, &KeyStore::default()).unwrap();
    assert!(!reopened.was_decrypted_on_load());
    assert!(reopened.is_decrypted().unwrap());
}

#[test]
fn decrypts_original_crypto_ncch_transparently() {
    round_trip(
        "ncch_compressed",
        EncryptionPlan {
            method: 0x00,
            fixed_key: false,
        },
    );
}

#[test]
fn decrypts_seven_x_crypto_inside_cci() {
    round_trip(
        "cci_plain",
        EncryptionPlan {
            method: 0x01,
            fixed_key: false,
        },
    );
}

#[test]
fn decrypts_fixed_key_crypto_without_any_keys() {
    let (plain_bytes, ncch) = plaintext_case("ncch_compressed");
    let mut encrypted = plain_bytes.clone();
    encrypt_ncch_in_place(
        &mut encrypted,
        ncch,
        &TestKeys::generate(3),
        &EncryptionPlan {
            method: 0x00,
            fixed_key: true,
        },
    );
    let mut plain = open(&plain_bytes, &KeyStore::default()).unwrap();
    let mut decrypted = open(&encrypted, &KeyStore::default()).unwrap();
    assert_same_contents(&mut plain, &mut decrypted);
}

#[test]
fn reports_missing_wrong_and_partial_keys() {
    let keys = TestKeys::generate(23);
    let (plain_bytes, ncch) = plaintext_case("ncch_compressed");
    let mut encrypted = plain_bytes.clone();
    encrypt_ncch_in_place(
        &mut encrypted,
        ncch,
        &keys,
        &EncryptionPlan {
            method: 0x01,
            fixed_key: false,
        },
    );
    assert_eq!(
        crypto_error(open(&encrypted, &KeyStore::default())),
        CryptoError::MissingKeys
    );
    assert_eq!(
        crypto_error(open(&encrypted, &TestKeys::generate(99).store())),
        CryptoError::WrongKeys
    );
    let primary_only =
        KeyStore::from_aes_keys_text(&format!("slot0x2CKeyX={}\n", hex(&keys.primary_x)));
    assert_eq!(
        crypto_error(open(&encrypted, &primary_only)),
        CryptoError::MissingKeySlot(SEVEN_X_SLOT)
    );
    let mut seeded = encrypted.clone();
    seeded[ncch + 0x18F] |= 0x20;
    assert_eq!(
        crypto_error(open(&seeded, &keys.store())),
        CryptoError::SeedCrypto
    );
    let message = CryptoError::MissingKeys.message_for("The game update “X.cia”", "update");
    assert!(message.starts_with("The game update “X.cia” is encrypted"));
    assert!(message.contains("aes_keys.txt"));
}

fn cbc_encrypt(data: &mut [u8], key: &[u8; 16], iv: [u8; 16]) {
    let cipher = Aes128::new(key);
    let mut previous = iv;
    for block in data.as_chunks_mut::<16>().0 {
        for (byte, prior) in block.iter_mut().zip(previous) {
            *byte ^= prior;
        }
        let encrypted = cipher.encrypt_block(block);
        *block = encrypted;
        previous = encrypted;
    }
}

fn cia_offsets(cia: &[u8]) -> (usize, usize, usize) {
    let align = |value: usize| value.div_ceil(64) * 64;
    let size = |offset: usize| u32_le(cia, offset) as usize;
    let ticket = align(align(0x2020) + size(0x08));
    let tmd = align(ticket + size(0x0C));
    let content = align(tmd + size(0x10));
    (ticket, tmd, content)
}

fn signed_body(bytes: &[u8], offset: usize) -> usize {
    let signature_type = u32::from_be_bytes(bytes[offset..offset + 4].try_into().unwrap());
    offset
        + 4
        + match signature_type {
            0x010003 => 0x23C,
            0x010004 => 0x13C,
            0x010005 => 0x7C,
            other => panic!("signature type {other:x}"),
        }
}

#[test]
fn decrypts_title_key_encrypted_cia_with_ncch_crypto() {
    let keys = TestKeys::generate(41);
    let (mut plain_bytes, ncch) = plaintext_case("cia_update");
    let (ticket, tmd, content) = cia_offsets(&plain_bytes);
    plain_bytes[ticket..ticket + 4].copy_from_slice(&0x0001_0004u32.to_be_bytes());
    let mut encrypted = plain_bytes.clone();
    encrypt_ncch_in_place(
        &mut encrypted,
        ncch,
        &keys,
        &EncryptionPlan {
            method: 0x00,
            fixed_key: false,
        },
    );
    assert_eq!(content, ncch);
    let ticket_body = signed_body(&encrypted, ticket);
    let tmd_body = signed_body(&encrypted, tmd);
    let chunk = tmd_body + 0xC4 + 0x900;
    let content_size =
        u64::from_be_bytes(encrypted[chunk + 8..chunk + 16].try_into().unwrap()) as usize;
    encrypted[chunk + 6..chunk + 8].copy_from_slice(&1u16.to_be_bytes());
    let title_key = pseudo_random_block(0x7171);
    let mut wrapped_title_key = title_key;
    let mut title_iv = [0u8; 16];
    title_iv[..8].copy_from_slice(&encrypted[ticket_body + 0x9C..ticket_body + 0xA4]);
    cbc_encrypt(&mut wrapped_title_key, &keys.common_key(), title_iv);
    encrypted[ticket_body + 0x7F..ticket_body + 0x8F].copy_from_slice(&wrapped_title_key);
    encrypted[ticket_body + 0xB1] = 0;
    let index = u16::from_be_bytes([encrypted[chunk + 4], encrypted[chunk + 5]]);
    let mut content_iv = [0u8; 16];
    content_iv[..2].copy_from_slice(&index.to_be_bytes());
    let content_end = (content + content_size).min(encrypted.len()) / 16 * 16;
    cbc_encrypt(&mut encrypted[content..content_end], &title_key, content_iv);

    let mut plain = open(&plain_bytes, &KeyStore::default()).unwrap();
    let mut decrypted = open(&encrypted, &keys.store()).unwrap();
    assert_eq!(plain.version(), decrypted.version());
    assert_same_contents(&mut plain, &mut decrypted);
    assert_eq!(
        crypto_error(open(&encrypted, &KeyStore::default())),
        CryptoError::MissingCommonKeys
    );
}

#[test]
fn plaintext_vectors_never_need_keys() {
    for name in ["ncch_compressed", "cci_plain", "cia_update"] {
        let (bytes, _) = plaintext_case(name);
        let ncch = open(&bytes, &KeyStore::default()).unwrap();
        assert!(!ncch.was_decrypted_on_load(), "{name}");
    }
}

fn env_path(name: &str) -> Option<PathBuf> {
    std::env::var_os(name).map(PathBuf::from)
}

#[test]
fn real_keys_scramble_to_their_stored_common_normal_keys() {
    let Some(keys_path) = env_path("RANDOMON_AES_KEYS") else {
        eprintln!("set RANDOMON_AES_KEYS to run");
        return;
    };
    let mut store = KeyStore::default();
    assert!(store.merge_file(&keys_path));
    let mut checked = 0;
    for index in 0..6 {
        if let (Some(derived), Some(stored)) = (
            store.common_key_from_parts(index),
            store.stored_common_normal_key(index),
        ) {
            assert!(
                derived == stored,
                "common{index} did not match its stored normal key"
            );
            checked += 1;
        }
    }
    eprintln!("verified {checked} common keys against the scrambler");
}

#[test]
fn real_encrypted_rom_opens_transparently() {
    let (Some(rom), Some(keys_path)) = (
        env_path("RANDOMON_ENCRYPTED_3DS_ROM"),
        env_path("RANDOMON_AES_KEYS"),
    ) else {
        eprintln!("set RANDOMON_ENCRYPTED_3DS_ROM and RANDOMON_AES_KEYS to run");
        return;
    };
    set_aes_keys_path(Some(keys_path));
    assert!(!discover_keys().is_empty());
    let mut reader = BufReader::new(File::open(&rom).unwrap());
    let product_code = container::product_code(&mut reader).unwrap();
    let title_id = container::title_id(&mut reader).unwrap();
    let mut ncch = Ncch::open(reader, product_code, title_id).unwrap();
    assert!(ncch.was_decrypted_on_load());
    assert!(ncch.is_decrypted().unwrap());
    assert!(!ncch.get_code().unwrap().is_empty());
    let paths: Vec<String> = ncch
        .romfs_paths()
        .iter()
        .map(|path| path.to_string())
        .collect();
    assert!(!paths.is_empty());
    let first_garc = paths.iter().find(|path| path.starts_with("a/")).unwrap();
    assert!(
        ncch.get_file(first_garc)
            .unwrap()
            .unwrap()
            .starts_with(b"CRAG")
    );
    eprintln!(
        "decrypted {} romfs files, version {}",
        paths.len(),
        ncch.version()
    );
}
