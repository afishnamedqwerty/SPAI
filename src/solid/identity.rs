///! WebID Identity Management
///!
///! Handles WebID profile fetching and Client ID document management.

use crate::Result;
use crate::solid::ipc::IpcChannel;
use anyhow::Context;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use url::Url;

/// Solid Identity Client
///
/// Manages WebID profiles and Client ID documents via TypeScript bridge.
pub struct SolidIdentityClient {
    /// Agent's WebID
    webid: Url,
    /// Agent's Client ID Document IRI
    client_id: Url,
    /// IPC channel to TypeScript bridge
    ipc: Arc<Mutex<IpcChannel>>,
}

impl SolidIdentityClient {
    /// Create new identity client
    ///
    /// # Arguments
    /// * `webid` - The WebID IRI for this agent
    /// * `client_id` - The Client ID Document IRI
    /// * `bridge_path` - Path to TypeScript bridge executable
    pub fn new(webid: Url, client_id: Url, bridge_path: PathBuf) -> Result<Self> {
        let ipc = IpcChannel::spawn(bridge_path)
            .context("Failed to spawn Solid identity bridge")?;

        Ok(Self {
            webid,
            client_id,
            ipc: Arc::new(Mutex::new(ipc)),
        })
    }

    /// Fetch WebID profile document
    pub fn fetch_webid_profile(&self) -> Result<WebIdProfile> {
        let params = serde_json::json!({
            "webid": self.webid.as_str()
        });

        let mut ipc = self.ipc.lock().unwrap();
        let response = ipc.request("fetchProfile", params)
            .context("Failed to fetch WebID profile")?;

        let profile: WebIdProfile = serde_json::from_value(response)
            .context("Failed to parse WebID profile")?;

        Ok(profile)
    }

    /// Get the WebID for this client
    pub fn webid(&self) -> &Url {
        &self.webid
    }

    /// Get the Client ID for this client
    pub fn client_id(&self) -> &Url {
        &self.client_id
    }

    /// Shutdown the identity client
    pub fn shutdown(self) -> Result<()> {
        let ipc = Arc::try_unwrap(self.ipc)
            .map_err(|_| anyhow::anyhow!("Cannot shutdown: IPC channel still in use"))?
            .into_inner()
            .unwrap();

        ipc.shutdown()
    }
}

/// WebID Profile information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebIdProfile {
    /// The WebID IRI
    pub webid: String,
    /// User's name (optional)
    pub name: Option<String>,
    /// OIDC Issuer IRI
    pub oidc_issuer: Option<String>,
    /// Storage location (Pod IRI)
    pub storage: Option<String>,
    /// Inbox location
    pub inbox: Option<String>,
}

impl WebIdProfile {
    /// Get the OIDC issuer, or return error if not set
    pub fn require_oidc_issuer(&self) -> Result<Url> {
        self.oidc_issuer
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("WebID profile does not specify an OIDC issuer"))
            .and_then(|s| Url::parse(s).context("Invalid OIDC issuer URL"))
    }

    /// Get the storage location (Pod IRI)
    pub fn require_storage(&self) -> Result<Url> {
        self.storage
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("WebID profile does not specify a storage location"))
            .and_then(|s| Url::parse(s).context("Invalid storage URL"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_webid_profile_parsing() {
        let json = serde_json::json!({
            "webid": "https://alice.example/profile/card#me",
            "name": "Alice",
            "oidcIssuer": "https://idp.example/",
            "storage": "https://alice.example/",
            "inbox": "https://alice.example/inbox/"
        });

        let profile: WebIdProfile = serde_json::from_value(json).unwrap();

        assert_eq!(profile.webid, "https://alice.example/profile/card#me");
        assert_eq!(profile.name, Some("Alice".to_string()));
        assert!(profile.require_oidc_issuer().is_ok());
        assert!(profile.require_storage().is_ok());
    }
}
