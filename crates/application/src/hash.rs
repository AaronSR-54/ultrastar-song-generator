use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

use serde::Serialize;
use sha2::{Digest, Sha256};

/// SHA-256 of an in-memory buffer, as lowercase hexadecimal.
pub fn hash_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex(&hasher.finalize())
}

/// SHA-256 of a serializable value using its JSON representation.
pub fn hash_json<T: Serialize>(value: &T) -> Result<String, serde_json::Error> {
    let bytes = serde_json::to_vec(value)?;
    Ok(hash_bytes(&bytes))
}

/// Streaming SHA-256 of a file.
pub fn hash_file(path: impl AsRef<Path>) -> io::Result<String> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hex(&hasher.finalize()))
}

fn hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write as _;
        let _ = write!(output, "{byte:02x}");
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_sha256_values() {
        assert_eq!(
            hash_bytes(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            hash_bytes(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn same_input_hashes_equal_and_different_input_changes() {
        assert_eq!(hash_bytes(b"hello"), hash_bytes(b"hello"));
        assert_ne!(hash_bytes(b"hello"), hash_bytes(b"Hello"));
    }

    #[test]
    fn file_hash_matches_memory_hash() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("data.bin");
        std::fs::write(&path, b"hello world").unwrap();
        assert_eq!(hash_file(&path).unwrap(), hash_bytes(b"hello world"));
    }

    #[test]
    fn json_hash_is_stable() {
        #[derive(serde::Serialize)]
        struct Payload {
            language: String,
            model: String,
        }
        let payload = Payload {
            language: "Spanish".into(),
            model: "medium".into(),
        };
        assert_eq!(hash_json(&payload).unwrap(), hash_json(&payload).unwrap());
    }
}
