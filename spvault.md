# SPVault

**A Permissionless Credential Posturing Toolkit**

SPVault provides a defense-in-depth credential management system with hardware TEE protection, WebID identity verification, ODRL consent policies, and browser automation for secure credential injection.

## Features

- 🔐 **Hardware TEE Integration** - Secure Enclave, TPM 2.0, SGX with software fallback
- 🌐 **Solid-OIDC Authentication** - WebID profiles with dual-binding (human + agent)
- 📜 **ODRL Consent Policies** - Cryptographic proof of user consent
- 🖥️ **Browser Automation** - CDP-based credential injection with Shadow DOM isolation
- 🍪 **Cookie Enforcement** - IAB TCF v2.0 category blocking
- ⚡ **Performance Benchmarking** - Criterion benchmarks for all TEE operations

## Architecture

```
┌─────────────────────────────────────────────────────────────────────────┐
│                           VaultBroker                                    │
│   Orchestrates the seven defense layers for credential operations       │
└────────────────┬─────────────────────────────────────────────┬──────────┘
                 │                                             │
    ┌────────────┴────────────┐                   ┌────────────┴──────────┐
    │      spvault-tee        │                   │    spvault-bridge     │
    │  ┌──────────────────┐   │                   │  ┌────────────────┐   │
    │  │ SoftwareEnclave  │   │                   │  │BridgeController│   │
    │  │ (SQLCipher+      │   │                   │  │   CDP Session  │   │
    │  │  Argon2id)       │   │                   │  │  DOM Injector  │   │
    │  └──────────────────┘   │                   │  │ ConsentHandler │   │
    │  ┌──────────────────┐   │                   │  │ CookieEnforcer │   │
    │  │ Hardware Backends│   │                   │  └────────────────┘   │
    │  │ • SecureEnclave  │   │                   └───────────────────────┘
    │  │ • TPM 2.0        │   │
    │  │ • PlatformKeys   │   │
    │  └──────────────────┘   │
    └─────────────────────────┘
```

## Seven Defense Layers

| Layer | Purpose | Implementation |
|-------|---------|----------------|
| 1. Network | TLS + Certificate Pinning | External (reqwest) |
| 2. Identity | WebID + Solid-OIDC | `spvault-identity` |
| 3. Token | DPoP with TEE keys | `spvault-identity` |
| 4. Authorization | ODRL policy evaluation | `spvault-consent` |
| 5. Consent | User manifest verification | `spvault-consent` |
| 6. Isolation | TEE + Shadow DOM | `spvault-tee`, `spvault-bridge` |
| 7. Audit | Cryptographic logging | `spvault-broker` |

## Quick Start

```bash
# Build all crates
cargo build --workspace

# Run tests
cargo test --workspace

# Run benchmarks
cargo bench -p spvault-tee

# Run browser integration tests (requires Chromium)
chromium --remote-debugging-port=9222 --headless &
cargo test --test browser_integration -- --ignored
```

## Crates

### spvault-core

Core types and secure memory primitives.

```rust
use spvault_core::{SecureBuffer, CredentialRef, Handle};

// Create guarded memory buffer
let secret = SecureBuffer::new(32);
// Memory is:
// - mlock'd (non-swappable)
// - madvise'd (MADV_DONTDUMP)
// - Guard pages on both ends
// - Automatically zeroed on drop
```

### spvault-tee

Hardware TEE detection and software enclave emulation.

```rust
use spvault_tee::{detect_tee, TeeBackend, SoftwareEnclave, EnclaveConfig};

// Auto-detect best available TEE
let detection = detect_tee().await?;
println!("Using: {:?}", detection.level);  // Hardware/Software/Ephemeral

// Or configure software enclave explicitly
let config = EnclaveConfig {
    db_path: PathBuf::from("/secure/vault.db"),
    argon2_memory_cost: 256 * 1024,  // 256 MiB
    argon2_time_cost: 4,
    argon2_parallelism: 4,
    ..Default::default()
};

let mut enclave = SoftwareEnclave::with_config(config).await?;
enclave.initialize().await?;

// Seal sensitive data
let sealed = enclave.seal(b"secret", "key-id").await?;

// Generate attestation token
let attestation = enclave.generate_attestation(b"nonce").await?;
```

### spvault-identity

Solid-OIDC authentication with WebID profiles and DPoP proofs.

```rust
use spvault_identity::{WebIdProfile, DpopProofBuilder, Session, RefreshConfig};

// Fetch and validate WebID profile
let client = reqwest::Client::new();
let webid = WebId::new("https://alice.example/profile#me")?;
let profile = WebIdProfile::fetch(&webid, &client).await?;

// Check issuer allowlist
profile.validate_issuer("https://solidcommunity.net")?;

// Create DPoP-protected session with auto-refresh
let session = Session::new(webid, profile, dpop)
    .with_refresh_config(
        "https://solidcommunity.net/token".to_string(),
        "client-id".to_string(),
    )
    .with_expiry(3600);

// Session auto-refreshes when 20% of lifetime remains
if session.should_refresh() {
    session.refresh(&tee_backend).await?;
}
```

### spvault-consent

ODRL policy evaluation and rate limiting.

```rust
use spvault_consent::{PolicyEngine, ConsentManifest, Policy};

// Load user consent manifest
let manifest = ConsentManifest::load(&tee, &webid).await?;

// Evaluate access request against policies
let engine = PolicyEngine::new(policies);
let decision = engine.evaluate(&credential_ref, &origin, &purpose).await;

match decision {
    PolicyDecision::Permit { constraints } => { /* Allowed with limits */ }
    PolicyDecision::Deny { reason } => { /* Blocked */ }
}
```

### spvault-bridge

Browser automation with CDP for secure credential injection.

```rust
use spvault_bridge::{BridgeSession, BridgeController, InjectionRequest};

// Connect to Chrome DevTools Protocol
let config = BridgeConfig {
    remote_debugging_port: Some(9222),
    default_timeout: Duration::from_secs(10),
    headless: true,
    enforce_consent: true,
};
let session = BridgeSession::connect(config).await?;
let page = session.new_page().await?;

// Create controller for policy-gated injection
let controller = BridgeController::new(broker.clone(), session.clone()).await?;

// Navigate and prepare (handles consent banners, cookies)
controller.navigate_and_prepare(&page, "https://login.example.com").await?;

// Inject credentials with retry logic
let results = controller.inject_credentials(&page, vec![
    InjectionRequest {
        credential_ref: CredentialRef::new("alice-login"),
        field_type: FieldType::Username,
        selector: "#username".to_string(),
        purpose: Purpose::Authentication,
    },
]).await?;
```

### spvault-broker

Central orchestration implementing all seven defense layers.

```rust
use spvault_broker::{VaultBroker, BrokerConfig};

let broker = VaultBroker::new(
    tee_backend,
    consent_engine,
    identity_provider,
).await?;

// Request credential access (policy-checked)
let handle = broker.request_handle(
    &credential_ref,
    &origin,
    vec![FieldType::Username, FieldType::Password],
    Purpose::Authentication,
).await?;

// Handle is time-limited and use-count-limited
```

## Performance

Benchmarks on Intel i7 (software enclave with SQLCipher):

| Operation | Payload | Latency |
|-----------|---------|---------|
| Seal | 1 KiB | ~2.1 ms |
| Unseal | 1 KiB | ~0.8 ms |
| Sign (ES256) | 256 B | ~0.3 ms |
| Key Gen (ES256) | - | ~1.2 ms |
| Attestation | - | ~0.1 ms |
| Init (Argon2id 256 MiB) | - | ~850 ms |

Run benchmarks:
```bash
cargo bench -p spvault-tee
```

## Browser Tests

Integration tests require Chromium with remote debugging:

```bash
# Start headless Chrome
chromium --remote-debugging-port=9222 --headless &

# Run tests
cargo test --test browser_integration -- --ignored
```

Tests cover:
- CDP session management
- Shadow DOM injection
- Consent banner detection (OneTrust, CookieBot)
- IAB TCF v2.0 cookie enforcement
- Form submission

## Platform Support

| Platform | TEE Backend | Protection Level |
|----------|-------------|------------------|
| macOS (T2/Silicon) | Secure Enclave | Hardware |
| Linux (SGX) | Intel SGX | Hardware |
| Linux/Windows | TPM 2.0 | Hardware |
| All | SQLCipher + Argon2id | Software |
| All | Ephemeral | Session-only |

