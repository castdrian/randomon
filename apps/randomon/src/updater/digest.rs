use std::io::Read;
use std::path::Path;

use sha2::Digest;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sha256([u8; 32]);

impl Sha256 {
    pub fn to_hex(self) -> String {
        self.0.iter().map(|byte| format!("{byte:02x}")).collect()
    }
}

pub fn parse_github_digest(digest: &str) -> Option<Sha256> {
    let hex = digest.trim().strip_prefix("sha256:")?;
    if hex.len() != 64 {
        return None;
    }
    let mut bytes = [0u8; 32];
    for (index, byte) in bytes.iter_mut().enumerate() {
        *byte = u8::from_str_radix(hex.get(index * 2..index * 2 + 2)?, 16).ok()?;
    }
    Some(Sha256(bytes))
}

pub fn of_reader(mut reader: impl Read) -> std::io::Result<Sha256> {
    let mut hasher = sha2::Sha256::new();
    let mut buffer = vec![0u8; 64 * 1024];
    loop {
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            return Ok(Sha256(hasher.finalize().into()));
        }
        hasher.update(&buffer[..read]);
    }
}

pub fn of_file(path: &Path) -> std::io::Result<Sha256> {
    of_reader(std::fs::File::open(path)?)
}

pub fn verify(actual: Sha256, expected: Sha256) -> Result<(), String> {
    if actual == expected {
        Ok(())
    } else {
        Err(format!(
            "The download is corrupted: its SHA-256 is {} but GitHub lists {}.",
            actual.to_hex(),
            expected.to_hex()
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ABC: &str = "sha256:ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";

    #[test]
    fn parses_github_digests() {
        let digest = parse_github_digest(ABC).unwrap();
        assert_eq!(
            digest.to_hex(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert!(parse_github_digest("md5:0123").is_none());
        assert!(parse_github_digest("sha256:abc").is_none());
        assert!(
            parse_github_digest(
                "sha256:zz7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
            )
            .is_none()
        );
    }

    #[test]
    fn verifies_matching_content() {
        let expected = parse_github_digest(ABC).unwrap();
        assert!(verify(of_reader(&b"abc"[..]).unwrap(), expected).is_ok());
        let error = verify(of_reader(&b"abd"[..]).unwrap(), expected).unwrap_err();
        assert!(error.contains("corrupted"));
    }

    #[test]
    fn hashes_files() {
        let path = std::env::temp_dir().join(format!("randomon-digest-{}", std::process::id()));
        std::fs::write(&path, b"abc").unwrap();
        assert_eq!(of_file(&path).unwrap(), parse_github_digest(ABC).unwrap());
        let _ = std::fs::remove_file(&path);
    }
}
