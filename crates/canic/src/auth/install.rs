//! Passive fresh-install envelope. The application owns the nested argument bytes.
use super::enrollment::types::Principal;
use ic_wire::{Direction, Encoding, Limits};
use serde::{Deserialize, Serialize};

const PREVIOUS_METHOD: u64 = 0x4341_5554_4807;
const METHOD: u64 = 0x4341_5554_4808;
const FRAME: Encoding = Encoding::Frame {
    method: METHOD,
    direction: Direction::Request,
};
const PREVIOUS_FRAME: Encoding = Encoding::Frame {
    method: PREVIOUS_METHOD,
    direction: Direction::Request,
};
const LIMITS: Limits = Limits {
    bytes: 4096,
    allocation: 65536,
    nodes: 8192,
    depth: 8,
};

/// Application-selected limits on top of Auth's bounded receiver profile.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LimitsPolicy {
    /// Default lease requested by a session.
    pub default_session_secs: u64,
    /// Maximum lease allowed by the receiver.
    pub max_session_secs: u64,
    /// Maximum signed credential proof lifetime.
    pub max_proof_ttl_ns: u64,
    /// Override the supplier's maximum physical sessions.
    pub max_sessions: Option<u32>,
    /// Maximum retained issuer/account/device-slot generation fences.
    pub max_admitted_logins: Option<u32>,
}

/// Storage lifetime selected at installation.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Storage {
    /// An upgrade closes sessions permanently.
    #[default]
    Volatile,
    /// Preserve the owner in its exclusive governed allocation.
    Stable,
}

/// Fleet-approved receiver configuration and unchanged inner app payload.
#[derive(Debug, Deserialize, Serialize)]
pub struct Install<P = LimitsPolicy> {
    /// Approved receiver storage lifetime.
    pub storage: Storage,
    /// Actual Root selected by the deployment plan.
    pub root: Principal,
    /// Accepted credential issuer.
    pub issuer: Principal,
    /// Canonical network identifier.
    pub network: [u8; 32],
    /// Logical fleet audience identifier.
    pub audience: [u8; 32],
    /// Accepted credential role.
    pub role: String,
    /// Selected receiver limits.
    pub limits: P,
    /// Original app initialization payload.
    pub application: Vec<u8>,
}

// Previous envelopes and host checkpoints predate the known-login override.
#[derive(Deserialize)]
pub(super) struct PreviousLimits {
    default_session_secs: u64,
    max_session_secs: u64,
    max_proof_ttl_ns: u64,
    max_sessions: Option<u32>,
}

impl Install<PreviousLimits> {
    pub(super) fn into_current(self) -> Install {
        Install {
            storage: self.storage,
            root: self.root,
            issuer: self.issuer,
            network: self.network,
            audience: self.audience,
            role: self.role,
            limits: LimitsPolicy {
                default_session_secs: self.limits.default_session_secs,
                max_session_secs: self.limits.max_session_secs,
                max_proof_ttl_ns: self.limits.max_proof_ttl_ns,
                max_sessions: self.limits.max_sessions,
                max_admitted_logins: None,
            },
            application: self.application,
        }
    }
}

fn matches(bytes: &[u8], method: u64) -> bool {
    bytes
        .first_chunk()
        .is_some_and(|header| ic_wire::validate_request_header(method, header).is_ok())
}

impl Install {
    /// Recognize this receiver envelope without treating arbitrary application bytes as Auth.
    #[must_use]
    #[cfg(feature = "topology")]
    pub(crate) fn is_frame(bytes: &[u8]) -> bool {
        matches(bytes, METHOD) || matches(bytes, PREVIOUS_METHOD)
    }

    /// Encode exact bytes for this fresh installation.
    /// # Errors
    /// Refuses an envelope exceeding the install budget.
    pub fn encode(&self) -> Result<Vec<u8>, ic_wire::Error> {
        if self.limits.max_admitted_logins.is_none() {
            // Preserve approved bytes and compatibility with previously built receivers.
            let fields = (
                self.storage,
                self.root,
                self.issuer,
                self.network,
                self.audience,
                &self.role,
                (
                    self.limits.default_session_secs,
                    self.limits.max_session_secs,
                    self.limits.max_proof_ttl_ns,
                    self.limits.max_sessions,
                ),
                &self.application,
            );
            return ic_wire::encode(&fields, PREVIOUS_FRAME, LIMITS);
        }
        ic_wire::encode(self, FRAME, LIMITS)
    }

    /// Decode only this versioned install frame.
    /// # Errors
    /// Refuses wrong directions, identities, malformed or oversized payloads.
    pub fn decode(bytes: &[u8]) -> Result<Self, ic_wire::Error> {
        if matches(bytes, PREVIOUS_METHOD) {
            let previous: Install<PreviousLimits> = ic_wire::decode(bytes, PREVIOUS_FRAME, LIMITS)?;
            return Ok(previous.into_current());
        }
        ic_wire::decode(bytes, FRAME, LIMITS)
    }
}
