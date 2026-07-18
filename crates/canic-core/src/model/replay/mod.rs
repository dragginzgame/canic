//! Module: model::replay
//!
//! Responsibility: define pure shared replay receipt identifiers and state.
//! Does not own: storage mutation, replay reservation, or command execution.
//! Boundary: consumed by replay ops and stable replay storage records.
use crate::{
    cdk::types::Principal,
    ids::{CanisterRole, IntentId},
};
use std::{fmt, str::FromStr};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const REPLAY_RECEIPT_SCHEMA_VERSION: u32 = 1;
pub const REPLAY_PAYLOAD_HASH_SCHEMA_VERSION: u32 = 1;
pub const PLACEMENT_CHILD_REPLAY_COMMAND_KIND: &str = "root.allocate_placement_child";
pub const ROOT_PROVISION_REPLAY_COMMAND_KIND: &str = "root.provision";

const REPLAY_PAYLOAD_HASH_DOMAIN: &[u8] = b"canic-replay-payload-hash:v1";

///
/// OperationId
///
/// Stable operation identifier shared by replay-protected commands.
/// Owned by the replay model and serialized into stable replay receipts.
///

#[derive(Clone, Copy, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct OperationId([u8; 32]);

impl OperationId {
    /// Build an operation id from its canonical 32-byte representation.
    #[must_use]
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Return the canonical 32-byte operation id representation.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Consume the operation id and return its canonical bytes.
    #[must_use]
    pub const fn into_bytes(self) -> [u8; 32] {
        self.0
    }
}

impl fmt::Debug for OperationId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "OperationId({self})")
    }
}

impl fmt::Display for OperationId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in self.0 {
            write!(f, "{byte:02x}")?;
        }
        Ok(())
    }
}

impl From<[u8; 32]> for OperationId {
    fn from(value: [u8; 32]) -> Self {
        Self::from_bytes(value)
    }
}

impl From<OperationId> for [u8; 32] {
    fn from(value: OperationId) -> Self {
        value.into_bytes()
    }
}

impl TryFrom<&[u8]> for OperationId {
    type Error = OperationIdParseError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        let bytes: [u8; 32] =
            value
                .try_into()
                .map_err(|_| OperationIdParseError::InvalidByteLength {
                    actual: value.len(),
                })?;
        Ok(Self(bytes))
    }
}

impl FromStr for OperationId {
    type Err = OperationIdParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if value.len() != 64 {
            return Err(OperationIdParseError::InvalidHexLength {
                actual: value.len(),
            });
        }

        let mut bytes = [0u8; 32];
        for (index, chunk) in value.as_bytes().chunks_exact(2).enumerate() {
            let high = decode_hex_nibble(chunk[0])?;
            let low = decode_hex_nibble(chunk[1])?;
            bytes[index] = (high << 4) | low;
        }
        Ok(Self(bytes))
    }
}

///
/// OperationIdParseError
///
/// Typed parse failure for operation id byte and hex conversions.
/// Owned by the replay model and returned by `OperationId` constructors.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OperationIdParseError {
    InvalidByteLength { actual: usize },
    InvalidHexLength { actual: usize },
    InvalidHexCharacter { byte: u8 },
}

///
/// CommandKind
///
/// Validated replay command namespace.
/// Owned by the replay model and used to partition replay receipts.
///

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct CommandKind(String);

impl CommandKind {
    /// Validate and create a replay command namespace.
    pub fn new(value: impl Into<String>) -> Result<Self, CommandKindError> {
        let value = value.into();
        if value.is_empty() {
            return Err(CommandKindError::Empty);
        }
        if !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-' | b':'))
        {
            return Err(CommandKindError::InvalidCharacter);
        }
        Ok(Self(value))
    }

    /// Return the validated command namespace as text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

///
/// CommandKindError
///
/// Typed validation failure for replay command namespaces.
/// Owned by the replay model and returned by `CommandKind::new`.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CommandKindError {
    Empty,
    InvalidCharacter,
}

///
/// AuthKind
///
/// Authentication class bound into replay actor identity.
/// Owned by the replay model and included in payload hashes.
///

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum AuthKind {
    DirectCaller,
    DelegatedToken,
    RoleAttestation,
}

///
/// ReplayActor
///
/// Effective actor identity bound to replay receipts and payload hashes.
/// Owned by the replay model and consumed by replay guards.
///

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ReplayActor {
    pub effective_principal: Principal,
    pub auth_kind: AuthKind,
}

impl ReplayActor {
    /// Build a replay actor for direct caller authentication.
    #[must_use]
    pub const fn direct_caller(caller: Principal) -> Self {
        Self {
            effective_principal: caller,
            auth_kind: AuthKind::DirectCaller,
        }
    }
}

///
/// ReplayReceipt
///
/// Canonical replay receipt state independent of stable-memory encoding.
/// Owned by the replay model and persisted through storage record adapters.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ReplayReceipt {
    pub schema_version: u32,
    pub command_kind: CommandKind,
    pub operation_id: OperationId,
    pub actor: ReplayActor,
    pub payload_hash_schema_version: u32,
    pub payload_hash: [u8; 32],
    pub status: ReplayReceiptStatus,
    pub created_at_ns: u64,
    pub updated_at_ns: u64,
    pub expires_at_ns: Option<u64>,
    pub response_schema_version: Option<u32>,
    pub response_bytes: Option<Vec<u8>>,
    pub staged_response_schema_version: Option<u32>,
    pub staged_response_bytes: Option<Vec<u8>>,
    pub cost_guard_settlement: Option<ReplayCostGuardSettlement>,
    pub effect: Option<ExternalEffectDescriptor>,
}

///
/// ReplayCostGuardSettlement
///
/// Durable cost-intent identity required to finish accounting without repeating
/// an external effect.
/// Owned by the replay model and persisted with the protected replay receipt.
///

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ReplayCostGuardSettlement {
    pub quota_intent_id: IntentId,
    pub reservation_intent_id: IntentId,
}

///
/// ReplayReceiptStatus
///
/// Lifecycle state for a replay receipt.
/// Owned by the replay model and interpreted by replay guards.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ReplayReceiptStatus {
    Reserved,
    ExternalEffectInFlight,
    Committed,
    RecoveryRequired { reason: RecoveryReason },
}

///
/// RecoveryReason
///
/// Stable replay recovery reason after uncertain external effects.
/// Owned by the replay model and exposed through replay decisions.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum RecoveryReason {
    ExternalEffectStatusUnknown,
    ResponseCommitFailed,
    CostSettlementFailed,
    StateProjectionFailed,
    Other(String),
}

///
/// ExternalEffectDescriptor
///
/// Replay-visible description of an external side effect boundary.
/// Owned by the replay model and persisted while effects are in flight.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ExternalEffectDescriptor {
    ManagementCreateCanister { command_kind: CommandKind },
    ManagementCall { canister: Principal, method: String },
    IcpTransfer { operation_id: OperationId },
}

/// Whether a committed placement-child receipt must remain until its caller acknowledges it.
#[must_use]
pub fn placement_receipt_requires_acknowledgement(
    status: &ReplayReceiptStatus,
    effect: Option<&ExternalEffectDescriptor>,
) -> bool {
    *status == ReplayReceiptStatus::Committed
        && matches!(
            effect,
            Some(ExternalEffectDescriptor::ManagementCreateCanister { command_kind })
                if command_kind.as_str() == PLACEMENT_CHILD_REPLAY_COMMAND_KIND
        )
}

///
/// ReplayPayloadHasher
///
/// Deterministic hasher for replay command payloads.
/// Owned by the replay model and used by workflow replay adapters.
///

pub struct ReplayPayloadHasher {
    inner: Sha256,
}

impl ReplayPayloadHasher {
    /// Start a deterministic replay payload hash for a command and actor.
    #[must_use]
    pub fn new(command_kind: &CommandKind, actor: &ReplayActor) -> Self {
        let mut inner = Sha256::new();
        hash_bytes(&mut inner, REPLAY_PAYLOAD_HASH_DOMAIN);
        hash_u32(&mut inner, REPLAY_PAYLOAD_HASH_SCHEMA_VERSION);
        hash_str(&mut inner, command_kind.as_str());
        hash_replay_actor(&mut inner, actor);
        Self { inner }
    }

    /// Add a boolean field to the replay payload hash.
    pub fn hash_bool(&mut self, value: bool) {
        hash_bool(&mut self.inner, value);
    }

    /// Add a `u64` field to the replay payload hash.
    pub fn hash_u64(&mut self, value: u64) {
        hash_u64(&mut self.inner, value);
    }

    /// Add byte string data to the replay payload hash.
    pub fn hash_bytes(&mut self, value: &[u8]) {
        hash_bytes(&mut self.inner, value);
    }

    /// Add UTF-8 string data to the replay payload hash.
    pub fn hash_str(&mut self, value: &str) {
        hash_str(&mut self.inner, value);
    }

    /// Add a principal to the replay payload hash.
    pub fn hash_principal(&mut self, value: &Principal) {
        hash_principal(&mut self.inner, value);
    }

    /// Add a canister role to the replay payload hash.
    pub fn hash_role(&mut self, value: &CanisterRole) {
        hash_str(&mut self.inner, value.as_str());
    }

    /// Finish and return the canonical replay payload hash.
    #[must_use]
    pub fn finish(self) -> [u8; 32] {
        self.inner.finalize().into()
    }
}

const fn decode_hex_nibble(byte: u8) -> Result<u8, OperationIdParseError> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        b'A'..=b'F' => Ok(byte - b'A' + 10),
        _ => Err(OperationIdParseError::InvalidHexCharacter { byte }),
    }
}

fn hash_replay_actor(hasher: &mut Sha256, actor: &ReplayActor) {
    hash_principal(hasher, &actor.effective_principal);
    hash_str(
        hasher,
        match actor.auth_kind {
            AuthKind::DirectCaller => "DirectCaller",
            AuthKind::DelegatedToken => "DelegatedToken",
            AuthKind::RoleAttestation => "RoleAttestation",
        },
    );
    // Preserve the retired actor-extension marker in the payload-hash layout.
    hash_bool(hasher, false);
}

fn hash_bool(hasher: &mut Sha256, value: bool) {
    hasher.update([u8::from(value)]);
}

fn hash_u32(hasher: &mut Sha256, value: u32) {
    hasher.update(value.to_be_bytes());
}

fn hash_u64(hasher: &mut Sha256, value: u64) {
    hasher.update(value.to_be_bytes());
}

fn hash_bytes(hasher: &mut Sha256, bytes: &[u8]) {
    hasher.update((bytes.len() as u64).to_be_bytes());
    hasher.update(bytes);
}

fn hash_str(hasher: &mut Sha256, value: &str) {
    hash_bytes(hasher, value.as_bytes());
}

fn hash_principal(hasher: &mut Sha256, principal: &Principal) {
    hash_bytes(hasher, principal.as_slice());
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn p(id: u8) -> Principal {
        Principal::from_slice(&[id; 29])
    }

    #[test]
    fn operation_id_is_exactly_32_bytes_and_hex_round_trips() {
        let raw = [0xabu8; 32];
        let id = OperationId::from_bytes(raw);
        let text = id.to_string();

        assert_eq!(text.len(), 64);
        assert_eq!(text.parse::<OperationId>().expect("hex parses"), id);
        assert_eq!(OperationId::try_from(&raw[..]).expect("bytes parse"), id);
    }

    #[test]
    fn operation_id_rejects_wrong_widths() {
        assert!(matches!(
            OperationId::try_from(&[1u8; 31][..]),
            Err(OperationIdParseError::InvalidByteLength { actual: 31 })
        ));
        assert!(matches!(
            "aa".parse::<OperationId>(),
            Err(OperationIdParseError::InvalidHexLength { actual: 2 })
        ));
    }

    #[test]
    fn operation_id_rejects_invalid_hex() {
        let text = format!("{}zz", "00".repeat(31));

        assert!(matches!(
            text.parse::<OperationId>(),
            Err(OperationIdParseError::InvalidHexCharacter { byte: b'z' })
        ));
    }

    #[test]
    fn command_kind_rejects_empty_and_space_values() {
        assert_eq!(CommandKind::new(""), Err(CommandKindError::Empty));
        assert_eq!(
            CommandKind::new("pool create"),
            Err(CommandKindError::InvalidCharacter)
        );
        assert_eq!(
            CommandKind::new("pool.create_empty.v1")
                .expect("kind")
                .as_str(),
            "pool.create_empty.v1"
        );
    }

    #[test]
    fn payload_hash_binds_command_kind_actor_and_payload() {
        let command = CommandKind::new("proof.issue.v1").expect("kind");
        let actor = ReplayActor::direct_caller(p(1));

        let mut first = ReplayPayloadHasher::new(&command, &actor);
        first.hash_str("payload");
        let first = first.finish();

        let mut changed_payload = ReplayPayloadHasher::new(&command, &actor);
        changed_payload.hash_str("other");
        assert_ne!(first, changed_payload.finish());

        let other_command = CommandKind::new("proof.issue.v2").expect("kind");
        let mut changed_command = ReplayPayloadHasher::new(&other_command, &actor);
        changed_command.hash_str("payload");
        assert_ne!(first, changed_command.finish());

        let other_actor = ReplayActor::direct_caller(p(2));
        let mut changed_actor = ReplayPayloadHasher::new(&command, &other_actor);
        changed_actor.hash_str("payload");
        assert_ne!(first, changed_actor.finish());
    }
}
