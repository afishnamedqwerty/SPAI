///! Solid-OIDC Authentication Client
///!
///! Hybrid implementation: DPoP in Rust, OIDC protocol flow via TypeScript bridge.

use crate::Result;
use crate::solid::dpop::DPoPManager;
use crate::solid::identity::SolidIdentityClient;
use anyhow::Context;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use url::Url;

/// DPoP-bound access token
#[derive(Debug, Clone)]
pub struct DPoPBoundToken {
    /// Access token
    pub access_token: String,
    /// Refresh token (optional)
    pub refresh_token: Option<String>,
    /// ID token
    pub id_token: String,
    /// Expiration time
    pub expires_at: DateTime<Utc>,
    /// Whether token is DPoP-bound
    pub dpop_bound: bool,
}

impl DPoPBoundToken {
    /// Check if token is expired or about to expire
    pub fn is_expired(&self) -> bool {
        // Consider expired if less than 5 minutes remaining
        Utc::now() + Duration::minutes(5) >= self.expires_at
    }
}

/// Solid-OIDC authentication client
pub struct SolidOidcClient {
    /// Identity client (provides WebID and IPC bridge)
    identity_client: Arc<SolidIdentityClient>,
    /// DPoP manager for proof generation
    dpop_manager: Arc<DPoPManager>,
    /// Token cache (keyed by OIDC issuer)
    token_cache: Arc<RwLock<HashMap<String, DPoPBoundToken>>>,
}

impl SolidOidcClient {
    /// Create new OIDC client
    pub fn new(
        identity_client: Arc<SolidIdentityClient>,
        dpop_manager: Arc<DPoPManager>,
    ) -> Self {
        Self {
            identity_client,
            dpop_manager,
            token_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Authenticate to a resource server
    ///
    /// This method:
    /// 1. Discovers the OIDC issuer from the resource server
    /// 2. Checks cache for valid token
    /// 3. If needed, performs OIDC flow via TypeScript bridge
    /// 4. Returns DPoP-bound access token
    pub async fn authenticate(&self, resource_url: &Url) -> Result<DPoPBoundToken> {
        // 1. Get WebID profile to find OIDC issuer
        let profile = self.identity_client.fetch_webid_profile()
            .context("Failed to fetch WebID profile")?;

        let issuer_url = profile.require_oidc_issuer()
            .context("WebID profile must specify OIDC issuer")?;

        let issuer_str = issuer_url.as_str().to_string();

        // 2. Check cache for valid token
        {
            let cache = self.token_cache.read().unwrap();
            if let Some(token) = cache.get(&issuer_str) {
                if !token.is_expired() {
                    return Ok(token.clone());
                }
            }
        }

        // 3. Perform OIDC authentication flow
        let token = self.perform_oidc_flow(&issuer_url).await?;

        // 4. Cache token
        {
            let mut cache = self.token_cache.write().unwrap();
            cache.insert(issuer_str, token.clone());
        }

        Ok(token)
    }

    /// Perform OIDC authentication flow
    async fn perform_oidc_flow(&self, issuer: &Url) -> Result<DPoPBoundToken> {
        // Generate DPoP proof for token endpoint
        // Note: We don't know the token endpoint yet, so we use issuer + /token
        let token_endpoint = format!("{}/token", issuer.as_str().trim_end_matches('/'));

        let dpop_proof = self.dpop_manager.create_proof(
            "POST",
            &token_endpoint,
            None, // No access token hash for initial request
        ).context("Failed to create DPoP proof")?;

        // Delegate OIDC flow to TypeScript bridge
        let params = serde_json::json!({
            "issuer": issuer.as_str(),
            "clientId": self.identity_client.client_id().as_str(),
            "redirectUri": "http://localhost:3000/callback", // TODO: Make configurable
            "dpopProof": dpop_proof,
        });

        let ipc = &self.identity_client.ipc;
        let mut ipc_guard = ipc.lock().unwrap();
        let response = ipc_guard.request("authenticate", params)
            .context("OIDC authentication failed")?;

        // Parse response
        let auth_response: AuthResponse = serde_json::from_value(response)
            .context("Failed to parse authentication response")?;

        // Calculate expiration time
        let expires_at = Utc::now() + Duration::seconds(3600); // Default 1 hour

        Ok(DPoPBoundToken {
            access_token: auth_response.access_token.unwrap_or_default(),
            refresh_token: auth_response.refresh_token,
            id_token: auth_response.id_token.unwrap_or_default(),
            expires_at,
            dpop_bound: true,
        })
    }

    /// Make authenticated HTTP request with DPoP
    pub async fn authenticated_request(
        &self,
        method: &str,
        url: &Url,
    ) -> Result<reqwest::Response> {
        // Get access token
        let token = self.authenticate(url).await?;

        // Create DPoP proof for this specific request
        let ath = DPoPManager::compute_ath(&token.access_token);
        let dpop_proof = self.dpop_manager.create_proof(
            method,
            url.as_str(),
            Some(&ath),
        ).context("Failed to create DPoP proof")?;

        // Make HTTP request with DPoP headers
        let client = reqwest::Client::new();
        let response = client
            .request(
                method.parse().context("Invalid HTTP method")?,
                url.clone(),
            )
            .header("Authorization", format!("DPoP {}", token.access_token))
            .header("DPoP", dpop_proof)
            .send()
            .await
            .context("HTTP request failed")?;

        Ok(response)
    }

    /// Clear token cache (force re-authentication)
    pub fn clear_cache(&self) {
        let mut cache = self.token_cache.write().unwrap();
        cache.clear();
    }
}

/// Authentication response from TypeScript bridge
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AuthResponse {
    is_logged_in: bool,
    web_id: Option<String>,
    session_id: String,
    access_token: Option<String>,
    refresh_token: Option<String>,
    id_token: Option<String>,
    expires_at: Option<i64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_expiration() {
        let token = DPoPBoundToken {
            access_token: "test".to_string(),
            refresh_token: None,
            id_token: "test".to_string(),
            expires_at: Utc::now() - Duration::minutes(10), // Expired 10 min ago
            dpop_bound: true,
        };

        assert!(token.is_expired());
    }

    #[test]
    fn test_token_not_expired() {
        let token = DPoPBoundToken {
            access_token: "test".to_string(),
            refresh_token: None,
            id_token: "test".to_string(),
            expires_at: Utc::now() + Duration::hours(1), // Expires in 1 hour
            dpop_bound: true,
        };

        assert!(!token.is_expired());
    }
}
