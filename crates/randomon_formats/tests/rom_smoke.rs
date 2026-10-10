mod common;

use std::fs::File;
use std::io::{BufReader, Cursor};
use std::path::PathBuf;

use randomon_formats::ctr::container;
use randomon_formats::ctr::{Decompression, GarcArchive, Ncch};
use randomon_formats::gb;

fn rom_path(variable: &str, file_name: &str) -> Option<PathBuf> {
    if let Some(explicit) = std::env::var_os(variable) {
        return Some(PathBuf::from(explicit));
    }
    let directory = PathBuf::from(std::env::var_os("RANDOMON_ROM_DIR")?);
    let candidate = directory.join(file_name);
    candidate.exists().then_some(candidate)
}

fn open_ncch(path: &PathBuf) -> Ncch<BufReader<File>> {
    let mut reader = BufReader::new(File::open(path).unwrap());
    let product_code = container::product_code(&mut reader).unwrap();
    let title_id = container::title_id(&mut reader).unwrap();
    Ncch::open(reader, product_code, title_id).unwrap()
}

#[test]
fn gbc_rom_header_reads() {
    let Some(path) = rom_path("RANDOMON_GBC_ROM", "Pokémon - Yellow Version.gbc") else {
        eprintln!("set RANDOMON_GBC_ROM or RANDOMON_ROM_DIR to run");
        return;
    };
    let rom = std::fs::read(path).unwrap();
    assert!(gb::rom_sig_matches(&rom, "POKEMON YELLOW"));
    assert!(gb::header_checksum_is_valid(&rom));
    assert!(gb::global_checksum_is_valid(&rom));
}

#[test]
fn three_ds_rom_reads_and_parses_garcs() {
    let Some(path) = rom_path("RANDOMON_3DS_ROM", "Pokémon Alpha Sapphire.3ds") else {
        eprintln!("set RANDOMON_3DS_ROM or RANDOMON_ROM_DIR to run");
        return;
    };
    let mut ncch = open_ncch(&path);
    assert!(ncch.is_decrypted().unwrap());
    eprintln!(
        "product {:?} title {:?} version {} compressed code {}",
        ncch.product_code(),
        ncch.title_id(),
        ncch.version(),
        ncch.is_code_compressed()
    );
    let code = ncch.get_code().unwrap();
    eprintln!("code.bin {} bytes", code.len());
    let paths: Vec<String> = ncch
        .romfs_paths()
        .iter()
        .map(|path| path.to_string())
        .collect();
    eprintln!("{} romfs files", paths.len());
    let mut parsed = 0;
    let mut identical = 0;
    let mut rejected = 0;
    for path in paths.iter().filter(|path| path.starts_with("a/")).take(80) {
        let data = ncch.get_file(path).unwrap().unwrap();
        if data.starts_with(b"CRAG") {
            let Ok(garc) = GarcArchive::parse(&data, Decompression::Skip) else {
                rejected += 1;
                continue;
            };
            let rebuilt = garc.to_bytes().unwrap();
            if garc.files.iter().all(|directory| directory.len() <= 1) {
                let reparsed = GarcArchive::parse(&rebuilt, Decompression::Skip).unwrap();
                assert_eq!(reparsed.files, garc.files, "{path}");
                identical += usize::from(rebuilt == data);
            }
            parsed += 1;
        }
    }
    eprintln!(
        "parsed {parsed} GARCs, {identical} rebuilt byte-identically, {rejected} rejected like upstream"
    );
    assert!(parsed > 0);
}

#[test]
fn three_ds_update_cia_reads() {
    let Some(path) = rom_path(
        "RANDOMON_3DS_UPDATE",
        "Pokémon Alpha Sapphire Update v1.4.cia",
    ) else {
        eprintln!("set RANDOMON_3DS_UPDATE or RANDOMON_ROM_DIR to run");
        return;
    };
    let mut ncch = open_ncch(&path);
    eprintln!(
        "update version {} title {:?}",
        ncch.version(),
        ncch.title_id()
    );
    assert!(ncch.version() > 0);
    assert!(ncch.is_decrypted().unwrap());
    let code = ncch.get_code().unwrap();
    assert!(!code.is_empty());
    let _ = Cursor::new(code);
}

fn file_sha256(path: &std::path::Path) -> String {
    use std::io::Read;
    let mut reader = BufReader::new(File::open(path).unwrap());
    let mut hasher = randomon_formats::sha256::Sha256::new();
    let mut buffer = vec![0u8; 1 << 20];
    loop {
        let read = reader.read(&mut buffer).unwrap();
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    hasher
        .finish()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

struct ParityRun<'a> {
    rom: PathBuf,
    modified_file: &'a str,
    touched_file: Option<&'a str>,
    expected_sha256: String,
    expected_layered: Option<PathBuf>,
    output_name: &'a str,
}

fn run_save_parity(run: ParityRun<'_>) {
    let scratch =
        std::env::var_os("RANDOMON_SCRATCH_DIR").map_or_else(std::env::temp_dir, PathBuf::from);
    let mut ncch = open_ncch(&run.rom);
    let file = ncch.get_file(run.modified_file).unwrap().unwrap();
    let mut modified = file.clone();
    modified.resize(file.len() + 16, 0);
    for index in (0..modified.len()).step_by(33) {
        modified[index] ^= 0x5A;
    }
    ncch.write_file(run.modified_file, &modified).unwrap();
    if let Some(touched) = run.touched_file {
        ncch.get_file(touched).unwrap();
    }
    let mut code = ncch.get_code().unwrap();
    for index in (0x1000..code.len()).step_by(4099) {
        code[index] ^= 0x01;
    }
    ncch.write_code(&code).unwrap();
    let output_path = scratch.join(format!("{}.3ds", run.output_name));
    let output = File::options()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(&output_path)
        .unwrap();
    let started = std::time::Instant::now();
    ncch.save_as_ncch(output, "ORAS", 987654321).unwrap();
    eprintln!("saved {} in {:?}", run.output_name, started.elapsed());
    let digest = file_sha256(&output_path);
    let _ = std::fs::remove_file(&output_path);
    assert_eq!(digest, run.expected_sha256);
    if let Some(expected_layered) = run.expected_layered {
        let title = ncch.title_id().unwrap().to_owned();
        let entries = ncch.layered_fs_entries().unwrap();
        assert!(!entries.is_empty());
        for (relative, data) in entries {
            let expected = std::fs::read(expected_layered.join(&title).join(&relative)).unwrap();
            assert!(expected == data, "{}", relative.display());
        }
    }
}

fn expected_digest(variable: &str) -> Option<String> {
    std::env::var_os(variable).map(|value| value.to_string_lossy().into_owned())
}

#[test]
fn three_ds_save_matches_upstream_digest() {
    let (Some(rom), Some(expected_sha256)) = (
        rom_path("RANDOMON_3DS_ROM", "Pokémon Alpha Sapphire.3ds"),
        expected_digest("RANDOMON_3DS_EXPECTED_SHA256"),
    ) else {
        eprintln!("set RANDOMON_3DS_ROM and RANDOMON_3DS_EXPECTED_SHA256 to run");
        return;
    };
    run_save_parity(ParityRun {
        rom,
        modified_file: "a/0/1/1",
        touched_file: Some("a/0/0/1"),
        expected_sha256,
        expected_layered: std::env::var_os("RANDOMON_3DS_EXPECTED_LAYERED").map(PathBuf::from),
        output_name: "randomon_base_out",
    });
}

#[test]
fn three_ds_update_save_matches_upstream_digest() {
    let (Some(rom), Some(expected_sha256)) = (
        rom_path(
            "RANDOMON_3DS_UPDATE",
            "Pokémon Alpha Sapphire Update v1.4.cia",
        ),
        expected_digest("RANDOMON_3DS_UPDATE_EXPECTED_SHA256"),
    ) else {
        eprintln!("set RANDOMON_3DS_UPDATE and RANDOMON_3DS_UPDATE_EXPECTED_SHA256 to run");
        return;
    };
    run_save_parity(ParityRun {
        rom,
        modified_file: "DllBattle.cro",
        touched_file: None,
        expected_sha256,
        expected_layered: std::env::var_os("RANDOMON_3DS_UPDATE_EXPECTED_LAYERED")
            .map(PathBuf::from),
        output_name: "randomon_update_out",
    });
}
