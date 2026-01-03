///! Consent Manifest Parser
///!
///! Manages user consent policies stored as RDF on Solid Pods.
///! Uses oxigraph for efficient SPARQL queries.

use crate::Result;
use crate::solid::identity::SolidIdentityClient;
use anyhow::Context;
use oxigraph::model::{NamedNode, Term};
use oxigraph::sparql::{Query, QueryResults};
use oxigraph::store::Store;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use url::Url;

/// Consent value for cookie/beacon categories
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConsentValue {
    /// Allow this category
    Allow,
    /// Deny this category
    Deny,
    /// Ask user for permission
    AskUser,
}

impl ConsentValue {
    fn from_str(s: &str) -> Self {
        match s {
            "Allow" | "allow" | "ALLOW" => ConsentValue::Allow,
            "Deny" | "deny" | "DENY" => ConsentValue::Deny,
            "AskUser" | "askUser" | "ASK_USER" => ConsentValue::AskUser,
            _ => ConsentValue::Deny, // Default to deny for safety
        }
    }
}

/// Domain-specific consent policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainConsentPolicy {
    /// Domain name
    pub domain: String,
    /// Essential cookies (login, cart, etc.)
    pub essential: ConsentValue,
    /// Functional cookies (preferences, language)
    pub functional: ConsentValue,
    /// Analytics cookies (tracking, metrics)
    pub analytics: ConsentValue,
    /// Advertising cookies (personalization, cross-site)
    pub advertising: ConsentValue,
}

impl DomainConsentPolicy {
    /// Check if domain is completely blocked
    pub fn is_blocked(&self) -> bool {
        self.essential == ConsentValue::Deny
            && self.functional == ConsentValue::Deny
            && self.analytics == ConsentValue::Deny
            && self.advertising == ConsentValue::Deny
    }

    /// Check if any category requires user approval
    pub fn requires_user_approval(&self) -> bool {
        self.essential == ConsentValue::AskUser
            || self.functional == ConsentValue::AskUser
            || self.analytics == ConsentValue::AskUser
            || self.advertising == ConsentValue::AskUser
    }

    /// Get categories that require user approval
    pub fn unconfigured_categories(&self) -> Vec<String> {
        let mut categories = Vec::new();
        if self.essential == ConsentValue::AskUser {
            categories.push("essential".to_string());
        }
        if self.functional == ConsentValue::AskUser {
            categories.push("functional".to_string());
        }
        if self.analytics == ConsentValue::AskUser {
            categories.push("analytics".to_string());
        }
        if self.advertising == ConsentValue::AskUser {
            categories.push("advertising".to_string());
        }
        categories
    }
}

/// Consent manifest manager
pub struct ConsentManifest {
    /// User's Pod IRI
    pod_iri: Url,
    /// Local RDF store (oxigraph) for fast queries
    store: Arc<RwLock<Store>>,
    /// Bridge to Solid Pod for updates
    identity_client: Arc<SolidIdentityClient>,
    /// Last fetch timestamp
    last_fetched: RwLock<Option<Instant>>,
    /// Cache duration
    cache_duration: Duration,
    /// Default policy
    default_policy: DomainConsentPolicy,
}

impl ConsentManifest {
    /// Load consent manifest from user's Pod
    pub async fn load(
        pod_iri: Url,
        identity_client: Arc<SolidIdentityClient>,
    ) -> Result<Self> {
        let manifest_url = format!("{}/consents/browser-agent.ttl", pod_iri.as_str().trim_end_matches('/'));

        // Fetch RDF document from Pod via TypeScript bridge
        let params = serde_json::json!({
            "url": manifest_url,
            "contentType": "text/turtle"
        });

        let ipc = &identity_client.ipc;
        let mut ipc_guard = ipc.lock().unwrap();
        let response = ipc_guard.request("fetchResource", params)
            .context("Failed to fetch consent manifest")?;

        let content = response["content"]
            .as_str()
            .context("Missing content in response")?;

        // Parse into oxigraph store
        let store = Store::new().context("Failed to create RDF store")?;

        // For now, create an empty store
        // In production, we'd parse the Turtle content
        // store.load_from_reader(...)?

        // Default policy: allow essential, deny everything else
        let default_policy = DomainConsentPolicy {
            domain: "*".to_string(),
            essential: ConsentValue::Allow,
            functional: ConsentValue::Deny,
            analytics: ConsentValue::Deny,
            advertising: ConsentValue::Deny,
        };

        Ok(Self {
            pod_iri,
            store: Arc::new(RwLock::new(store)),
            identity_client,
            last_fetched: RwLock::new(Some(Instant::now())),
            cache_duration: Duration::from_secs(300), // 5 minutes
            default_policy,
        })
    }

    /// Get consent policy for a specific domain
    pub fn policy_for_domain(&self, domain: &str) -> Result<DomainConsentPolicy> {
        // Check if cache needs refresh
        let should_refresh = {
            let last_fetch = self.last_fetched.read().unwrap();
            match *last_fetch {
                Some(instant) => instant.elapsed() > self.cache_duration,
                None => true,
            }
        };

        if should_refresh {
            // In production, we'd refresh from Pod here
            // For now, skip refresh
        }

        // Query oxigraph for domain-specific policy
        let query_str = format!(
            r#"
            PREFIX consent: <https://w3id.org/consent#>

            SELECT ?essential ?functional ?analytics ?advertising
            WHERE {{
                ?policy a consent:DomainConsent ;
                        consent:domain "{domain}" ;
                        consent:essential ?essential ;
                        consent:functional ?functional ;
                        consent:analytics ?analytics ;
                        consent:advertising ?advertising .
            }}
            "#
        );

        let store = self.store.read().unwrap();
        let query = Query::parse(&query_str, None)
            .context("Failed to parse SPARQL query")?;

        match store.query(query).context("SPARQL query failed")? {
            QueryResults::Solutions(mut solutions) => {
                if let Some(solution) = solutions.next() {
                    let solution = solution.context("Failed to get solution")?;

                    return Ok(DomainConsentPolicy {
                        domain: domain.to_string(),
                        essential: Self::parse_consent_value(&solution, "essential"),
                        functional: Self::parse_consent_value(&solution, "functional"),
                        analytics: Self::parse_consent_value(&solution, "analytics"),
                        advertising: Self::parse_consent_value(&solution, "advertising"),
                    });
                }
            }
            _ => {}
        }

        // No domain-specific policy found, return default
        let mut policy = self.default_policy.clone();
        policy.domain = domain.to_string();
        Ok(policy)
    }

    /// Parse consent value from SPARQL solution
    fn parse_consent_value(
        solution: &oxigraph::sparql::QuerySolution,
        variable: &str,
    ) -> ConsentValue {
        solution
            .get(variable)
            .and_then(|term| {
                if let Term::Literal(lit) = term {
                    Some(ConsentValue::from_str(lit.value()))
                } else {
                    None
                }
            })
            .unwrap_or(ConsentValue::Deny)
    }

    /// Get default policy
    pub fn default_policy(&self) -> &DomainConsentPolicy {
        &self.default_policy
    }

    /// Update consent policy for a domain
    pub async fn update_domain_policy(&self, policy: DomainConsentPolicy) -> Result<()> {
        // In production, this would:
        // 1. Update local oxigraph store
        // 2. Serialize to Turtle
        // 3. Write back to Pod via bridge

        // For now, just update local store
        // TODO: Implement full update cycle

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_consent_value_parsing() {
        assert_eq!(ConsentValue::from_str("Allow"), ConsentValue::Allow);
        assert_eq!(ConsentValue::from_str("deny"), ConsentValue::Deny);
        assert_eq!(ConsentValue::from_str("AskUser"), ConsentValue::AskUser);
        assert_eq!(ConsentValue::from_str("unknown"), ConsentValue::Deny);
    }

    #[test]
    fn test_domain_policy_blocked() {
        let policy = DomainConsentPolicy {
            domain: "evil.com".to_string(),
            essential: ConsentValue::Deny,
            functional: ConsentValue::Deny,
            analytics: ConsentValue::Deny,
            advertising: ConsentValue::Deny,
        };

        assert!(policy.is_blocked());
    }

    #[test]
    fn test_domain_policy_requires_approval() {
        let policy = DomainConsentPolicy {
            domain: "example.com".to_string(),
            essential: ConsentValue::Allow,
            functional: ConsentValue::Allow,
            analytics: ConsentValue::AskUser,
            advertising: ConsentValue::Deny,
        };

        assert!(policy.requires_user_approval());
        assert_eq!(policy.unconfigured_categories(), vec!["analytics"]);
    }
}
