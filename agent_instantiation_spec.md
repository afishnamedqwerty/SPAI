# Agent Instantiation Specification: spvault Integrated Architecture

**Version 2.0.0**

## 1. Overview

This specification details the instantiation process for SPAI agents, rigorously integrated with the **spvault** permissionless credential posturing toolkit and **Solid Pod** identity architecture. It bridges the gap between agent construction (intelligence) and defense-in-depth security (identity, consent, execution protection).

## 2. Security Architecture

### 2.1 The Security Context
Every agent instantiation creates a `SecurityContext` that binds the valid execution environment (TEE) to a verifiable identity (WebID).

```rust
pub struct SecurityContext {
    pub tee: Arc<dyn TeeBackend>,       // Enclave/TPM/SGX
    pub identity: AgentIdentity,        // WebID + DPoP Keys
    pub broker: Arc<VaultBroker>,       // Policy Enforcement Point
}

pub struct AgentIdentity {
    pub webid: WebId,
    pub session: Session,               // DPoP-bound authenticated session
    pub trusted_issuers: Vec<String>,   // Allowlist for Solid-OIDC
}
```

### 2.2 Permissions & Posturing
Agents "posture" by presenting:
1.  **DPoP Proofs**: Cryptographic proof of key possession (keys never leave TEE).
2.  **Attestation**: Hardware evidence of code integrity.
3.  **Consent Records**: Linked Data evidence of user authorization.

## 3. Agent Constructor Specification

### 3.1 `AgentBuilder` Interface

The builder enforces a **secure-by-construction** invariant: no agent can be built without a fully initialized security layer.

```rust
pub struct AgentBuilder {
    // Identity & Security (Required)
    fn with_security_context(mut self, ctx: SecurityContext) -> Self;
    
    // Intelligence (OpenRouter)
    fn with_model(mut self, config: OpenRouterConfig) -> Self;
    
    // Capabilities
    fn with_tools(mut self, tools: Vec<Arc<dyn Tool>>) -> Self;
    
    // Governance
    fn with_consent_manifest(mut self, manifest: ConsentManifest) -> Self;
}
```

## 4. Detailed Integration Protocol

### 4.1 Step 1: TEE Initialization (Root of Trust)
*   **Action**: Detect and initialize the availble TEE (Secure Enclave, TPM, SGX, or Software Fallback).
*   **Output**: A `TeeBackend` handle for secure key generation.
*   **Key Gen**: Generate an ephemeral EC P-256 key pair *inside* the TEE for DPoP.
    *   *Constraint*: The private key **MUST NEVER** leave the TEE.

### 4.2 Step 2: Solid-OIDC Authentication (Identity)
*   **Objective**: Bind the agent to a Solid WebID.
*   **Flow**: Hybrid Federation (Direct or Broker Proxy).
    1.  **Discovery**: Fetch Client ID Document and OpenID Configuration.
    2.  **Challenge**: Generate DPoP-bound auth request signed by TEE key.
    3.  **Token Exchange**: Exchange authorization code for DPoP-bound ID Token.
*   **Validation** (Critical):
    *   **JWKS**: Verify signature against IdP keys.
    *   **Trusted Issuers**: Reject `iss` if not in `TRUSTED_ISSUER_LIST`.
    *   **Audience**: Verify `aud` contains `"solid"`.
    *   **WebID Binding**: Dereference WebID Profile and verify `solid:oidcIssuer` matches `iss`.

### 4.3 Step 3: Consent Loading (Authorization)
*   **Objective**: Load ODRL and ACP policies to govern agent behavior.
*   **Sources**:
    *   **ODRL**: `GET {credential_ref}/policy`
    *   **ACP**: `GET {credential_ref}.acr`
*   **Enforcement**: The `PolicyEngine` is initialized with these policies.

### 4.4 Step 4: Constructing the Vault Broker
The Broker is the central orchestrator that holds the TEE handle and Policy Engine.

```rust
let broker = VaultBroker::new(tee, policy_engine, identity_session).await?;
```

## 5. Tool Instantiation & Protection

### 5.1 The `PolicyGuard`
All tools that access credentials or network resources must be wrapped in a `PolicyGuard`.

```rust
struct PolicyGuard<T: Tool> {
    inner: T,
    broker: Arc<VaultBroker>,
    purpose: DpvPurpose, // e.g., dpv:ServiceProvision
    target_origin: Option<Origin>,
}

impl<T: Tool> Tool for PolicyGuard<T> {
    async fn execute(&self, params: Value, ctx: &ToolContext) -> Result<ToolOutput> {
         // ODRL Evaluation
         let decision = self.broker.evaluate_policy(
             &self.inner.credential_ire(),
             &ctx.agent_webid,
             &self.target_origin,
             &self.purpose
         ).await?;

         match decision {
             PolicyDecision::Permit(duties) => {
                 // 1. Mint semantic IRI handle
                 let handle = self.broker.mint_handle(...).await?;
                 // 2. Execute tool with handle
                 let result = self.inner.execute_with_handle(params, handle).await?;
                 // 3. Log Consent Record (Audit)
                 self.broker.log_consent_record(decision).await?;
                 Ok(result)
             },
             PolicyDecision::Deny(reason) => Err(Error::PermissionDenied(reason)),
         }
    }
}
```

### 5.2 Tool Schema Enhancements
Tools must benefit from DPV (Data Privacy Vocabulary) types.

*   `credential_ref`: Must be a full **IRI** (e.g., `https://alice.pod/credentials/github`).
*   `purpose`: Must be a valid **DPV IRI** (e.g., `http://www.w3.org/ns/dpv#ServiceProvision`).

## 6. IRI Handles & Data Sovereignty

### 6.1 Semantic Handle Structure
Handles are no longer opaque UUIDs. They are dereferenceable IRIs pointing to metadata in the Pod.

**Format**: `https://{pod_host}/credentials/{cred_alias}/handles/{uuid}#active`

### 6.2 Handle Resolution
*   **Agent View (JSON)**: `{"value": "123456", "ttl": 60}`
*   **Auditor View (RDF)**:
    ```turtle
    <handle_iri> a cred:CredentialHandle ;
         cred:boundToOrigin "https://github.com" ;
         cred:authorizedFields ("username", "password") ;
         solid:owner <https://alice.pod/profile#me> .
    ```

## 7. Implementation Roadmap

1.  **Phase 1**: Core TEE & OpenRouter (Standard).
2.  **Phase 2**: Solid-OIDC Client & DPoP Key Gen (Identity).
3.  **Phase 3**: ODRL/ACP Policy Engine (Consent).
4.  **Phase 4**: IRI Handle Minting & Resolution (Sovereignty).

## 8. Verification Plan

1.  **Integration Test**: `test_agent_instantiation_with_solid`
    *   Mock complete Solid-OIDC flow.
    *   Verify DPoP proof generation signature.
    *   Assert `PolicyGuard` blocks unauthorized origin.
2.  **Consent Compliance**:
    *   Attempt tool use with mismatched Purpose.
    *   Verify `ConsentRecord` creation in mock Pod.
3.  **IRI Resolution**:
    *   Dereference a minted handle and validate RDF graph structure.
