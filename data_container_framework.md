# Data Container Framework: Server-Side Integration & Sovereign Ad Protocols

**Version 1.2.0**

## 1. Executive Summary

This framework defines how server-side infrastructure ("Resource Servers") can PROFICIENTLY integrate with the **Solid Pods (spvault)** sovereign agent architecture. It moves beyond simple bot defense to enabled a cooperative ecosystem where servers can:
1.  **Automate Consent**: Enforce user privacy preferences (cookies, tracking) server-side without fragile UI banners.
2.  **Verify Sovereignty**: Distinguish legitimate, high-entropy sovereign agents from low-value scrapers using the **Sovereign Agent Access Profile (SAAP)**.
3.  **Enable Intent-Casting**: Allow Ad Engines and Services to query agent data (memory/intent) through privacy-preserving semantic search APIs, flipping the traditional tracking model.
4.  **Offer Frictionless Auth**: Provide a captcha-free, multi-IdP login experience that relies on cryptographic proofs rather than user friction.
5.  **Shape Traffic Dynamically**: Use identity entropy to prioritize legitimate ad-hoc scraping only when capacity exists, while guaranteeing reliable access for sovereign agents during load spikes.

## 2. The Sovereign Agent Access Profile (SAAP)

Servers must implement a rigid access profile to recognize and privilege sovereign agents. This "handshake" replaces the "humanness" of a captcha with the "sovereignty" of a cryptographic identity.

### 2.1 The SAAP Handshake Protocol
Every request from a Sovereign Agent MUST include:

1.  **Identity**: `Authorization: DPoP <access_token>` (Solid-OIDC bound)
    *   *Claim*: `webid` (The human principal)
    *   *Claim*: `azp` (The agent software/client ID)
2.  **Proof**: `DPoP: <jwt_proof>`
    *   *Constraint*: Bound to HTTP method/URI.
    *   *Constraint*: Signed by TEE-protected key (if available).
3.  **Governance**: `Sec-Consent-Digest: <hash>`
    *   *Purpose*: Proves the agent knows the user's current consent manifest.

### 2.2 Server-Side Verification Logic
A minimally compliant SAAP server:
1.  **Validates Token**: Verifies `iss` against a hardened allowlist (no open federation for sensitive ops).
2.  **Enforces DPoP**: Rejects any token not bound to the request signature (mitigates replay/theft).
3.  **Resolves Principal**: Maps the `webid` to a persistent internal User ID.

## 3. Automated Server-Side Consent

Instead of asking the agent to "click" a cookie banner, the server **pre-calculates** the compliant response based on the Agent's identity.

### 3.1 The "Consent-First" Response
When a SAAP-verified request arrives:

1.  **Lookup**: Server checks its `ConsentCache` for the principal's `ConsentManifest`.
    *   *Cache Miss*: Server performs a **Background Fetches** of the manifest from the user's Pod (using the incoming Access Token if needed).
    *   *Default*: Apply "Strict/Deny-All" policy until manifest is loaded.
2.  **Filter**:
    *   **Cookies**: Server filters `Set-Cookie` headers. If `manifest.allow_analytics == false`, the `_ga` cookie is stripped *before* the response leaves the server.
    *   **Scripts**: Server injects a custom `Content-Security-Policy` header. If `manifest.allow_ads == false`, ad network domains are removed from `script-src`.
3.  **Attest**: Server adds a `Sec-Consent-Applied: <category_list>` header to prove compliance to the agent's auditor.

## 4. Controlled Data Sharing & Handles

Agents do not "login" and give servers full access. They grant specific, time-bound access relative to a task.

### 4.1 The Handle Mechanism
To share data (e.g., "My Shipping Address" or "My Semantic Interest Vector"), the agent creates a **Handle**:

*   **Structure**: `https://pod.example/handles/{uuid}`
*   **Permissions**: Read-only, Time-limited (TTL=5m), Purpose-bound (ODRL).

### 4.2 Server Dereferencing
1.  Agent sends: `POST /api/checkout { "shipping_address": "https://pod.example/handles/uuid-123" }`
2.  Server performs: `GET https://pod.example/handles/uuid-123`
    *   *Auth*: Server presents its own Client ID + DPoP.
    *   *Broker*: The User's Vault Broker verifies the Server's identity matches the Handle's authorized audience.
3.  **Result**: Server gets the JSON data. Agent gets an audit log entry ("Amazon read address at 12:00").

## 5. Ad Engine Semantic Search API

This section defines the **Privacy-Preserving Intent-Casting** protocol. Traditional ads track users to guess intent. Sovereign agents **broadcast** intent anonymously and filter ads locally.

### 5.1 The "Reverse Ad" Architecture

*   **User**: "I want to buy a vintage mechanical keyboard."
*   **Agent**: Finds "Mechanical Keyboard Store".
*   **Goal**: See relevant inventory/ads capabilities without the Store tracking the user's browsing history.

### 5.2 The API Specification

**Endpoint**: `POST /api/v1/ad-query/semantic`

**Request (Agent -> Ad Engine)**:
```json
{
  "query_vector": [0.12, -0.45, 0.88, ...], // Embedding of "vintage mechanical keyboard"
  "context_signals": {
    "intent": "purchase",
    "budget_range": [100, 300],
    "region": "US-EAST"
  },
  "privacy_mode": "anonymous" // No WebID provided, or ephemeral WebID used
}
```

**Response (Ad Engine -> Agent)**:
```json
{
  "candidates": [
    {
      "id": "ad-7788",
      "title": "Restored Model M Keyboard",
      "vector": [0.11, -0.42, 0.90, ...], // Allow agent to verify relevance
      "bid_value": 0.50, // Opsional: dynamic pricing
      "content_handle": "https://ad-server.com/content/ad-7788"
    }
  ]
}
```

### 5.3 Local ranking & Conversion
1.  **Local Filtering**: The Agent compares the candidate vectors against its full, private memory store.
    *   *Check*: "User explicitly said they hate loud switches." -> Agent filters out "Blue Switch" ads locally.
2.  **Display**: Agent presents the curated list to the user.
3.  **Attributed Click**:
    *   If user selects an ad, Agent calls `GET /api/v1/ad-click/{id}`.
    *   **New**: Agent creates a **Conversion Handle** that grants the Ad Engine proof of the transaction *without* revealing the user's persistent identity history.

### 5.4 Benefits for Ad Engines
*   **Higher Quality Signal**: Agents provide explicit intent loops ("I am looking for X"), removing the need for probabilistic tracking.
*   **Zero-Liability**: The Ad Engine holds no PII, no user history, and no concern for GDPR "Right to be Forgotten" calls on user profiles they don't have.
*   **Fraud Reduction**: Every query is signed by a hardware-attested Sovereign Agent (via SAAP), eliminating bot fraud.

## 6. Server-Side Verification & Traffic Management

Scale without being overwhelmed. Sovereign Agent traffic differs fundamentally from bot traffic: it allows **Identity-Based** rather than **IP-Based** management.

### 6.1 Identity Verification Hierarchy
Servers should implement a layered verification strategy to balance security and latency:

*   **Layer 1 (Fast): Token & Issuer**:
    *   Validate DPoP-bound JWT signature.
    *   Check `iss` against strict allowlist (e.g., specific corporate IdPs or trusted Pod providers).
    *   *Result*: Drops 99% of unauthorized traffic at the edge (Cloudflare Worker/NGINX).
*   **Layer 2 (Cached): Principal Resolution**:
    *   Map `webid` to internal user record.
    *   Check `azp` (Client ID) for banned software versions.
*   **Layer 3 (Slow): Deep Dereference**:
    *   Fetch WebID Profile triples. **ONLY** on first contact or cache miss.
    *   *DoS Prevention*: Strict rate limit on outbound dereferences (e.g., 1 per minute per WebID).

### 6.2 Advanced Fingerprinting (JA4+)
While WebID proves identity, **JA4+ Fingerprinting** proves environment integrity.

*   **JA4 (TLS)**: Verifies the agent is running the expected TEE-capable client (e.g., custom Rust/Tls backend) and not a generic Python script.
*   **JA4H (HTTP)**: Correlates header ordering with the declared Sovereign Agent version.
*   **Policy**:
    *   *Match*: Allow high throughput.
    *   *Mismatch*: Flag as anomaly; throttle rate limits; require Step-Up (e.g., interactive captcha or fresh TEE attestation).

### 6.3 Identity-Based Traffic Management
Move away from IP-based rate limiting, which hurts shared networks (CGNAT/VPNs).

| Feature | IP-Based (Legacy) | WebID-Based (Sovereign) |
| :--- | :--- | :--- |
| **Granularity** | Coarse (Device/Network) | Fine (User/Agent Instance) |
| **Evasion** | Easy (Rotate IP/Proxy) | Hard (Cannot forge signed WebID/DPoP) |
| **Fairness** | Punishes shared IPs | Isolates abusive actors precisely |
| **Policy** | `Limit: 100 req/ip/min` | `Limit: 1000 req/webid/min` |

### 6.4 DPoP Replay Protection
To prevent token theft from becoming session hijacking:
1.  **Nonce Enforcement**: Server issues `DPoP-Nonce` header.
2.  **JTI Tracking**: Cache `jti` (JWT ID) for 5 minutes (token lifetime) to prevent replay.
3.  **Binding Check**: Ensure `ath` (Access Token Hash) in DPoP proof matches the Bearer token.

### 6.5 Threat Model Transition
| Threat | Mitigation |
| :--- | :--- |
| **Credential Stuffing** | DPoP makes stolen tokens useless without the TEE-protected private key. |
| **Scraping** | Rate limit by WebID; ban WebIDs that violate Terms of Service. |
| **Impersonation** | Strict Issuer Allowlist prevents "Self-Signed" fake identities. |

## 7. Captcha-Free Seamlessness & Traffic Resilience

This architecture provides a dual benefit: friction is removed for legitimate agents through cryptographic trust, while resilience is maintained through intelligent throttling.

### 7.1 The "Proof-of-Humanity" Replacement
Traditional captchas (click the hydrants) attempt to filter bots by testing visual cognition. SAAP replaces this with **Proof-of-Sovereignty**:
*   **Concept**: If an agent can sign a request with a key remotely attested to a Secure Enclave (TEE), it effectively proves it is a *compliant software agent* acting on behalf of a human.
*   **Benefit**: No visual challenges. No "I am not a robot" checkboxes.
*   **Implementation**:
    *   Server checks `DPoP` signature + `Attestation-Class` header.
    *   If valid TEE signature: **Bypass Captcha**.
    *   If Software-only signature: **Serve Challenge** (or restrict to read-only).

### 7.2 Multi-IdP Federation (Bring Your Own Identity)
Websites no longer need to manage thousands of integration points.
*   **Protocol**: Resource Servers trust a list of *Root Authorities* (e.g., "Solid Community", "Inrupt Enterprise").
*   **Mechanism**: The user provides their `WebID`. The Server performs **OIDC Discovery** on the WebID's issuer.
*   **Seamless Login**:
    1.  Agent sends `Authorization: DPoP <token>` in the first request.
    2.  Server validates the token relies on a trusted issuer in its `allowlist`.
    3.  **Zero-Click Login**: The server automatically creates/resumes a session for that WebID. No "Login with Google" redirects if the Agent already holds a valid token.

### 7.3 Resilience: Circuit Breakers & Fallbacks
When traffic spikes (e.g., viral event, DDOS), the Server must degrade gracefully without blocking legitimate users.

**The "Whitelisting Lane" Circuit Breaker**:
*   **Normal Mode**: Full WebID dereferencing and profile fetching.
*   **Panic Mode (Load > 80%)**:
    1.  **Stop Dereferencing**: Do not fetch internal WebID profiles. Authenticate based strictly on the JWT signature (stateless verification).
    2.  **Whitelist-Only Priority**: Agents with WebIDs on the **WebID Whitelist** retain full access.
    3.  **Unknowns Throttled**: Valid tokens from *unknown* WebIDs are shunted to a "Low Priority" queue (high latency, read-only).
    4.  **Bad Actors Dropped**: Invalid signatures or DPoP failures are dropped at the edge (Layer 1).

**Whitelist Management**:
*   **Explicit**: "Gold Tier" subscription = WebID added to Redis Whitelist set.
*   **Implicit**: "Reputation Score" > 50 = Auto-added to whitelist.
*   **Behavior**: Whitelisted agents bypass generic rate limits and CAPTCHAs even during high load, ensuring reliability for the most valuable users.

## 8. Dynamic Traffic Shaping & Multi-IdP Load Balancing

Robustness comes from adaptability. This section defines how servers switch postures based on load, leveraging Multi-IdP verification to distribute the auth burden.

### 8.1 Traffic Posture Modes

Servers should operate in one of two modes depending on real-time capacity monitoring.

#### Mode A: "Green Light" (Low/Normal Traffic)
**Policy: Open but Audited**
*   **Sovereign Agents**: Full Service. Premium rate limits. No challenges.
*   **Anonymous Traffic**: **Allowed**. Scrapers and bots without WebID/DPoP credentials are permitted to access public read-only endpoints (e.g., product pages), subject to standard IP-based rate limits.
*   **Rationale**: Maximizes reach and indexing (SEO) when server capacity is abundant. Identity verification is used mainly for personalization (Consent/Login).

#### Mode B: "Red Light" (High/Spike Traffic)
**Policy: Identity-Gated First**
*   **Sovereign Agents**: **Prioritized**.
    *   Requests with valid WebID + DPoP + JA4 Match are processed.
    *   Higher priority given to TEE-Attested agents.
*   **Anonymous Traffic**: **Dropped at Edge**.
    *   If `Authorization` header is missing or invalid: `429 Too Many Requests` or `503 Service Unavailable`.
    *   **Rationale**: During a DDoS or viral spike, non-attributable traffic is the first to be shed. This guarantees uptime for authenticated human-backed agents.

### 8.2 Using Multi-IdP for Verification Scaling
Traditional OAuth relies on a centralized auth server (spof). Solid Multi-IdP distributes this load.

*   **Distributed Validation**: The Resource Server verifies tokens from *many* issuers (Google, Inrupt, Community Pods).
    *   *Adantage*: A DDoS on one Identity Provider (e.g., "solidcommunity.net") does not take down the Resource Server's ability to serve users from other providers (e.g., "inrupt.com").
*   **Stateless Edge Verification**:
    *   Resource Servers cache **JWKS (Public Keys)** for trusted issuers at the Edge (CDN).
    *   *Result*: 99% of auth decisions happen at the edge without touching the origin database, enabling massive scale for Sovereign Agent traffic.

### 8.3 JA4 + WebID: The Ultimate Filter
During "Red Light" mode, the server combines signals to reject sophisticated bots:

*   **Bot**: Presents valid WebID but uses Python `requests` (JA4 mismatch). -> **Block**.
*   **Scraper**: Uses valid Browser (JA4 match) but no WebID. -> **Block**.
*   **Sovereign Agent**: Presents Valid WebID + Valid DPoP + JA4 Match (Browser Bridge). -> **Allow**.

This combination ensures that when resources are scarce, they are consumed only by verifiable agents.
