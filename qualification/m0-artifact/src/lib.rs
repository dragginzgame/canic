//! Test-only custody of one approved artifact and one autonomous installation.
//! No creation, funding, artifact catalogue or production scheduler is supplied.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[cfg(target_family = "wasm")]
mod canister;
#[cfg(test)]
mod tests;

/// Stage one exact artifact and its installation authority.
pub const STAGE: u64 = 1;
/// Read retained evidence without returning the artifact bytes.
pub const STATUS: u64 = 2;
/// Observe the target module; never repeat installation.
pub const RECONCILE: u64 = 3;
/// Select post-upgrade trap injection with a Wire boolean argument.
pub const UPGRADE: u64 = 4;
/// Identity of the isolated stable checkpoint frame.
pub const CHECKPOINT: u64 = 5;
/// Maximum approved uncompressed Wasm in this experiment: 64 KiB.
pub const MAX_WASM_BYTES: usize = 65_536;
/// Bounded large-frame policy for admission and the single checkpoint.
pub const ARTIFACT_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 262_144,
    allocation: 2_097_152,
    nodes: 131_072,
    depth: 16,
};
/// Small status and lifecycle argument frames do not inherit upload budgets.
pub const LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 4096,
    allocation: 32768,
    nodes: 4096,
    depth: 16,
};

/// Irreplaceable result capture window to interrupt.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Fault {
    /// Retain the install reply, then verify the installed module.
    None,
    /// Write the uploaded chunk hash, then trap before that callback commits.
    AfterUploadWrite,
    /// Write the reply evidence and trap before that callback commits.
    AfterResultWrite,
}

/// Exact one-shot installation approved by the executor's controller.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Approval {
    /// Already provisioned empty worker, controlled solely by the executor.
    #[serde(with = "serde_bytes")]
    pub target: Vec<u8>,
    /// Expected IC module hash, SHA-256 of the uncompressed Wasm.
    pub sha256: [u8; 32],
    /// Bytes retained before the operator leaves.
    #[serde(with = "serde_bytes")]
    pub wasm: Vec<u8>,
    /// Earliest dispatch time, at most one hour after admission.
    pub execute_at_ns: u64,
    /// Deliberately selected callback interruption.
    pub fault: Fault,
}

/// Admission failures do not consume the fixture's single slot.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum AdmissionError {
    /// Target is not an ordinary non-anonymous principal.
    Target,
    /// Module is empty, too large or lacks the raw Wasm version-one header.
    Wasm,
    /// Bytes do not match the separately approved identity.
    Hash,
    /// Dispatch is not within the fixture's future admission window.
    Time,
    /// An existing approval cannot be replaced, even after failure.
    Occupied,
}

impl Approval {
    /// Check bounded content and time before retaining authority.
    ///
    /// # Errors
    /// Returns the first failed content or time requirement; authentication and
    /// management-controller checks belong to the installed endpoint/provider.
    pub fn validate(&self, now_ns: u64) -> Result<(), AdmissionError> {
        self.validate_contents()?;
        if self.execute_at_ns <= now_ns
            || self.execute_at_ns.saturating_sub(now_ns) > 3_600_000_000_000
        {
            return Err(AdmissionError::Time);
        }
        Ok(())
    }

    fn validate_contents(&self) -> Result<(), AdmissionError> {
        if self.target.is_empty() || self.target.len() > 29 || self.target == [4] {
            return Err(AdmissionError::Target);
        }
        if self.wasm.len() > MAX_WASM_BYTES || !self.wasm.starts_with(b"\0asm\x01\0\0\0") {
            return Err(AdmissionError::Wasm);
        }
        if artifact_hash(&self.wasm) != self.sha256 {
            return Err(AdmissionError::Hash);
        }
        Ok(())
    }
}

/// Compute the protocol's uncompressed Wasm identity.
#[must_use]
pub fn artifact_hash(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

/// Evidence states for this single installation; uncertainty never permits retry.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub enum Phase {
    /// No approval admitted.
    #[default]
    Empty,
    /// Artifact and authority retained; waiting for the approved dispatch time.
    Scheduled,
    /// Chunk upload was issued; its callback evidence has not committed.
    UploadIssued,
    /// Install was issued; its returned result has not committed.
    Issued,
    /// A successful reply committed before module verification.
    Installed,
    /// A management observation matches both hash and sole controller.
    Verified,
    /// The installation callback rejected; no replacement attempt is admitted.
    Rejected(u32),
}

/// Small evidence projection; the retained artifact is not part of this reply.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct Status {
    /// Current retained phase.
    pub phase: Phase,
    /// Number of issued installation attempts.
    pub attempts: u8,
    /// Number of uploads issued to the executor's own management chunk store.
    pub upload_attempts: u8,
    /// Exact upload request commitment, retained before dispatch.
    pub upload_request_sha256: Option<[u8; 32]>,
    /// Matching chunk identity retained from the upload callback.
    pub chunk_hash: Option<Vec<u8>>,
    /// Whether the success callback committed, distinct from later observation.
    pub reply_retained: bool,
    /// Digest of the exact encoded management request retained before dispatch.
    pub request_sha256: Option<[u8; 32]>,
    /// Exact target principal bytes.
    pub target: Option<Vec<u8>>,
    /// Approved artifact identity.
    pub sha256: Option<[u8; 32]>,
    /// Approved artifact size.
    pub wasm_bytes: usize,
    /// Exact module identity observed through the management API.
    pub observed_module: Option<Vec<u8>>,
    /// Marker proving the second executable is active; not a schema version.
    pub maintenance_candidate: bool,
    /// Physical stable pages occupied by this isolated fixture.
    pub stable_pages: u64,
}

#[cfg(any(target_family = "wasm", test))]
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
struct Retained {
    approval: Option<Approval>,
    phase: Phase,
    attempts: u8,
    upload_attempts: u8,
    upload_request_sha256: Option<[u8; 32]>,
    chunk_hash: Option<Vec<u8>>,
    reply_retained: bool,
    request_sha256: Option<[u8; 32]>,
    observed_module: Option<Vec<u8>>,
}
