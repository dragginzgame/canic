//! Finite snapshot executor exchange. Backup owns procedure admission and host evidence.
//! Platform owns inner DTOs/codecs; the manager owns original effect/result custody.
use crate::digest;
pub use ic_platform_protocol::management_snapshots as management;
use serde::{Deserialize, Serialize};
#[cfg(test)]
mod tests;

/// Execute one original slot; no implicit resubmission.
pub const EXECUTE: u64 = 80;
/// Authenticated update lookup; no inner effects.
pub const LOOKUP: u64 = 81;
/// Qualified protocol/authority generation. No rotation or rollback is admitted.
pub const GENERATION: u64 = 1;
/// Full one-MiB extent plus Candid framing.
pub const MAX_INNER: usize = 1_048_576 + 16_384;
/// Finite lifetime slots, across both operations.
pub const MAX_STEPS: usize = 64;
/// Reserved result bytes, never refunded by lookup or short replies.
pub const MAX_RETAINED: usize = 16 * 1_048_576;
/// Maximum cycles attached to one original inner call.
pub const MAX_CYCLES: u64 = 1_000_000_000_000;
/// Request contains typed arguments and the exact encoded-byte commitment.
pub const LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: MAX_INNER + 16_384,
    // Portable DTO Vec serialization visits borrowed elements. Wire accounts
    // their native pointer sizes conservatively; this is not a live-heap size.
    allocation: 136 * MAX_INNER,
    // Platform's portable DTOs use ordinary Vec<u8>, including upload chunks.
    // Wire's serializer charges four visited nodes per ordinary byte element.
    nodes: 5_000_000,
    depth: 24,
};

/// Capture and restore identities never share a slot.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Kind {
    /// Export a stopped application's snapshot.
    Capture,
    /// Upload and load; never start or activate the application.
    Restore,
}
/// Fixed procedure positions, not arbitrary method dispatch.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Step {
    /// Explicit stop, if the controller requests it before Backup's procedure.
    Stop,
    /// Original stopped-state/controller observation.
    InitialStatus,
    /// Snapshot inventory before allocation.
    List,
    /// Create the original capture snapshot.
    Take,
    /// Read original or destination metadata.
    Metadata,
    /// Read a capture extent or verify an uploaded extent.
    Read(u32),
    /// Allocate a non-replacing restore snapshot.
    Allocate,
    /// Upload one extent.
    Upload(u32),
    /// Recheck before destructive load.
    BeforeLoad,
    /// Load the original verified destination.
    Load,
    /// Final stopped-state observation.
    FinalStatus,
}
/// Immutable values under (kind, operation ID), not additional namespaces.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Operation {
    /// Backup's original 16-byte ID.
    pub id: [u8; 16],
    /// Fixed procedure.
    pub kind: Kind,
    /// Exact Backup approval digest.
    pub approval: [u8; 32],
    /// Host-selected network/trust commitment; the manager cannot discover a trust root.
    pub network: [u8; 32],
    /// Exact manager principal text.
    pub manager: String,
    /// Original authenticated host signer.
    pub caller: String,
    /// Surviving manager authority generation.
    pub generation: u64,
    /// Existing managed child, never the manager or Store.
    pub target: String,
}
/// Selected Platform arguments. This is not a copied management DTO.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Effect {
    /// Stop, status or inventory, selected by the fixed step.
    Canister(management::CanisterArgs),
    /// Non-replacing, non-uninstalling snapshot creation.
    Take(management::TakeSnapshotArgs),
    /// Metadata read.
    Metadata(management::SnapshotArgs),
    /// One bounded data read.
    Read(management::ReadDataArgs),
    /// Non-replacing metadata upload.
    Allocate(management::UploadMetadataArgs),
    /// One bounded data upload.
    Upload(management::UploadDataArgs),
    /// Load without starting.
    Load(management::LoadSnapshotArgs),
}
/// Exact inner effect and its finite liabilities.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Request {
    /// Immutable operation binding.
    pub operation: Operation,
    /// Fixed original slot.
    pub step: Step,
    /// Typed external arguments for independent manager admission.
    pub effect: Effect,
    /// SHA-256 of the original external bytes approved by Backup.
    pub arguments_hash: [u8; 32],
    /// Exact original external byte count.
    pub arguments_bytes: u32,
    /// Absolute original dispatch deadline; lookup does not renew it.
    pub expires_at_ns: u64,
    /// Finite attachment, not an estimate of target snapshot storage costs.
    pub cycles: u64,
    /// Original upper bound for system call cost plus attachment. Never refunded.
    pub cycle_limit: u64,
    /// Maximum retained inner reply/rejection bytes, reserved before dispatch.
    pub reply_bytes: u32,
}
/// Exact values echoed by execute and lookup for host verification.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Binding {
    /// Full original operation.
    pub operation: Operation,
    /// Original slot.
    pub step: Step,
    /// Domain-separated digest of the full bounded request.
    pub request_hash: [u8; 32],
    /// Original inner method.
    pub method: String,
    /// Original argument byte identity.
    pub arguments_hash: [u8; 32],
    /// Original argument length.
    pub arguments_bytes: u32,
    /// Original deadline.
    pub expires_at_ns: u64,
    /// Original attachment reservation.
    pub cycles: u64,
    /// Original maximum cost plus attachment liability.
    pub cycle_limit: u64,
    /// Original result reservation.
    pub reply_bytes: u32,
}
/// Original inner outcome. An outer rejection is not this enum's Rejected case.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Outcome {
    /// Issued or unretained outcome; never redispatch.
    Pending,
    /// Exact bounded management reply bytes.
    Reply(#[serde(with = "serde_bytes")] Vec<u8>),
    /// Exact inner rejection code and text.
    Rejected(u32, String),
    /// Locally refused before issuance; consumed slot, no retry authority.
    Refused,
}
/// Durable manager statement, authenticated by the outer ingress certificate.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Record {
    /// Complete original commitment.
    pub binding: Binding,
    /// Original inner outcome.
    pub outcome: Outcome,
}
/// Update whose outer response was authenticated by the host.
#[derive(Clone, Copy, Debug)]
pub enum Exchange {
    /// Response to the original execute ingress.
    Execute,
    /// Response to the separately authorized observation ingress.
    Lookup,
}

/// A response that cannot supply original inner evidence.
#[derive(Debug)]
pub enum EvidenceError {
    /// Wire rejected the response framing or payload.
    Wire(ic_wire::Error),
    /// The returned record differs from the expected original slot.
    Binding,
    /// Retained bytes exceed the original reservation.
    Size,
}

impl Exchange {
    /// Decode the payload of an already authenticated successful outer ingress.
    /// The caller authenticates the manager, request ID and certificate first.
    /// The inner Result distinguishes manager admission errors from retained records.
    /// # Errors
    /// Rejects wrong frames, changed commitments and over-budget evidence.
    pub fn decode_response(
        self,
        expected: &Binding,
        bytes: &[u8],
    ) -> Result<Result<Record, Error>, EvidenceError> {
        let method = match self {
            Self::Execute => EXECUTE,
            Self::Lookup => LOOKUP,
        };
        let result: Result<Record, Error> = ic_wire::decode(
            bytes,
            ic_wire::Encoding::Frame {
                method,
                direction: ic_wire::Direction::Response,
            },
            LIMITS,
        )
        .map_err(EvidenceError::Wire)?;
        if let Ok(record) = &result {
            if record.binding != *expected {
                return Err(EvidenceError::Binding);
            }
            if record.outcome.retained_bytes() > expected.reply_bytes as usize {
                return Err(EvidenceError::Size);
            }
        }
        Ok(result)
    }
}

impl Outcome {
    /// Charge exact reply bytes or rejection message bytes to the reserved slot.
    #[must_use]
    pub fn retained_bytes(&self) -> usize {
        match self {
            Self::Reply(bytes) => bytes.len(),
            Self::Rejected(_, message) => message.len(),
            Self::Pending | Self::Refused => 0,
        }
    }
}
/// Admission failure does not grant replacement authority.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Error {
    /// Invalid or inconsistent arguments, method, route or budget.
    Invalid,
    /// Same original identity with changed values.
    Conflict,
    /// No retained record; never proof of no effect.
    Missing,
    /// Finite slot/byte/cycle budget is exhausted.
    Capacity,
    /// Another unresolved effect or management operation owns the target.
    Busy,
    /// The original new-dispatch deadline elapsed.
    Expired,
}
impl Request {
    /// Validate exact Platform encoding and derive the commitment without any effects.
    /// Sender version is preserved here and checked by the manager only for new dispatch.
    /// # Errors
    /// Refuses unknown combinations, changed bytes, replacement, excessive extents or budgets.
    pub fn prepare(&self) -> Result<(Binding, Vec<u8>), Error> {
        let op = &self.operation;
        if op.generation != GENERATION
            || op.id == [0; 16]
            || op.approval == [0; 32]
            || op.network == [0; 32]
            || self.cycle_limit > MAX_CYCLES
            || self.cycles > self.cycle_limit
            || self.reply_bytes == 0
            || self.reply_bytes as usize > MAX_INNER
        {
            return Err(Error::Invalid);
        }
        let id = |v: &[u8]| !v.is_empty() && v.len() <= 256;
        let (method, target, bytes) = match (&self.effect, self.step, op.kind) {
            (Effect::Canister(a), Step::Stop, _) => {
                ("stop_canister", a.canister_id, a.encode(MAX_INNER))
            }
            (Effect::Canister(a), Step::InitialStatus | Step::FinalStatus, _)
            | (Effect::Canister(a), Step::BeforeLoad, Kind::Restore) => {
                ("canister_status", a.canister_id, a.encode(MAX_INNER))
            }
            (Effect::Canister(a), Step::List, _) => (
                "list_canister_snapshots",
                a.canister_id,
                a.encode(MAX_INNER),
            ),
            (Effect::Take(a), Step::Take, Kind::Capture)
                if a.replace_snapshot.is_none() && a.uninstall_code != Some(true) =>
            {
                ("take_canister_snapshot", a.canister_id, a.encode(MAX_INNER))
            }
            (Effect::Metadata(a), Step::Metadata, _) if id(&a.snapshot_id) => (
                "read_canister_snapshot_metadata",
                a.canister_id,
                a.encode(MAX_INNER),
            ),
            (Effect::Read(a), Step::Read(n), _)
                if n < MAX_STEPS as u32 && id(&a.snapshot_id) && read_bound(&a.kind) =>
            {
                (
                    "read_canister_snapshot_data",
                    a.canister_id,
                    a.encode(MAX_INNER),
                )
            }
            (Effect::Allocate(a), Step::Allocate, Kind::Restore)
                if a.replace_snapshot.is_none()
                    && a.certified_data.len() <= 32
                    && a.globals.len() <= 1024 =>
            {
                (
                    "upload_canister_snapshot_metadata",
                    a.canister_id,
                    a.encode(MAX_INNER),
                )
            }
            (Effect::Upload(a), Step::Upload(n), Kind::Restore)
                if n < MAX_STEPS as u32 && id(&a.snapshot_id) && a.chunk.len() <= 1_048_576 =>
            {
                (
                    "upload_canister_snapshot_data",
                    a.canister_id,
                    a.encode(MAX_INNER),
                )
            }
            (Effect::Load(a), Step::Load, Kind::Restore) if id(&a.snapshot_id) => {
                ("load_canister_snapshot", a.canister_id, a.encode(MAX_INNER))
            }
            _ => return Err(Error::Invalid),
        };
        let bytes = bytes.map_err(|_| Error::Invalid)?;
        if target.to_text() != op.target
            || digest(&bytes) != self.arguments_hash
            || bytes.len() != self.arguments_bytes as usize
        {
            return Err(Error::Invalid);
        }
        let encoded = ic_wire::encode(self, ic_wire::Encoding::Payload, LIMITS)
            .map_err(|_| Error::Invalid)?;
        Ok((
            Binding {
                operation: op.clone(),
                step: self.step,
                request_hash: digest(&[b"canic.snapshot.request\0".as_slice(), &encoded].concat()),
                method: method.into(),
                arguments_hash: self.arguments_hash,
                arguments_bytes: self.arguments_bytes,
                expires_at_ns: self.expires_at_ns,
                cycles: self.cycles,
                cycle_limit: self.cycle_limit,
                reply_bytes: self.reply_bytes,
            },
            bytes,
        ))
    }
}
fn read_bound(kind: &management::ReadKind) -> bool {
    match kind {
        management::ReadKind::WasmModule(e)
        | management::ReadKind::WasmMemory(e)
        | management::ReadKind::StableMemory(e) => {
            e.size <= 1_048_576 && e.offset.checked_add(e.size).is_some()
        }
        management::ReadKind::WasmChunk(c) => c.hash.len() == 32,
    }
}
