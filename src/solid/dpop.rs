///! DPoP (Demonstrating Proof of Possession) Implementation
///!
///! Implements RFC 9449 for cryptographically binding access tokens to agent key pairs.

use crate::Result;
use anyhow::Context;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use chrono::Utc;
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use p256::ecdsa::{SigningKey, VerifyingKey};
use rand_core::OsRng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use uuid::Uuid;

/// DPoP Manager handles key generation and proof creation
pub struct DPoPManager {
    /// ECDSA P-256 signing key
    signing_key: SigningKey,
    /// Corresponding verifying (public) key
    verifying_key: VerifyingKey,
    /// Key ID for rotation tracking
    kid: String,
}

impl DPoPManager {
    /// Generate a new DPoP key pair
    pub fn generate() -> Result<Self> {
        let signing_key = SigningKey::random(&mut OsRng);
        let verifying_key = VerifyingKey::from(&signing_key);
        let kid = Uuid::new_v4().to_string();

        Ok(Self {
            signing_key,
            verifying_key,
            kid,
        })
    }

    /// Create DPoP proof JWT for a specific HTTP request
    ///
    /// # Arguments
    /// * `htm` - HTTP method (e.g., "GET", "POST")
    /// * `htu` - HTTP URI being accessed
    /// * `ath` - Access token hash (optional, for resource server requests)
    pub fn create_proof(
        &self,
        htm: &str,
        htu: &str,
        ath: Option<&str>,
    ) -> Result<String> {
        // Create header with embedded public key
        let mut header = Header::new(Algorithm::ES256);
        header.typ = Some("dpop+jwt".to_string());
        header.kid = Some(self.kid.clone());

        // Embed public key as JWK in header
        let jwk = self.public_jwk();
        header.extra = serde_json::json!({
            "jwk": jwk
        });

        // Create claims
        let claims = DPoPClaims {
            jti: Uuid::new_v4().to_string(),
            htm: htm.to_string(),
            htu: htu.to_string(),
            iat: Utc::now().timestamp(),
            ath: ath.map(String::from),
        };

        // Sign with private key
        let key_bytes = self.signing_key.to_bytes();
        let encoding_key = EncodingKey::from_ec_der(&key_bytes);

        let token = encode(&header, &claims, &encoding_key)
            .context("Failed to encode DPoP proof")?;

        Ok(token)
    }

    /// Get public key as JWK (JSON Web Key)
    fn public_jwk(&self) -> serde_json::Value {
        let encoded_point = self.verifying_key.to_encoded_point(false);

        serde_json::json!({
            "kty": "EC",
            "crv": "P-256",
            "x": URL_SAFE_NO_PAD.encode(encoded_point.x().unwrap()),
            "y": URL_SAFE_NO_PAD.encode(encoded_point.y().unwrap()),
        })
    }

    /// Get key ID
    pub fn kid(&self) -> &str {
        &self.kid
    }

    /// Compute access token hash for DPoP proof
    pub fn compute_ath(access_token: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(access_token.as_bytes());
        let hash = hasher.finalize();
        URL_SAFE_NO_PAD.encode(hash)
    }

    /// Save key to OS keychain (requires keyring feature)
    #[cfg(feature = "solid-integration")]
    pub fn save_to_keychain(&self, service: &str, account: &str) -> Result<()> {
        use keyring::Entry;

        let entry = Entry::new(service, account)
            .context("Failed to create keyring entry")?;

        let key_bytes = self.signing_key.to_bytes();
        let key_b64 = URL_SAFE_NO_PAD.encode(&key_bytes);

        entry.set_password(&key_b64)
            .context("Failed to save key to keychain")?;

        Ok(())
    }

    /// Load key from OS keychain
    #[cfg(feature = "solid-integration")]
    pub fn load_from_keychain(service: &str, account: &str) -> Result<Self> {
        use keyring::Entry;

        let entry = Entry::new(service, account)
            .context("Failed to create keyring entry")?;

        let key_b64 = entry.get_password()
            .context("Failed to retrieve key from keychain")?;

        let key_bytes = URL_SAFE_NO_PAD.decode(&key_b64)
            .context("Failed to decode key")?;

        let key_bytes_array: [u8; 32] = key_bytes.try_into()
            .map_err(|_| anyhow::anyhow!("Invalid key length"))?;

        let signing_key = SigningKey::from_bytes(&key_bytes_array.into())
            .context("Failed to parse signing key")?;

        let verifying_key = VerifyingKey::from(&signing_key);

        // Generate new kid on load
        let kid = Uuid::new_v4().to_string();

        Ok(Self {
            signing_key,
            verifying_key,
            kid,
        })
    }
}

/// DPoP proof claims (JWT payload)
#[derive(Debug, Serialize, Deserialize)]
struct DPoPClaims {
    /// Unique proof ID (prevents replay)
    jti: String,
    /// HTTP method
    htm: String,
    /// HTTP URI
    htu: String,
    /// Issued at (Unix timestamp)
    iat: i64,
    /// Access token hash (for resource server requests)
    #[serde(skip_serializing_if = "Option::is_none")]
    ath: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dpop_generation() {
        let manager = DPoPManager::generate().unwrap();

        let proof = manager.create_proof(
            "GET",
            "https://example.com/resource",
            None,
        ).unwrap();

        // Proof should be a valid JWT
        assert!(proof.split('.').count() == 3);
    }

    #[test]
    fn test_ath_computation() {
        let token = "test_access_token";
        let ath = DPoPManager::compute_ath(token);

        // Should be base64url encoded
        assert!(!ath.contains('+'));
        assert!(!ath.contains('/'));
        assert!(!ath.contains('='));
    }

    #[test]
    fn test_public_jwk() {
        let manager = DPoPManager::generate().unwrap();
        let jwk = manager.public_jwk();

        assert_eq!(jwk["kty"], "EC");
        assert_eq!(jwk["crv"], "P-256");
        assert!(jwk["x"].is_string());
        assert!(jwk["y"].is_string());
    }
}
