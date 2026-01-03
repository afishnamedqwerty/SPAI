///! Consent Enforcement Guardrail
///!
///! Enforces user consent policies before allowing agent navigation to domains.

use crate::guardrails::{GuardrailContext, GuardrailResult, InputGuardrail};
use crate::hitl::{ActionType, ApprovalContext, ApprovalRequest, Priority};
use crate::Result;
use crate::solid::consent::{ConsentManifest, ConsentValue};
use anyhow::Context;
use async_trait::async_trait;
use regex::Regex;
use std::sync::Arc;
use tokio::sync::RwLock;
use url::Url;

/// Consent enforcement guardrail
///
/// Checks user consent manifest before allowing agent to navigate to domains.
/// Blocks navigation if domain is denied, triggers HITL if approval needed.
pub struct ConsentEnforcementGuardrail {
    /// Consent manifest
    manifest: Arc<RwLock<ConsentManifest>>,
    /// Guardrail ID
    id: String,
}

impl ConsentEnforcementGuardrail {
    /// Create new consent enforcement guardrail
    pub fn new(manifest: Arc<RwLock<ConsentManifest>>) -> Self {
        Self {
            manifest,
            id: "consent-enforcement".to_string(),
        }
    }

    /// Extract URLs from input text
    fn extract_urls(text: &str) -> Vec<Url> {
        let url_pattern = Regex::new(
            r"https?://[^\s<>\"'\)]+|www\.[^\s<>\"'\)]+"
        ).unwrap();

        let mut urls = Vec::new();

        for capture in url_pattern.find_iter(text) {
            let url_str = capture.as_str();

            // Add https:// prefix if missing
            let url_str = if url_str.starts_with("www.") {
                format!("https://{}", url_str)
            } else {
                url_str.to_string()
            };

            if let Ok(url) = Url::parse(&url_str) {
                urls.push(url);
            }
        }

        urls
    }

    /// Extract domain from URL
    fn extract_domain(url: &Url) -> Option<String> {
        url.domain().map(|d| d.to_string())
    }
}

#[async_trait]
impl InputGuardrail for ConsentEnforcementGuardrail {
    fn id(&self) -> &str {
        &self.id
    }

    async fn check(
        &self,
        input: &str,
        ctx: &GuardrailContext,
    ) -> Result<GuardrailResult> {
        // Extract URLs from input that agent might navigate to
        let urls = Self::extract_urls(input);

        if urls.is_empty() {
            // No URLs in input, pass through
            return Ok(GuardrailResult::pass("No URLs detected in input"));
        }

        let manifest = self.manifest.read().await;

        // Check each URL against consent policy
        for url in urls {
            let domain = match Self::extract_domain(&url) {
                Some(d) => d,
                None => continue, // Skip invalid domains
            };

            let policy = manifest.policy_for_domain(&domain)
                .context("Failed to get consent policy")?;

            // Check if domain is completely blocked
            if policy.is_blocked() {
                return Ok(GuardrailResult {
                    passed: false,
                    tripwire_triggered: true,
                    reasoning: format!(
                        "Domain '{}' is blocked by user consent policy. \
                         All cookie categories (essential, functional, analytics, advertising) are denied. \
                         Agent is not permitted to access this domain.",
                        domain
                    ),
                    suggested_modification: Some(format!(
                        "Remove references to '{}' or ask user to update consent manifest at their Pod.",
                        domain
                    )),
                    confidence: 1.0,
                });
            }

            // Check if domain requires user approval
            if policy.requires_user_approval() {
                let unconfigured = policy.unconfigured_categories();

                // Create HITL approval request
                let approval_request = ApprovalRequest {
                    id: crate::types::ApprovalId::new(),
                    agent_id: ctx.agent_id.clone(),
                    action_type: ActionType::ConsentRequired,
                    description: format!(
                        "Agent needs consent to access '{}'. \
                         Current policy requires user decision for: {}",
                        domain,
                        unconfigured.join(", ")
                    ),
                    context: ApprovalContext::Custom(serde_json::json!({
                        "type": "consent_request",
                        "domain": domain,
                        "unconfigured_categories": unconfigured,
                        "url": url.as_str(),
                    })),
                    priority: Priority::Normal,
                    deadline: None,
                    suggested_approvers: vec![],
                };

                // In a real implementation, we'd trigger HITL approval here
                // For now, we'll return a guardrail result indicating approval needed
                return Ok(GuardrailResult {
                    passed: false,
                    tripwire_triggered: false, // Don't tripwire, just request approval
                    reasoning: format!(
                        "Domain '{}' requires user consent approval for: {}. \
                         Cannot proceed without explicit user permission.",
                        domain,
                        unconfigured.join(", ")
                    ),
                    suggested_modification: Some(format!(
                        "Request user approval for accessing '{}' or update consent manifest.",
                        domain
                    )),
                    confidence: 1.0,
                });
            }

            // If we get here, domain is allowed according to consent policy
            // Check specific cookie categories
            if policy.analytics == ConsentValue::Deny || policy.advertising == ConsentValue::Deny {
                // Domain is allowed but with restrictions
                // Log this for informational purposes
                tracing::info!(
                    domain = %domain,
                    analytics = ?policy.analytics,
                    advertising = ?policy.advertising,
                    "Domain access allowed with cookie restrictions"
                );
            }
        }

        // All URLs pass consent checks
        Ok(GuardrailResult::pass(format!(
            "All {} URL(s) comply with user consent policy",
            urls.len()
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_url_extraction() {
        let text = "Visit https://example.com and www.test.org for more info";
        let urls = ConsentEnforcementGuardrail::extract_urls(text);

        assert_eq!(urls.len(), 2);
        assert_eq!(urls[0].as_str(), "https://example.com");
        assert_eq!(urls[1].as_str(), "https://www.test.org");
    }

    #[test]
    fn test_domain_extraction() {
        let url = Url::parse("https://subdomain.example.com/path?query=1").unwrap();
        let domain = ConsentEnforcementGuardrail::extract_domain(&url);

        assert_eq!(domain, Some("subdomain.example.com".to_string()));
    }

    #[test]
    fn test_url_extraction_complex() {
        let text = r#"
            Check out https://github.com/user/repo and
            http://localhost:3000 or www.wikipedia.org
        "#;

        let urls = ConsentEnforcementGuardrail::extract_urls(text);
        assert!(urls.len() >= 2); // At least github and wikipedia
    }
}
