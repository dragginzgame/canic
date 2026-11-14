//! One retained child upgrade, independent of the original creation approval.
use crate::{Failure, MAX_WASM, Refusal, digest};
use serde::{Deserialize, Serialize};

#[cfg(test)]
mod tests;

/// Admit one immutable upgrade and its bytes.
pub const STAGE: u64 = 24;
/// Read committed upgrade evidence.
pub const STATUS: u64 = 25;
/// Reconstruct uncertain upload/install outcomes without repeating effects.
pub const RECONCILE: u64 = 26;

/// Exact child, code transition and finite dispatch interval.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Approval {
    /// Enduring management owner.
    pub manager: String,
    /// Existing child; never a newly allocated output.
    pub child: String,
    /// Required installed module before upgrade.
    pub previous_module: [u8; 32],
    /// Approved replacement module.
    pub module_hash: [u8; 32],
    /// Earliest dispatch time in Unix nanoseconds.
    pub execute_at_ns: u64,
    /// Last time a new effect can be issued.
    pub expires_at_ns: u64,
}

/// Bounded artifact envelope.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Stage {
    /// Immutable authority.
    pub approval: Approval,
    /// Exact raw Wasm, retained before scheduling.
    #[serde(with = "serde_bytes")]
    pub wasm: Vec<u8>,
}

/// Committed progress; issued effects are never issued again.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub enum Phase {
    /// No upgrade admitted.
    #[default]
    Empty,
    /// Bytes and authority retained, waiting for dispatch.
    Scheduled,
    /// Old module, sole controller and child version retained.
    Checked,
    /// Upload intent committed with outgoing call.
    UploadIssued,
    /// Approved chunk retained in the management chunk store.
    Uploaded,
    /// Upgrade intent committed with outgoing call.
    InstallIssued,
    /// Successful upgrade reply retained.
    Installed,
    /// Replacement module, controller and exact version increment observed.
    Verified,
    /// Application readiness observed after verification.
    Ready,
    /// Deadline prevented another effect; authority remains consumed.
    Expired,
    /// Reserve prevented another effect; authority remains consumed.
    ReserveBlocked,
}

/// Retained upgrade evidence, separate from creation evidence.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct Status {
    /// Original immutable approval.
    pub approval: Option<Approval>,
    /// Committed phase.
    pub phase: Phase,
    /// Upload and upgrade dispatch counts, each at most one.
    pub attempts: [u8; 2],
    /// Hashes of exact encoded effect arguments.
    pub requests: [Option<[u8; 32]>; 2],
    /// Child version observed before dispatch; completion requires exactly one increment.
    pub previous_version: Option<u64>,
    /// Replacement version established by management observation.
    pub observed_version: Option<u64>,
    /// Failure does not grant retry authority.
    pub failure: Option<Failure>,
}

impl Stage {
    /// Check the exact retained child, artifact and finite interval before admission.
    /// # Errors
    /// Refuses another child/owner, unchanged code, invalid bytes or an invalid interval.
    pub fn validate(
        &self,
        manager: &str,
        child: &str,
        previous: &[u8; 32],
        now: u64,
    ) -> Result<(), Refusal> {
        let a = &self.approval;
        if a.manager != manager
            || a.child != child
            || a.previous_module != *previous
            || a.module_hash == a.previous_module
        {
            return Err(Refusal::Binding);
        }
        if self.wasm.len() > MAX_WASM
            || !self.wasm.starts_with(b"\0asm\x01\0\0\0")
            || digest(&self.wasm) != a.module_hash
        {
            return Err(Refusal::Artifact);
        }
        if a.execute_at_ns < now
            || a.execute_at_ns.saturating_sub(now) > 3_600_000_000_000
            || a.expires_at_ns <= a.execute_at_ns
            || a.expires_at_ns.saturating_sub(now) > 3_600_000_000_000
        {
            return Err(Refusal::Time);
        }
        Ok(())
    }
}
