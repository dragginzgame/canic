//! Root-owned Store maintenance intent, separate from application slots and Store-local admission.
use serde::{Deserialize, Serialize};

/// Retain one immutable maintenance approval before remote effects.
pub const STAGE: u64 = 50;
/// Advance only the exact retained attempt; issued stop effects are observed, never repeated.
pub const ADVANCE: u64 = 51;
/// Read the surviving Root journal without resuming work.
pub const OBSERVE: u64 = 52;

/// Exact Store/code boundary and its already-reviewed replacement commitment.
/// The first runtime step qualifies closure and stopping only; it cannot install this module.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Approval {
    /// Store controlled solely by this Root; never Root or an application child.
    pub store: String,
    /// Exact current executable.
    pub previous_module: [u8; 32],
    /// Exact proposed different executable, whose bytes remain with Host.
    pub module_hash: [u8; 32],
    /// Store-local monotonic attempt, retained identically by both owners.
    pub attempt: canic_wasm_store_protocol::MaintenanceAttempt,
    /// Last dispatch time for closure and stopping; at most one hour from staging.
    pub expires_at_ns: u64,
}
/// Committed phases. Stopped is a drain observation, not a successful upgrade.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Phase {
    /// Root has fenced new child/publication mutations; no remote call issued yet.
    Reserved,
    /// Exact closure issued; observe Store to recover a lost reply.
    Closing,
    /// Matching Store closure is retained.
    Closed,
    /// One stop call issued; never reissue it after uncertainty.
    StopIssued,
    /// Sole-control, old code and stopped status observed after the issued call.
    Stopped,
}
/// Surviving evidence outside Store; independent of Store availability.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Status {
    /// Original immutable maintenance approval.
    pub approval: Approval,
    /// Captured Root controller; continuation requires the same current controller.
    pub operator: String,
    /// Latest committed phase.
    pub phase: Phase,
    /// Exact encoded stop request hash, present only after issuance.
    pub stop_request: Option<[u8; 32]>,
    /// Latest failure; never evidence that the issued effect did not happen.
    pub failure: Option<super::Failure>,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn store_maintenance_evidence_fits_the_existing_metadata_budget() {
        let status = Status {
            approval: Approval {
                store: "a".repeat(63),
                previous_module: [254; 32],
                module_hash: [255; 32],
                attempt: canic_wasm_store_protocol::MaintenanceAttempt {
                    sequence: u64::MAX,
                    operation: [255; 32],
                },
                expires_at_ns: u64::MAX,
            },
            operator: "b".repeat(63),
            phase: Phase::StopIssued,
            stop_request: Some([255; 32]),
            failure: Some(crate::Failure::Rejected(u32::MAX)),
        };
        let reply = Ok::<_, crate::Refusal>(status.clone());
        let bytes = ic_wire::native::encode_response(STAGE, &reply, crate::LIMITS).unwrap();
        assert_eq!(
            ic_wire::native::decode_response::<Result<Status, crate::Refusal>>(
                STAGE,
                &bytes,
                crate::LIMITS
            )
            .unwrap(),
            reply
        );
        let bytes = ic_wire::native::encode_response(OBSERVE, &Some(status.clone()), crate::LIMITS)
            .unwrap();
        assert_eq!(
            ic_wire::native::decode_response::<Option<Status>>(OBSERVE, &bytes, crate::LIMITS)
                .unwrap(),
            Some(status)
        );
    }
}
