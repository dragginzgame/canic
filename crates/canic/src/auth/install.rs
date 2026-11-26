//! Passive fresh-install envelope. The application owns the nested argument bytes.
use super::enrollment::types::Principal;
use ic_wire::{Direction, Encoding, Limits};
use serde::{Deserialize, Serialize};

const METHOD: u64 = ic_wire::method_id("canic.auth.stable_receiver_install.v1");
const FRAME: Encoding = Encoding::Frame {
    method: METHOD,
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

/// Fleet-approved receiver configuration and unchanged inner app payload.
#[derive(Debug, Deserialize, Serialize)]
pub struct Install {
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
    pub limits: LimitsPolicy,
    /// Original app initialization payload.
    pub application: Vec<u8>,
}

fn matches(bytes: &[u8], method: u64) -> bool {
    bytes
        .first_chunk()
        .is_some_and(|header| ic_wire::validate_request_header(method, header).is_ok())
}

impl Install {
    /// Recognize this receiver envelope without treating arbitrary application bytes as Auth.
    #[must_use]
    pub(crate) fn is_frame(bytes: &[u8]) -> bool {
        // Recognize retired frames so topology readers reject them through decode.
        [METHOD, 0x4341_5554_4807, 0x4341_5554_4808]
            .into_iter()
            .any(|method| matches(bytes, method))
    }

    /// Encode exact bytes for this fresh installation.
    /// # Errors
    /// Refuses an envelope exceeding the install budget.
    pub fn encode(&self) -> Result<Vec<u8>, ic_wire::Error> {
        ic_wire::encode(self, FRAME, LIMITS)
    }

    /// Decode only this versioned install frame.
    /// # Errors
    /// Refuses wrong directions, identities, malformed or oversized payloads.
    pub fn decode(bytes: &[u8]) -> Result<Self, ic_wire::Error> {
        ic_wire::decode(bytes, FRAME, LIMITS)
    }
}
