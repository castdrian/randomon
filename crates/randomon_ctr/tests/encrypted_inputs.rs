use randomon_core::RandomizationError;
use randomon_ctr::CtrRom;
use randomon_ctr::rom::{RomInput, bytes_source};
use randomon_formats::FormatError;
use randomon_formats::ctr::crypto::{CryptoError, set_aes_keys_path};

const BASE_NAME: &str = "/roms/Pokémon X.3ds";
const UPDATE_NAME: &str = "/roms/Pokémon X Update v1.5.cia";

fn encrypted_looking_cxi() -> Vec<u8> {
    let mut image = vec![0u8; 0x1000];
    image[0x100..0x104].copy_from_slice(b"NCCH");
    image[0x1A0..0x1A4].copy_from_slice(&4u32.to_le_bytes());
    image[0x1A4..0x1A8].copy_from_slice(&1u32.to_le_bytes());
    image[0x800..0xA00].fill(0xA5);
    image
}

fn message(error: RandomizationError) -> String {
    error.to_string()
}

#[test]
fn crypto_messages_name_the_input_at_fault() {
    let base = message(
        RomInput::BaseRom.load_error(BASE_NAME, FormatError::Crypto(CryptoError::MissingKeys)),
    );
    assert_eq!(
        base,
        "The ROM \u{201C}Pokémon X.3ds\u{201D} is encrypted and no 3DS keys were found. Point randomon to an emulator's aes_keys.txt or use a decrypted ROM."
    );
    let update = message(
        RomInput::GameUpdate.load_error(UPDATE_NAME, FormatError::Crypto(CryptoError::MissingKeys)),
    );
    assert_eq!(
        update,
        "The game update \u{201C}Pokémon X Update v1.5.cia\u{201D} is encrypted and no 3DS keys were found. Point randomon to an emulator's aes_keys.txt or use a decrypted update."
    );
    let seeded = message(
        RomInput::GameUpdate.load_error(UPDATE_NAME, FormatError::Crypto(CryptoError::SeedCrypto)),
    );
    assert!(seeded.starts_with("The game update \u{201C}Pokémon X Update v1.5.cia\u{201D}"));
    let unreadable =
        message(RomInput::BaseRom.load_error(BASE_NAME, FormatError::invalid("NCCH", "truncated")));
    assert!(unreadable.starts_with("The ROM \u{201C}Pokémon X.3ds\u{201D} could not be read"));
}

#[test]
fn opening_encrypted_inputs_without_keys_blames_the_right_file() {
    set_aes_keys_path(Some(
        std::env::temp_dir().join("randomon-no-such-aes-keys.txt"),
    ));
    let base_error = CtrRom::open(bytes_source(encrypted_looking_cxi()), BASE_NAME)
        .err()
        .map(message)
        .expect("encrypted base rom is rejected");
    assert!(
        base_error.starts_with(
            "The ROM \u{201C}Pokémon X.3ds\u{201D} is encrypted and no 3DS keys were found."
        ),
        "{base_error}"
    );
    if let Some(base_rom) = std::env::var_os("RANDOMON_GEN6_ROM") {
        let source = randomon_ctr::rom::open_file_source(std::path::Path::new(&base_rom))
            .expect("base rom opens");
        let mut rom = CtrRom::open(source, BASE_NAME).expect("decrypted base rom loads");
        let update_error = rom
            .load_named_update(bytes_source(encrypted_looking_cxi()), UPDATE_NAME)
            .err()
            .map(message)
            .expect("encrypted update is rejected");
        assert!(
            update_error.starts_with(
                "The game update \u{201C}Pokémon X Update v1.5.cia\u{201D} is encrypted and no 3DS keys were found."
            ),
            "{update_error}"
        );
        assert!(!rom.has_update());
    }
    set_aes_keys_path(None);
}
