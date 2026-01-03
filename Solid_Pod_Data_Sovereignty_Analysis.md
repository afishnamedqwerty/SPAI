Data Sovereignty Architecture

Solid Pods, WebID Authentication & IRI Handshake Protocols

**For Autonomous Agent Browser Authentication**

*A Critical Analysis for SPAI Agent Harness Integration*

Version 1.0.0 --- December 2025

1\. Executive Summary

This analysis examines the integration of Solid Pod technology with the SPAI agent harness to establish true data sovereignty for autonomous agents. The core challenge is enabling AI agents operating headless browsers to authenticate with web servers while maintaining user control over data and consent decisions.

We propose a three-layer authentication architecture:

1.  **Identity Layer:** WebID-based agent identification with dereferenceable Client ID Documents stored on Solid Pods

2.  **Authentication Layer:** Solid-OIDC with DPoP (Demonstrating Proof of Possession) to cryptographically bind tokens to agent key pairs

3.  **Consent Layer:** IRI-based consent manifests stored on user Pods, defining cookie/beacon injection policies per-domain

**Key Finding:** The Solid ecosystem provides the most mature standards-based approach to agent data sovereignty, but significant engineering challenges remain in bridging the gap between Solid\'s RDF-centric protocols and conventional web authentication mechanisms (cookies, sessions, OAuth flows).

2\. Problem Statement & Threat Model

2.1 The Agent Authentication Trilemma

When an AI agent operates a headless browser on behalf of a user, three competing concerns create a fundamental trilemma:

  ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------
  **Concern**             **Description**
  ----------------------- ---------------------------------------------------------------------------------------------------------------------------------------------------
  **User Sovereignty**    The user must retain control over what data the agent can access, what actions it can take, and what third parties can track its behavior

  **Server Trust**        Web servers need assurance that the agent is authorized to act on behalf of the claimed user, and that tokens haven\'t been stolen/replayed

  **Agent Capability**    The agent must be able to effectively complete tasks, which often requires accepting cookies, executing JavaScript, and maintaining session state
  ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------

2.2 Threat Model

The authentication system must defend against:

-   **Token Theft:** Malicious servers capturing and replaying authentication tokens

-   **Agent Impersonation:** Rogue agents claiming to represent legitimate users

-   **Consent Bypass:** Agents accepting tracking cookies/beacons without user authorization

-   **Scope Creep:** Agents accessing resources beyond their authorized scope

-   **Data Exfiltration:** Captured session data being sent to unauthorized parties

3\. Solid Pod as Data Sovereignty Foundation

3.1 What is a Solid Pod?

A Solid Pod (Personal Online Data Store) is a decentralized data storage container where users maintain complete ownership and control over their data. Unlike centralized services where data is stored on company servers, Solid Pods give users the ability to choose where their data lives and who can access it.

Key characteristics relevant to agent authentication:

-   **Decoupled Identity:** WebID (identity) can be hosted separately from Pod (storage)

-   **Linked Data Native:** All data stored as RDF, enabling semantic querying and IRI-based references

-   **Fine-grained ACL:** Web Access Control (WAC) allows per-resource, per-agent permissions

-   **Standard Protocols:** Built on LDP (Linked Data Platform), WebID-OIDC, and HTTP

3.2 Agent Identity via Solid Client IDs

In Solid-OIDC, applications (including AI agents) identify themselves using a Client ID that is a dereferenceable IRI pointing to a Client ID Document. This document is a JSON-LD resource containing metadata about the client.

3.2.1 Client ID Document Structure

{ \"@context\": \"https://www.w3.org/ns/solid/oidc-context.jsonld\", // The Client ID is the IRI of this document \"client_id\": \"https://agent.example.com/spai-agent.jsonld\", // Where the agent can receive OAuth callbacks \"redirect_uris\": \[ \"https://agent.example.com/callback\", \"http://localhost:3000/callback\" // For development \], // Human-readable agent metadata \"client_name\": \"SPAI Research Agent v1.0\", \"client_uri\": \"https://agent.example.com/\", \"logo_uri\": \"https://agent.example.com/logo.png\", \"tos_uri\": \"https://agent.example.com/terms\", \"policy_uri\": \"https://agent.example.com/privacy\", // Contact for the agent operator \"contacts\": \[\"admin@agent.example.com\"\], // OAuth scopes the agent may request \"scope\": \"openid offline_access webid\", // Supported grant types \"grant_types\": \[\"authorization_code\", \"refresh_token\"\], // Response types for OIDC flows \"response_types\": \[\"code\"\] }

**Critical Insight:** By hosting the Client ID Document on a domain the agent operator controls, the agent\'s identity is cryptographically tied to DNS ownership. This provides a trust anchor without requiring pre-registration with every Identity Provider.

4\. WebID-OIDC and IRI Handshake Protocol

4.1 Understanding IRIs in the Solid Context

An IRI (Internationalized Resource Identifier) is a Unicode extension of URIs that serves as the fundamental identifier in RDF and Linked Data systems. In Solid, IRIs serve multiple critical functions:

  -------------------------------------------------------------------------------------------------------------
  **IRI Type**        **Example**                                 **Purpose**
  ------------------- ------------------------------------------- ---------------------------------------------
  **WebID**           https://alice.pod.example/profile/card#me   Identifies a person, agent, or organization

  **Client ID**       https://agent.example/client.jsonld         Identifies a client application

  **Resource IRI**    https://alice.pod.example/data/file.ttl     Identifies a specific data resource

  **Issuer IRI**      https://idp.example.com/                    Identifies an OIDC Identity Provider
  -------------------------------------------------------------------------------------------------------------

4.2 The IRI Handshake Flow

When an agent needs to authenticate to a web server, an \"IRI handshake\" establishes mutual trust through a series of dereferenceable lookups:

┌─────────────────────────────────────────────────────────────────────────────────┐ │ IRI HANDSHAKE AUTHENTICATION FLOW │ ├─────────────────────────────────────────────────────────────────────────────────┤ │ │ │ ┌─────────┐ 1. Present WebID IRI ┌─────────────┐ │ │ │ Agent │ ─────────────────────────► │ Resource │ │ │ │ │ │ Server (RS) │ │ │ └────┬────┘ └──────┬──────┘ │ │ │ │ │ │ │ │ 2. Dereference WebID │ │ │ ▼ │ │ │ ┌─────────────┐ │ │ │ │ WebID │ │ │ │ │ Profile │ │ │ │ │ Document │ │ │ │ └──────┬──────┘ │ │ │ │ │ │ │ │ 3. Extract solid:oidcIssuer │ │ │ ▼ │ │ │ 4. Redirect to OP ┌─────────────┐ │ │ │ ◄─────────────────────────────── │ Identity │ │ │ │ │ Provider │ │ │ │ 5. Authenticate + Consent │ (OP) │ │ │ │ ─────────────────────────────► │ │ │ │ │ └──────┬──────┘ │ │ │ │ │ │ │ 6. ID Token + DPoP Proof │ │ │ │ ◄───────────────────────────────────────┘ │ │ │ │ │ │ 7. Access RS with DPoP-bound Token ┌─────────────┐ │ │ │ ─────────────────────────────────────► │ Resource │ │ │ │ │ Server │ │ │ │ 8. Verify: Token + DPoP + WebID │ │ │ │ │ ◄───────────────────────────────────── │ │ │ │ │ └─────────────┘ │ │ │ └─────────────────────────────────────────────────────────────────────────────────┘

5\. DPoP: Proof of Possession for Agent Tokens

5.1 Why Bearer Tokens Fail for Agents

Traditional OAuth bearer tokens are inherently vulnerable when used by agents accessing multiple resource servers. The fundamental problem: any server that receives a bearer token can replay it against other servers.

**Attack Scenario (Token Theft):**

1.  Agent authenticates with legitimate IdP, receives bearer token

2.  Agent sends request to evil-server.example with bearer token

3.  Evil server captures token, replays it against bank.example

4.  Bank.example accepts token, evil server accesses user\'s financial data

5.2 DPoP Solution Architecture

DPoP (RFC 9449) binds access tokens to a client\'s cryptographic key pair. The agent must prove possession of the private key with every request, making stolen tokens useless.

// Agent generates asymmetric key pair at initialization pub struct AgentKeyPair { /// ECDSA P-256 or RSA-2048 private key (NEVER leaves agent) private_key: PrivateKey, /// Public key embedded in DPoP proofs public_key: PublicKey, /// Key ID for tracking/rotation kid: String, } // DPoP Proof JWT Structure { // Header \"typ\": \"dpop+jwt\", \"alg\": \"ES256\", \"jwk\": { \"kty\": \"EC\", \"crv\": \"P-256\", \"x\": \"base64url-encoded-x-coordinate\", \"y\": \"base64url-encoded-y-coordinate\" } } { // Payload \"jti\": \"unique-proof-id-abc123\", // Prevents replay \"htm\": \"POST\", // HTTP method being used \"htu\": \"https://rs.example/resource\", // Target URL \"iat\": 1701456789, // Issued at \"ath\": \"fUHyO2r2Z3DZ53EsNrWBb0xWX\...\" // Access token hash (for RS requests) } // Signature created with agent\'s private key

5.3 DPoP in the Agent Authentication Flow

1.  **Token Request:** Agent sends DPoP proof (with htm=POST, htu=token_endpoint) to IdP

2.  **Token Binding:** IdP embeds public key thumbprint in access token\'s \'cnf\' claim

3.  **Resource Access:** Agent sends NEW DPoP proof + access token to Resource Server

4.  **Verification:** RS verifies DPoP signature AND that public key matches token\'s \'cnf\' claim

**Security Guarantee:** Even if a malicious server captures the access token, it cannot use it because it doesn\'t possess the agent\'s private key needed to generate valid DPoP proofs.

6\. Cookie & Beacon Consent Architecture

6.1 The Consent Challenge for Headless Agents

When agents operate headless browsers, they encounter cookie consent banners that cannot be simply \"clicked through\" without explicit user authorization. GDPR and similar regulations require informed consent for non-essential cookies, but the consent decision belongs to the user, not the agent.

**Categories of Cookies/Beacons:**

  ----------------------------------------------------------------------------------------------------------
  **Category**      **Purpose**                                   **Agent Handling**
  ----------------- --------------------------------------------- ------------------------------------------
  **Essential**     Login sessions, security tokens, cart state   Auto-accept (required for functionality)

  **Functional**    Preferences, language, accessibility          Accept per user consent manifest

  **Analytics**     Usage tracking, performance metrics           Requires explicit user consent

  **Advertising**   Cross-site tracking, ad personalization       Default DENY unless explicit consent
  ----------------------------------------------------------------------------------------------------------

6.2 Consent Manifest Architecture

We propose storing user consent preferences as an RDF document on their Solid Pod. This \"Consent Manifest\" is an IRI-addressable resource that agents must dereference before accepting any non-essential cookies.

\# Consent Manifest stored at: https://alice.pod.example/consents/browser-agent.ttl \@prefix consent: \<https://w3id.org/consent#\> . \@prefix xsd: \<http://www.w3.org/2001/XMLSchema#\> . \@prefix agent: \<https://agent.example.com/spai-agent.jsonld#\> . \# Default policy for all domains \<#default-policy\> a consent:ConsentPolicy ; consent:appliesTo agent:spai-agent ; consent:defaultEssential consent:Allow ; consent:defaultFunctional consent:Allow ; consent:defaultAnalytics consent:Deny ; consent:defaultAdvertising consent:Deny ; consent:lastUpdated \"2025-12-08T00:00:00Z\"\^\^xsd:dateTime . \# Domain-specific override: Allow analytics for trusted site \<#google-analytics-consent\> a consent:DomainConsent ; consent:domain \"docs.google.com\" ; consent:analytics consent:Allow ; consent:advertising consent:Deny ; consent:validUntil \"2026-12-08T00:00:00Z\"\^\^xsd:dateTime ; consent:grantedBy \<https://alice.pod.example/profile/card#me\> . \# Explicit block for known tracker domains \<#tracker-block\> a consent:DomainConsent ; consent:domain \"doubleclick.net\" ; consent:essential consent:Deny ; consent:functional consent:Deny ; consent:analytics consent:Deny ; consent:advertising consent:Deny .

7\. Implementation Architecture

7.1 Agent-Side Components

pub struct SolidAuthenticatedAgent { /// Agent\'s WebID (dereferenceable IRI) agent_webid: Iri, /// Agent\'s Client ID Document IRI client_id: Iri, /// DPoP key pair for token binding dpop_keys: AgentKeyPair, /// Cached OIDC tokens per Identity Provider token_cache: HashMap\<Iri, DPoPBoundToken\>, /// User\'s Pod IRI (for consent manifest lookup) user_pod: Iri, /// Cached consent manifest consent_manifest: ConsentManifest, /// Headless browser instance browser: HeadlessBrowser, } impl SolidAuthenticatedAgent { /// Authenticate to a resource server using Solid-OIDC + DPoP pub async fn authenticate(&mut self, resource_server: &Iri) -\> Result\<()\> { // 1. Check if we have a valid token for this RS\'s required IdP if let Some(token) = self.get_cached_token(resource_server).await? { return Ok(()); } // 2. Discover the RS\'s OIDC requirements let oidc_config = self.discover_oidc_config(resource_server).await?; // 3. Generate fresh DPoP proof for token request let dpop_proof = self.generate_dpop_proof( &oidc_config.token_endpoint, \"POST\" )?; // 4. Execute OIDC Authorization Code flow with PKCE + DPoP let auth_code = self.perform_authorization_flow(&oidc_config).await?; // 5. Exchange code for DPoP-bound tokens let tokens = self.exchange_code_for_tokens( &oidc_config, auth_code, dpop_proof ).await?; // 6. Cache tokens self.token_cache.insert(oidc_config.issuer.clone(), tokens); Ok(()) } /// Navigate headless browser with consent-aware cookie handling pub async fn navigate_with_consent(&mut self, url: &Url) -\> Result\<Page\> { // 1. Load consent manifest from user\'s Pod self.refresh_consent_manifest().await?; // 2. Configure browser cookie policy based on manifest let cookie_policy = self.consent_manifest.policy_for_domain(url.domain())?; self.browser.set_cookie_policy(cookie_policy); // 3. Navigate to URL let page = self.browser.navigate(url).await?; // 4. If consent banner detected, handle according to policy if let Some(banner) = page.detect_consent_banner().await? { self.handle_consent_banner(&banner, &cookie_policy).await?; } Ok(page) } }

8\. Critical Analysis & Challenges

8.1 Strengths of This Approach

-   **Standards-Based:** Built entirely on W3C recommendations (Solid, WebID, LDP) and IETF standards (OAuth 2.0, DPoP, OIDC)

-   **User Sovereignty:** All consent decisions stored on user-controlled Pod, not centralized servers

-   **Cryptographic Binding:** DPoP eliminates bearer token vulnerabilities

-   **Auditability:** All consent grants are timestamped RDF triples, enabling compliance audits

-   **Interoperability:** Agent identity portable across any Solid-compatible service

8.2 Significant Challenges

  ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------
  **Challenge**                  **Analysis**
  ------------------------------ ---------------------------------------------------------------------------------------------------------------------------------------------
  **Server Adoption**            Most web servers don\'t support Solid-OIDC. Agents will need fallback to conventional OAuth/session-based auth for non-Solid resources.

  **Consent Banner Diversity**   Cookie consent implementations vary wildly. No standard API exists. Agents must use heuristics or ML to identify and interact with banners.

  **Key Management**             DPoP private keys must be securely stored. HSM integration adds complexity. Key rotation requires re-authentication.

  **Latency**                    Multiple IRI dereferences (WebID → Profile → Issuer → Token) add latency. Aggressive caching needed for acceptable UX.

  **Consent UI Gap**             Users need a way to manage consent manifests. Requires building a Solid-compatible consent management UI, which doesn\'t exist yet.
  ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------

8.3 Recommended Hybrid Approach

Given the current ecosystem maturity, we recommend a tiered authentication strategy:

1.  **Tier 1 (Solid-Native):** Full WebID-OIDC + DPoP for Solid Pods and Solid-compatible services

2.  **Tier 2 (OAuth 2.0 + DPoP):** For services supporting DPoP but not Solid (e.g., modern OAuth implementations)

3.  **Tier 3 (Session-Based):** Fallback for traditional session/cookie authentication with consent manifest enforcement

4.  **Tier 4 (Unauthenticated):** Public resources with strict cookie blocking per consent manifest

9\. Integration with SPAI Architecture

9.1 Mapping to SPAI Components

  ----------------------------------------------------------------------------------------------
  **SPAI**        **Solid Integration**                  **Purpose**
  ------------------- -------------------------------------- -----------------------------------
  **Agents**          WebID + Client ID Document             Agent identity & metadata

  **Tools**           Solid Pod read/write, SPARQL queries   Access user data with ACL respect

  **Guardrails**      Consent Manifest enforcement           Cookie/beacon policy validation

  **Tracing**         RDF-serialized trace logs on Pod       Auditable agent action history

  **Human-in-Loop**   Consent management UI on Pod           User approval for new consents
  ----------------------------------------------------------------------------------------------

9.2 New Guardrail: ConsentEnforcementGuardrail

pub struct ConsentEnforcementGuardrail { /// User\'s Pod IRI for consent manifest lookup user_pod: Iri, /// Cached consent manifest manifest: RwLock\<ConsentManifest\>, /// Cache TTL cache_duration: Duration, } #\[async_trait\] impl InputGuardrail for ConsentEnforcementGuardrail { fn id(&self) -\> &str { \"consent-enforcement\" } async fn check( &self, input: &str, ctx: &GuardrailContext, ) -\> Result\<GuardrailResult\> { // Extract URLs from input that agent might navigate to let urls = extract_urls(input); for url in urls { let domain = url.domain().ok_or(Error::InvalidUrl)?; let policy = self.manifest.read().policy_for_domain(domain); // If domain is explicitly blocked, trigger tripwire if policy.is_blocked() { return Ok(GuardrailResult { passed: false, tripwire_triggered: true, reasoning: format!(\"Domain {} is blocked by user consent policy\", domain), suggested_modification: None, confidence: 1.0, }); } // If domain requires consent we don\'t have, request HITL if policy.requires_unconfigured_consent() { ctx.request_human_approval(ApprovalRequest { action_type: ActionType::ConsentRequired, description: format!(\"Agent needs consent to access {}\", domain), context: ApprovalContext::ConsentRequest { domain, categories: policy.unconfigured_categories() }, priority: Priority::Normal, deadline: None, }).await?; } } Ok(GuardrailResult::passed()) } }

10\. Recommendations & Next Steps

10.1 Immediate Actions (Phase 1)

1.  Implement DPoP key generation and proof creation in Rust using the \'jose\' crate

2.  Create Client ID Document hosting infrastructure (static JSON-LD server)

3.  Build consent manifest parser and domain policy resolver

4.  Integrate with existing Solid Pod providers (solidcommunity.net, inrupt.net) for testing

10.2 Medium-Term Goals (Phase 2)

-   Develop consent management UI as a Solid app

-   Implement ML-based cookie banner detection and classification

-   Create tiered authentication fallback system

-   Build trace logging to user Pod in RDF format

10.3 Long-Term Vision (Phase 3)

-   Propose consent manifest ontology to W3C Solid CG

-   Advocate for Solid-OIDC support in major web frameworks

-   Integrate with emerging browser-native consent APIs (if standardized)

-   Explore integration with Verifiable Credentials for agent attestation

11\. Conclusion

The Solid ecosystem provides the most principled foundation for agent data sovereignty, combining dereferenceable identity (WebID), cryptographic token binding (DPoP), fine-grained access control (WAC), and user-owned storage (Pods). However, the current web ecosystem\'s limited Solid adoption means a hybrid approach is necessary.

The proposed architecture enables agents to authenticate to web servers while respecting user sovereignty over consent decisions. By storing consent manifests as RDF on user Pods, we ensure that cookie/beacon acceptance policies are portable, auditable, and under user control.

**Key Takeaway:** The IRI handshake --- where agent, user, and server identities are all dereferenceable IRIs with cryptographic binding via DPoP --- represents the gold standard for agent authentication. While full ecosystem adoption remains years away, implementing this architecture now positions the SPAI harness at the forefront of privacy-respecting autonomous agents.

References

-   Solid Protocol Specification --- [[solidproject.org/TR/protocol]{.underline}](https://solidproject.org/TR/protocol)

-   Solid-OIDC Specification --- [[solidproject.org/TR/oidc]{.underline}](https://solidproject.org/TR/oidc)

-   RFC 9449: OAuth 2.0 DPoP --- [[datatracker.ietf.org/doc/html/rfc9449]{.underline}](https://datatracker.ietf.org/doc/html/rfc9449)

-   RFC 3987: Internationalized Resource Identifiers --- [[ietf.org/rfc/rfc3987.txt]{.underline}](https://www.ietf.org/rfc/rfc3987.txt)

-   WebID Specification --- [[w3.org/2005/Incubator/webid/spec/identity]{.underline}](https://www.w3.org/2005/Incubator/webid/spec/identity/)

-   Inrupt Identity Documentation --- [[docs.inrupt.com/guides/identity-in-solid]{.underline}](https://docs.inrupt.com/guides/identity-in-solid)

-   CookieBlock: Automating GDPR Consent (USENIX Security 2022)
