//! Single-use child creation authority and retained evidence; no application data or runtime.

#[cfg(test)]
mod tests;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Admit one immutable approval with its artifact bytes.
pub const STAGE: u64 = 20;
/// Read a small evidence projection without artifact bytes.
pub const STATUS: u64 = 21;
/// Reconstruct upload/install outcomes through observations, never repeat them.
pub const RECONCILE: u64 = 22;
/// Test-build-only fault endpoint; production managers do not export it.
pub const INJECT: u64 = 23;
/// Maximum worker artifact (uncompressed).
pub const MAX_WASM: usize = 524_288;
/// Fixed one-worker attachment ceiling.
pub const CREATION_CYCLES: u64 = 1_000_000_000_000;
/// Minimum liquid reserve retained alongside the worker attachment.
pub const RESERVE_CYCLES: u64 = 500_000_000_000;
/// Bounded operation evidence includes five hashes and two principal strings.
pub const LIMITS: ic_wire::Limits = ic_wire::Limits {
    allocation: 65_536,
    nodes: 1024,
    depth: 12,
    bytes: 4096,
};
/// Large staging envelope; observations use the separate small evidence policy.
pub const STAGE_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: MAX_WASM + 4096,
    // Wire charges eight allocation bytes per blob byte, plus the frame ceiling.
    allocation: MAX_WASM * 8 + MAX_WASM + 4096 + 65_536,
    nodes: 1024,
    depth: 12,
};

/// Exact immutable controller-approved work. No second slot or reset exists.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Approval {
    /// Exact manager principal on the caller-selected network.
    pub manager: String,
    /// SHA-256 of the retained raw Wasm.
    pub module_hash: [u8; 32],
    /// Must equal the fixed single-worker ceiling.
    pub creation_cycles: u64,
    /// Earliest dispatch, within one hour of admission.
    pub execute_at_ns: u64,
    /// Last time a new effect may be issued, at most ten minutes after dispatch.
    pub expires_at_ns: u64,
}

/// Full staging payload; the manager keeps bytes before scheduling work.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Stage {
    /// Exact one-slot authority.
    pub approval: Approval,
    /// Raw Wasm retained at the effect owner.
    #[serde(with = "serde_bytes")]
    pub wasm: Vec<u8>,
}

/// Committed phase. Issued states never dispatch the same effect again.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub enum Phase {
    /// No authority admitted.
    #[default]
    Empty,
    /// One admitted creation waiting for its dispatch time.
    Scheduled,
    /// Creation issued; its irreplaceable principal has not committed.
    CreateIssued,
    /// Principal retained in a completed callback message.
    Created,
    /// Upload issued; chunk evidence has not committed.
    UploadIssued,
    /// Approved chunk observed or its reply retained.
    Uploaded,
    /// Installation issued; outcome has not committed.
    InstallIssued,
    /// Install reply committed; module/controller observation remains.
    Installed,
    /// Module and sole manager controller observed; readiness remains.
    Verified,
    /// Matching binding and worker-reported readiness observed.
    Ready,
    /// Deadline passed before the next effect; no renewal is admitted.
    Expired,
    /// A local reserve check prevented dispatch. The slot stays consumed.
    ReserveBlocked,
}

/// Bounded failure evidence, not permission to repeat a management effect.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Failure {
    /// Provider rejection with its original numeric code.
    Rejected(u32),
    /// Runtime failure with outcome not qualified as absent.
    Uncertain,
    /// Returned bytes could not be admitted.
    InvalidReply,
    /// Observation disagrees with the approved hash/controller or readiness.
    Mismatch,
}

/// Public operation evidence; never exports artifact bytes or private profiles.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct Status {
    /// Original authority retained after every outcome.
    pub approval: Option<Approval>,
    /// Current committed phase.
    pub phase: Phase,
    /// Returned worker principal, only after retained creation evidence.
    pub worker: Option<String>,
    /// Creation, upload and install dispatch counts; each is at most one.
    pub attempts: [u8; 3],
    /// SHA-256 commitments to exact encoded management arguments in the same order.
    pub requests: [Option<[u8; 32]>; 3],
    /// Refund observed in the creation callback; absence preserves full uncertainty.
    pub creation_refund: Option<u128>,
    /// Most recent failure; a later observation can reconstruct evidence.
    pub failure: Option<Failure>,
    /// Management-observed module, distinct from the approval.
    pub observed_module: Option<Vec<u8>>,
    /// Current physical stable page use.
    pub stable_pages: u64,
    /// Distinct executable marker for the maintenance rehearsal.
    pub maintenance_candidate: bool,
}

/// Staging refusal; the unchanged original slot remains authoritative.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Refusal {
    /// Different work already owns the only slot.
    Occupied,
    /// Artifact is oversized, not raw Wasm, or has the wrong digest.
    Artifact,
    /// Wrong manager or cycle ceiling.
    Binding,
    /// Invalid dispatch/deadline interval.
    Time,
    /// Insufficient liquid cycles for attachment plus reserve.
    Reserve,
}

/// Hash exact artifact or management argument bytes.
#[must_use]
pub fn digest(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

impl Stage {
    /// Validate content, recipient, ceiling and admission time.
    ///
    /// # Errors
    /// Returns the first failed admission condition; controller guards live at the endpoint.
    pub fn validate(&self, manager: &str, now: u64) -> Result<(), Refusal> {
        let a = &self.approval;
        if a.manager != manager || a.creation_cycles != CREATION_CYCLES {
            return Err(Refusal::Binding);
        }
        if self.wasm.len() > MAX_WASM
            || !self.wasm.starts_with(b"\0asm\x01\0\0\0")
            || digest(&self.wasm) != a.module_hash
        {
            return Err(Refusal::Artifact);
        }
        if a.execute_at_ns <= now
            || a.execute_at_ns - now > 3_600_000_000_000
            || a.expires_at_ns <= a.execute_at_ns
            || a.expires_at_ns - a.execute_at_ns > 600_000_000_000
        {
            return Err(Refusal::Time);
        }
        Ok(())
    }
}
