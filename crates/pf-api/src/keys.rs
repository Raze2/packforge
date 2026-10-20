//! API keys: stored only as SHA-256 hashes in a TOML file.
//!
//! ```toml
//! [[key]]
//! id = "shop-prod"
//! hash = "sha256:9f86d081884c7d65..."
//! plan = "enterprise"
//! rate_per_minute = 600
//! ```

use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct KeyEntry {
    pub id: String,
    pub hash: String,
    #[serde(default = "default_plan")]
    pub plan: String,
    #[serde(default = "default_rate")]
    pub rate_per_minute: u32,
}

fn default_plan() -> String {
    "enterprise".into()
}

fn default_rate() -> u32 {
    120
}

#[derive(Debug, Deserialize)]
struct KeysFile {
    #[serde(default, rename = "key")]
    keys: Vec<KeyEntry>,
}

#[derive(Debug, thiserror::Error)]
pub enum KeysError {
    #[error("cannot parse keys file: {0}")]
    Parse(String),
    #[error("key '{id}': {message}")]
    Invalid { id: String, message: String },
}

/// Lookup table from key hash to key entry.
#[derive(Debug, Clone, Default)]
pub struct KeyStore {
    by_hash: HashMap<String, KeyEntry>,
}

impl KeyStore {
    pub fn from_toml(src: &str) -> Result<Self, KeysError> {
        let file: KeysFile = toml::from_str(src).map_err(|e| KeysError::Parse(e.to_string()))?;
        Self::from_entries(file.keys)
    }

    pub fn from_entries(entries: Vec<KeyEntry>) -> Result<Self, KeysError> {
        let mut by_hash = HashMap::new();
        for e in entries {
            let invalid = |message: &str| KeysError::Invalid {
                id: e.id.clone(),
                message: message.into(),
            };
            let hex = e
                .hash
                .strip_prefix("sha256:")
                .ok_or_else(|| invalid("hash must start with 'sha256:'"))?;
            if hex.len() != 64 || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
                return Err(invalid("hash must be 64 hex characters"));
            }
            if e.rate_per_minute == 0 {
                return Err(invalid("rate_per_minute must be at least 1"));
            }
            let key = hex.to_ascii_lowercase();
            if by_hash.insert(key, e.clone()).is_some() {
                return Err(invalid("duplicate hash"));
            }
        }
        Ok(Self { by_hash })
    }

    pub fn len(&self) -> usize {
        self.by_hash.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_hash.is_empty()
    }

    /// Finds the entry for a presented secret key.
    pub fn verify(&self, secret: &str) -> Option<&KeyEntry> {
        self.by_hash.get(&sha256_hex(secret.as_bytes()))
    }
}

/// Lower-case hex SHA-256 digest.
pub fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// Generates a new random secret, e.g. `pk_live_3J9...` (32 random bytes, base32).
pub fn generate_secret(prefix: &str) -> Result<String, getrandom::Error> {
    const ALPHABET: &[u8] = b"abcdefghijklmnopqrstuvwxyz234567";
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes)?;
    let body: String = bytes
        .iter()
        .map(|b| ALPHABET[usize::from(b % 32)] as char)
        .collect();
    Ok(format!("{prefix}_{body}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verifies_hashed_keys() {
        let secret = "pk_test_abc";
        let src = format!(
            "[[key]]\nid = \"t\"\nhash = \"sha256:{}\"\nplan = \"test\"\nrate_per_minute = 5\n",
            sha256_hex(secret.as_bytes())
        );
        let store = KeyStore::from_toml(&src).expect("parse");
        let e = store.verify(secret).expect("found");
        assert_eq!((e.id.as_str(), e.rate_per_minute), ("t", 5));
        assert!(store.verify("pk_test_wrong").is_none());
    }

    #[test]
    fn rejects_bad_hashes() {
        let bad = "[[key]]\nid = \"x\"\nhash = \"md5:abc\"\n";
        assert!(KeyStore::from_toml(bad).is_err());
        let short = "[[key]]\nid = \"x\"\nhash = \"sha256:abc\"\n";
        assert!(KeyStore::from_toml(short).is_err());
    }

    #[test]
    fn generated_secrets_are_unique() {
        let a = generate_secret("pk_live").expect("rng");
        let b = generate_secret("pk_live").expect("rng");
        assert!(a.starts_with("pk_live_") && a.len() == 8 + 32);
        assert_ne!(a, b);
    }
}
