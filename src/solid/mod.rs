///! Solid Pod Integration Module
///!
///! Provides identity, authentication, and consent management for Solid Pods.
///!
///! This module implements a polyglot architecture:
///! - TypeScript (@inrupt libraries) for protocol complexity via subprocess
///! - Rust for security-critical operations (DPoP, key management, policy enforcement)

#[cfg(feature = "solid-integration")]
pub mod ipc;

#[cfg(feature = "solid-integration")]
pub mod dpop;

#[cfg(feature = "solid-integration")]
pub mod identity;

#[cfg(feature = "solid-integration")]
pub mod auth;

#[cfg(feature = "solid-integration")]
pub mod consent;

#[cfg(feature = "solid-integration")]
pub mod tools;

#[cfg(feature = "solid-integration")]
pub mod guardrails;

// Re-export key types when feature is enabled
#[cfg(feature = "solid-integration")]
pub use self::{
    dpop::DPoPManager,
    identity::SolidIdentityClient,
    auth::SolidOidcClient,
    consent::ConsentManifest,
    tools::SolidPodTool,
    guardrails::ConsentEnforcementGuardrail,
};
