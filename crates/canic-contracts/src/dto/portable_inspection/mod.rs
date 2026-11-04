//! Bounded operator observations; retained sessions are not authorization decisions.
use ic_platform_types::Principal;

/// Exclusive caller cursor and a nonzero bounded page size.
#[derive(Clone, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct Request {
    pub after: Option<Principal>,
    pub limit: u16,
}

/// Public session metadata only; credentials and replay fingerprints stay private.
#[derive(Clone, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct SessionObservation {
    pub caller: Principal,
    pub issuer: Principal,
    pub subject: Principal,
    pub scopes: Vec<String>,
    pub established_at_ns: u64,
    pub expires_at_ns: u64,
    pub generation: u64,
    pub expired: bool,
    pub stale_generation: bool,
}

/// One current observation; later pages may see concurrent admission or logout.
#[derive(Clone, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct Page {
    pub observed_at_ns: u64,
    pub generation: u64,
    pub sessions: Vec<SessionObservation>,
    pub next_after: Option<Principal>,
}

/// Compact refusals never expose backing-store diagnostics.
#[derive(Clone, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub enum Refusal {
    InvalidPage,
    Unavailable,
    Clock,
}

/// Declared receiver policy, independent of current runtime readiness.
#[derive(Clone, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct DeclaredPolicy {
    pub fleet: crate::ids::FleetKey,
    pub role: crate::ids::CanisterRole,
    pub allowed_scopes: Vec<String>,
    pub default_session_ttl_secs: u64,
    pub maximum_session_ttl_secs: u64,
    pub receiver: Option<ReceiverPolicy>,
}

/// Time and credential bounds selected by the application owner.
#[derive(Clone, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct ReceiverPolicy {
    pub clock_skew_ns: u64,
    pub max_login_ttl_ns: u64,
    pub max_proof_ttl_ns: u64,
    pub max_certificate_age_ns: u64,
}

/// Fresh protected enrollment; possession of this report grants no authority.
#[derive(Clone, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct EnrollmentObservation {
    pub authority: Principal,
    pub revision: u64,
    pub generation: u64,
    pub expires_at_ns: u64,
    pub issuers: Vec<crate::dto::portable_enrollment::PortableIssuerEnrollment>,
}

/// Configuration alone never implies an active receiver or fresh issuer authority.
#[derive(Clone, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub enum RuntimeState {
    Disabled,
    Unavailable,
    UnqualifiedClock,
    WaitingForEnrollment,
    Ready(EnrollmentObservation),
}

/// Declared configuration and current protected state observed in one message.
#[derive(Clone, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct PolicyReport {
    pub observed_at_ns: u64,
    pub authority_generation: u64,
    pub declared: Option<DeclaredPolicy>,
    pub runtime: RuntimeState,
}
