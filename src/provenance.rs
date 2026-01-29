//! Cryptographic provenance for decision audit trails
//!
//! Every decision gets:
//! - SHA-256 content hash
//! - Ed25519 signature
//! - Chain link to previous decision
//!
//! This enables:
//! - Tamper detection
//! - Decision auditing
//! - Accountability trails

use anyhow::{Context, Result};
use ed25519_dalek::{Signer, SigningKey, VerifyingKey, Signature, Verifier};
use rand::rngs::OsRng;
use sha2::{Sha256, Digest};
use std::fs;
use std::path::Path;

/// Provenance manager for cryptographic operations
pub struct Provenance {
    signing_key: SigningKey,
}

impl Provenance {
    /// Create or load provenance keys
    pub fn init(key_path: &Path) -> Result<Self> {
        let signing_key = if key_path.exists() {
            Self::load_key(key_path)?
        } else {
            let key = Self::generate_key();
            Self::save_key(&key, key_path)?;
            key
        };

        Ok(Self { signing_key })
    }

    /// Generate a new Ed25519 signing key
    fn generate_key() -> SigningKey {
        SigningKey::generate(&mut OsRng)
    }

    /// Load key from file
    fn load_key(path: &Path) -> Result<SigningKey> {
        let bytes = fs::read(path)
            .with_context(|| format!("Failed to read key from {:?}", path))?;

        if bytes.len() != 32 {
            anyhow::bail!("Invalid key length: expected 32 bytes, got {}", bytes.len());
        }

        let mut key_bytes = [0u8; 32];
        key_bytes.copy_from_slice(&bytes);

        Ok(SigningKey::from_bytes(&key_bytes))
    }

    /// Save key to file
    fn save_key(key: &SigningKey, path: &Path) -> Result<()> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        fs::write(path, key.to_bytes())
            .with_context(|| format!("Failed to write key to {:?}", path))?;

        // Set restrictive permissions on Unix
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(path)?.permissions();
            perms.set_mode(0o600);
            fs::set_permissions(path, perms)?;
        }

        Ok(())
    }

    /// Get public key as hex string
    pub fn public_key_hex(&self) -> String {
        hex::encode(self.signing_key.verifying_key().as_bytes())
    }

    /// Hash content with SHA-256
    pub fn hash(&self, content: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(content);
        hex::encode(hasher.finalize())
    }

    /// Sign content with Ed25519
    pub fn sign(&self, content: &[u8]) -> Result<String> {
        let signature = self.signing_key.sign(content);
        Ok(hex::encode(signature.to_bytes()))
    }

    /// Verify a signature
    pub fn verify(&self, content: &[u8], signature_hex: &str, pubkey_hex: &str) -> Result<bool> {
        let sig_bytes = hex::decode(signature_hex)
            .context("Invalid signature hex")?;

        let pubkey_bytes = hex::decode(pubkey_hex)
            .context("Invalid public key hex")?;

        if sig_bytes.len() != 64 {
            anyhow::bail!("Invalid signature length");
        }

        if pubkey_bytes.len() != 32 {
            anyhow::bail!("Invalid public key length");
        }

        let mut sig_arr = [0u8; 64];
        sig_arr.copy_from_slice(&sig_bytes);
        let signature = Signature::from_bytes(&sig_arr);

        let mut pubkey_arr = [0u8; 32];
        pubkey_arr.copy_from_slice(&pubkey_bytes);
        let verifying_key = VerifyingKey::from_bytes(&pubkey_arr)
            .context("Invalid public key")?;

        Ok(verifying_key.verify(content, &signature).is_ok())
    }

    /// Verify a hash chain
    pub fn verify_chain(&self, chain: &[ChainLink]) -> ChainVerification {
        let mut errors = Vec::new();
        let mut prev_hash: Option<&str> = None;

        for (i, link) in chain.iter().enumerate() {
            // Verify hash chain continuity
            if let Some(expected_prev) = prev_hash {
                if link.previous_hash.as_deref() != Some(expected_prev) {
                    errors.push(format!(
                        "Chain break at position {}: expected prev_hash {:?}, got {:?}",
                        i, expected_prev, link.previous_hash
                    ));
                }
            }

            // Verify content hash matches
            let computed_hash = self.hash(&link.content);
            if computed_hash != link.content_hash {
                errors.push(format!(
                    "Hash mismatch at position {}: computed {}, stored {}",
                    i, computed_hash, link.content_hash
                ));
            }

            // Verify signature
            match self.verify(&link.content, &link.signature, &link.agent_pubkey) {
                Ok(true) => {}
                Ok(false) => {
                    errors.push(format!("Invalid signature at position {}", i));
                }
                Err(e) => {
                    errors.push(format!("Signature verification error at position {}: {}", i, e));
                }
            }

            prev_hash = Some(&link.content_hash);
        }

        ChainVerification {
            valid: errors.is_empty(),
            errors,
            chain_length: chain.len(),
        }
    }
}

/// A link in the provenance chain
#[derive(Debug)]
pub struct ChainLink {
    pub content: Vec<u8>,
    pub content_hash: String,
    pub previous_hash: Option<String>,
    pub signature: String,
    pub agent_pubkey: String,
}

/// Result of chain verification
#[derive(Debug)]
pub struct ChainVerification {
    pub valid: bool,
    pub errors: Vec<String>,
    pub chain_length: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_key_generation_and_signing() {
        let dir = tempdir().unwrap();
        let key_path = dir.path().join("test.key");

        let prov = Provenance::init(&key_path).unwrap();

        let content = b"test content";
        let signature = prov.sign(content).unwrap();
        let pubkey = prov.public_key_hex();

        // Verify the signature
        let valid = prov.verify(content, &signature, &pubkey).unwrap();
        assert!(valid);

        // Tampered content should fail
        let tampered = b"tampered content";
        let valid = prov.verify(tampered, &signature, &pubkey).unwrap();
        assert!(!valid);
    }

    #[test]
    fn test_key_persistence() {
        let dir = tempdir().unwrap();
        let key_path = dir.path().join("persist.key");

        // Create first instance
        let prov1 = Provenance::init(&key_path).unwrap();
        let pubkey1 = prov1.public_key_hex();

        // Create second instance - should load same key
        let prov2 = Provenance::init(&key_path).unwrap();
        let pubkey2 = prov2.public_key_hex();

        assert_eq!(pubkey1, pubkey2);
    }

    #[test]
    fn test_hash() {
        let dir = tempdir().unwrap();
        let key_path = dir.path().join("hash.key");
        let prov = Provenance::init(&key_path).unwrap();

        let hash1 = prov.hash(b"test");
        let hash2 = prov.hash(b"test");
        let hash3 = prov.hash(b"different");

        assert_eq!(hash1, hash2);
        assert_ne!(hash1, hash3);
        assert_eq!(hash1.len(), 64); // SHA-256 = 32 bytes = 64 hex chars
    }
}
