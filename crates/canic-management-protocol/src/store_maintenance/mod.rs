//! Root-owned Store maintenance intent, separate from application slots and Store-local admission.

#[cfg(test)]
mod tests;

mod preservation;

pub use preservation::{Preservation, Scan};
use serde::{Deserialize, Serialize};

/// Retain one immutable maintenance approval before remote effects.
pub const STAGE: u64 = 50;
/// Advance only the exact retained attempt; issued stop effects are observed, never repeated.
pub const ADVANCE: u64 = 51;
/// Read the surviving Root journal without resuming work.
pub const OBSERVE: u64 = 52;
/// Explicitly install the approved bytes once, then permit restart with admission closed.
pub const INSTALL: u64 = 53;
/// Explicitly activate only the verified replacement and exact retained Store attempt.
pub const ACTIVATE: u64 = 54;
/// Public diagnostic query; never authenticated completion custody or current Store readiness.
pub const STATUS: u64 = 55;
/// Reconcile already-issued effects by observation; never issue fresh maintenance effects.
pub const RECOVER: u64 = 56;
/// Read one exact retained approval, including completed historical attempts.
pub const OBSERVE_ATTEMPT: u64 = 59;
/// Retained Store upgrades, including the current attempt. History is never evicted.
pub const MAX_UPGRADES: usize = 8;
/// Two bounded metadata commitments and issued-effect evidence; child observations keep their budget.
pub const LIMITS: ic_wire::Limits = ic_wire::Limits {
    allocation: 262_144,
    nodes: 2048,
    depth: 16,
    ..super::LIMITS
};
/// First infrastructure replacement fits one management-cache chunk, independently of worker limits.
pub const MAX_MODULE_BYTES: usize = 1_048_576;
/// One replacement request; bytes remain transient in Root and durable with Host.
pub const INSTALL_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: MAX_MODULE_BYTES + 4096,
    allocation: MAX_MODULE_BYTES * 9 + 69_632,
    ..LIMITS
};

/// Host supplies a bounded replacement without making Root its durable artifact repository.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Install {
    /// Exact existing maintenance approval, not fresh or extended authority.
    pub approval: Approval,
    /// Raw Wasm, at most `MAX_MODULE_BYTES` for this first infrastructure path.
    #[serde(with = "serde_bytes")]
    pub wasm: Vec<u8>,
}

/// Exact Store/code boundary and its already-reviewed replacement commitment.
/// Installation and activation each require their explicit controller request.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Approval {
    /// Store controlled solely by this Root; never Root or an application child.
    pub store: String,
    /// Exact current executable.
    pub previous_module: [u8; 32],
    /// Exact proposed different executable, whose bytes remain with Host.
    pub module_hash: [u8; 32],
    /// Store-local monotonic attempt, retained identically by both owners.
    pub attempt: ic_wasm_store_protocol::MaintenanceAttempt,
    /// Last dispatch time for every new effect; at most one hour from staging.
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
    /// One upload to Store's management cache issued; lost replies recover by chunk observation.
    UploadIssued,
    /// Exact approved replacement chunk observed in Store's management cache.
    Uploaded,
    /// Installation was issued once; uncertainty never permits its replay.
    InstallIssued,
    /// Approved replacement observed under sole Root control, still stopped.
    Installed,
    /// Start was issued once; recover by observing physical status.
    StartIssued,
    /// Running with admission closed while comparing the complete metadata commitment.
    Verifying,
    /// Metadata preservation matches; explicit activation is still required.
    Verified,
    /// Matching activation issued once; observe Store to recover an uncertain result.
    ActivateIssued,
    /// Matching open admission observed; retain the completed operation without resetting it.
    Ready,
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
    /// Scan contract and closed catalog commitment. CUJ5 records retain absent historical evidence.
    pub preservation: Scan,
    /// Installation custody, absent before explicit installation or in migrated CUJ5 records.
    pub installation: Option<Installation>,
}

/// Exact issued effects and replacement verification. Contains no Wasm bytes.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Installation {
    /// Hash of the exact issued upload request; bytes remain with Host.
    pub upload_request: [u8; 32],
    /// Hash of the exact encoded upgrade request, including its original sender version.
    pub request: Option<[u8; 32]>,
    /// Root version included in that original request, never renewed on recovery.
    pub sender_version: Option<u64>,
    /// Start request commitment; absent until issued.
    pub start_request: Option<[u8; 32]>,
    /// Independent post-upgrade scan, compared with the retained pre-stop scan.
    pub verification: Scan,
    /// Exact Store activation frame commitment; absent until explicitly issued.
    pub activation_request: Option<[u8; 32]>,
}
