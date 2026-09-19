//! Module: fleet_ensure::ops::operator_mint::receipts::icp
//!
//! Responsibility: prepare exact replicated reads and authenticate ICP transfer evidence.
//! Boundary: no submission, archive calls, journal mutation or funding credit.

mod protobuf;
#[cfg(test)]
mod tests;
mod wire;

use crate::fleet_ensure::{
    model::operator_mint::OperatorMintIntentRecord,
    ops::operator_mint::{
        MINT_MEMO, OperatorMintWireError, account_identifier, principal_subaccount,
        receipts::{
            CertificateVerificationError, certificate,
            icp::wire::{EncodedBlocksReply, ReadArgs},
            network_identity_sha256,
        },
        validate_intent,
    },
};
use candid::Principal;
use ic_agent::{Agent, AgentError, RequestId, agent::signed::SignedUpdate};
use ic_certification::LookupResult;
use prost::Message;
use serde_bytes::ByteBuf;
use sha2_host::{Digest, Sha256};
use thiserror::Error;

pub use wire::ArchiveReadError;

/// Receipt-reader budgets; transport capture must obey the same byte boundary.
#[derive(Clone, Copy, Debug)]
pub struct IcpReadLimits {
    /// Maximum outer certificate bytes before CBOR decoding.
    pub certificate_bytes: usize,
    /// Maximum CBOR nesting during certificate decoding.
    pub certificate_depth: u8,
    /// Maximum certified reply bytes before Candid or protobuf decoding.
    pub reply_bytes: usize,
    /// Work quota passed to the Candid decoder.
    pub decoding_quota: usize,
    /// Work quota for skipped Candid fields.
    pub skipping_quota: usize,
}

/// Typed ICP receipt failures owned by ops; none authorizes another payment.
#[derive(Debug, Error)]
pub enum IcpReadError {
    #[error(transparent)]
    Intent(#[from] OperatorMintWireError),
    #[error("receipt reader differs from the reviewed network or signer")]
    ReaderMismatch,
    #[error("ICP receipt read could not be signed")]
    Prepare(#[source] AgentError),
    #[error("ICP receipt read arguments could not be encoded")]
    Arguments(#[source] candid::Error),
    #[error(transparent)]
    Certificate(#[from] CertificateVerificationError),
    #[error("certificate does not contain a replied result for this exact receipt read")]
    ReplyUnavailable,
    #[error("ICP receipt reply exceeds its byte budget")]
    BudgetExceeded,
    #[error("ICP receipt reply decoding failed or exceeded its work quota")]
    InvalidReply,
    #[error("ICP archive callback differs from the exact supported block read")]
    InvalidArchive,
    #[error(transparent)]
    Archive(#[from] ArchiveReadError),
    #[error("ICP receipt does not contain exactly the requested block")]
    BlockMismatch,
    #[error("ICP block has an unsupported or noncanonical protobuf schema")]
    UnsupportedBlock,
    #[error("ICP transfer differs from the reviewed account, amount, fee, memo or timestamp")]
    TransferMismatch,
}

/// Prepared read authority, constructed only from the reviewed intent and Agent.
/// Signing is local; submission requires the workflow's bounded observation owner.
#[derive(Debug)]
pub struct IcpTransferRead {
    intent: OperatorMintIntentRecord,
    block_index: u64,
    request: SignedUpdate,
    source: ReadSource,
}

#[derive(Debug)]
enum ReadSource {
    Ledger,
    Archive,
}

impl IcpTransferRead {
    /// Exact Ledger or authenticated archive selected for this read.
    #[must_use]
    pub const fn canister(&self) -> Principal {
        self.request.canister_id
    }
    /// Signed Ledger or archive read update envelope; never a transfer request.
    #[must_use]
    pub fn signed_request(&self) -> &[u8] {
        &self.request.signed_update
    }

    /// Exact request identity to poll for the consensus-certified read response.
    #[must_use]
    pub const fn request_id(&self) -> &RequestId {
        &self.request.request_id
    }
}

/// Authenticated ICP transfer evidence; only certificate verification constructs it.
/// It cannot be deserialized or independently admit a cycle credit.
#[derive(Debug)]
pub struct VerifiedIcpTransfer {
    intent: OperatorMintIntentRecord,
    block_index: u64,
    block_sha256: [u8; 32],
}

impl VerifiedIcpTransfer {
    /// Exact intent whose transfer fields were authenticated.
    #[must_use]
    pub const fn intent(&self) -> &OperatorMintIntentRecord {
        &self.intent
    }

    /// Authenticated Ledger-local ICP transfer index.
    #[must_use]
    pub const fn block_index(&self) -> u64 {
        self.block_index
    }

    /// SHA-256 of the exact encoded block authenticated by the read response.
    #[must_use]
    pub const fn block_sha256(&self) -> &[u8; 32] {
        &self.block_sha256
    }
}

/// Authenticated read result; an archive authorization alone proves no transfer.
#[derive(Debug)]
pub enum IcpReadOutcome {
    /// Original transfer fields have been authenticated and checked.
    Transfer(VerifiedIcpTransfer),
    /// The Ledger authorizes one exact archive read for the original block.
    Archive(IcpArchiveAuthorization),
}

/// Private, non-deserializable authority derived only from a certified Ledger reply.
#[derive(Debug)]
pub struct IcpArchiveAuthorization {
    intent: OperatorMintIntentRecord,
    block_index: u64,
    canister: Principal,
}

impl IcpArchiveAuthorization {
    /// Archive selected by the authenticated Ledger response.
    #[must_use]
    pub const fn canister(&self) -> Principal {
        self.canister
    }
}

/// Sign a one-block replicated read, without submitting it or modifying intent.
///
/// ICP's tip certificate does not authenticate its reported height. Replicated
/// `query_encoded_blocks` instead binds the index and reply to this request ID.
pub fn prepare_read(
    agent: &Agent,
    intent: &OperatorMintIntentRecord,
    block_index: u64,
) -> Result<IcpTransferRead, IcpReadError> {
    validate_intent(intent)?;
    sign_read(
        agent,
        intent,
        block_index,
        intent.authority.icp_ledger,
        ReadSource::Ledger,
    )
}

/// Sign only the archive callback authorized by a verified Ledger read.
/// The archive cannot redirect again or change the original transfer identity.
pub fn prepare_archive_read(
    agent: &Agent,
    archive: &IcpArchiveAuthorization,
) -> Result<IcpTransferRead, IcpReadError> {
    sign_read(
        agent,
        &archive.intent,
        archive.block_index,
        archive.canister,
        ReadSource::Archive,
    )
}

fn sign_read(
    agent: &Agent,
    intent: &OperatorMintIntentRecord,
    block_index: u64,
    canister: Principal,
    source: ReadSource,
) -> Result<IcpTransferRead, IcpReadError> {
    verify_reader(agent, intent)?;
    let argument = candid::encode_one(ReadArgs {
        start: block_index,
        length: 1,
    })
    .map_err(IcpReadError::Arguments)?;
    let method = match source {
        ReadSource::Ledger => "query_encoded_blocks",
        ReadSource::Archive => "get_encoded_blocks",
    };
    let request = agent
        .update(&canister, method)
        .with_arg(argument)
        .sign()
        .map_err(IcpReadError::Prepare)?;
    if request.sender != intent.authority.operator {
        return Err(IcpReadError::ReaderMismatch);
    }
    Ok(IcpTransferRead {
        intent: intent.clone(),
        block_index,
        request,
        source,
    })
}

/// Authenticate an exact read response and validate its original encoded transfer.
///
/// Ordinary query replies, uncertified heights, pending/rejected requests and
/// archive authorizations cannot construct a verified transfer through this path.
pub fn verify_read(
    agent: &Agent,
    read: &IcpTransferRead,
    certificate_bytes: &[u8],
    limits: IcpReadLimits,
) -> Result<IcpReadOutcome, IcpReadError> {
    verify_reader(agent, &read.intent)?;
    let certificate = certificate::authenticate(
        agent,
        read.request.canister_id,
        certificate_bytes,
        limits.certificate_bytes,
        limits.certificate_depth,
    )?;
    verify_reader(agent, &read.intent)?;
    let path = [
        b"request_status".as_slice(),
        read.request_id().as_slice(),
        b"status",
    ];
    if certificate.tree.lookup_path(path) != LookupResult::Found(b"replied") {
        return Err(IcpReadError::ReplyUnavailable);
    }
    let path = [
        b"request_status".as_slice(),
        read.request_id().as_slice(),
        b"reply",
    ];
    let LookupResult::Found(reply) = certificate.tree.lookup_path(path) else {
        return Err(IcpReadError::ReplyUnavailable);
    };
    if reply.len() > limits.reply_bytes {
        return Err(IcpReadError::BudgetExceeded);
    }
    let mut config = candid::de::DecoderConfig::new();
    config.set_decoding_quota(limits.decoding_quota);
    config.set_skipping_quota(limits.skipping_quota);
    match read.source {
        ReadSource::Ledger => verify_ledger_reply(read, reply, &config),
        ReadSource::Archive => {
            let blocks: Result<Vec<ByteBuf>, ArchiveReadError> =
                candid::utils::decode_one_with_config(reply, &config)
                    .map_err(|_| IcpReadError::InvalidReply)?;
            let blocks = blocks?;
            if blocks.len() != 1 {
                return Err(IcpReadError::BlockMismatch);
            }
            verified_transfer(read, &blocks[0])
        }
    }
}

fn verify_ledger_reply(
    read: &IcpTransferRead,
    bytes: &[u8],
    config: &candid::de::DecoderConfig,
) -> Result<IcpReadOutcome, IcpReadError> {
    let reply: EncodedBlocksReply = candid::utils::decode_one_with_config(bytes, config)
        .map_err(|_| IcpReadError::InvalidReply)?;
    if reply.chain_length <= read.block_index {
        return Err(IcpReadError::BlockMismatch);
    }
    if !reply.archived_blocks.is_empty() {
        let [archive] = reply.archived_blocks.as_slice() else {
            return Err(IcpReadError::InvalidArchive);
        };
        let exact_range = archive.start == read.block_index && archive.length == 1;
        let canister = archive.callback.canister_id;
        let valid_canister = canister != Principal::anonymous()
            && canister != Principal::management_canister()
            && canister != read.intent.authority.icp_ledger;
        let exact_callback = valid_canister && archive.callback.method == "get_encoded_blocks";
        if !reply.blocks.is_empty() || !exact_range || !exact_callback {
            return Err(IcpReadError::InvalidArchive);
        }
        // first_block_index is unspecified when the Ledger returns no local blocks.
        return Ok(IcpReadOutcome::Archive(IcpArchiveAuthorization {
            intent: read.intent.clone(),
            block_index: read.block_index,
            canister,
        }));
    }
    if reply.first_block_index != read.block_index || reply.blocks.len() != 1 {
        return Err(IcpReadError::BlockMismatch);
    }
    verified_transfer(read, &reply.blocks[0])
}

fn verified_transfer(
    read: &IcpTransferRead,
    encoded: &[u8],
) -> Result<IcpReadOutcome, IcpReadError> {
    verify_block(encoded, &read.intent, read.block_index)?;
    Ok(IcpReadOutcome::Transfer(VerifiedIcpTransfer {
        intent: read.intent.clone(),
        block_index: read.block_index,
        block_sha256: Sha256::digest(encoded).into(),
    }))
}

fn verify_reader(agent: &Agent, intent: &OperatorMintIntentRecord) -> Result<(), IcpReadError> {
    if network_identity_sha256(&agent.read_root_key()) != intent.authority.network_identity_sha256
        || agent.get_principal().ok() != Some(intent.authority.operator)
    {
        return Err(IcpReadError::ReaderMismatch);
    }
    Ok(())
}

// Compare transfer authority as one exact value; no default fee, account or time.
#[derive(Eq, PartialEq)]
struct TransferFields {
    from: [u8; 32],
    to: [u8; 32],
    amount: u64,
    fee: u64,
    memo: u64,
    created_at_time: u64,
}

fn verify_block(
    bytes: &[u8],
    intent: &OperatorMintIntentRecord,
    index: u64,
) -> Result<(), IcpReadError> {
    let block = protobuf::Block::decode(bytes).map_err(|_| IcpReadError::UnsupportedBlock)?;
    // Upstream emits canonical prost bytes. Round-trip equality rejects unknown
    // fields, duplicate singular values, extensions and noncanonical encodings.
    if block.encode_to_vec() != bytes || block.timestamp.is_none() {
        return Err(IcpReadError::UnsupportedBlock);
    }
    match (&block.parent_hash, index) {
        (None, 0) => {}
        (Some(hash), 1..) if hash.value.len() == 32 => {}
        _ => return Err(IcpReadError::UnsupportedBlock),
    }
    let transaction = block.transaction.ok_or(IcpReadError::UnsupportedBlock)?;
    let send = transaction.send.ok_or(IcpReadError::UnsupportedBlock)?;
    let actual = TransferFields {
        from: send
            .from
            .ok_or(IcpReadError::UnsupportedBlock)?
            .value
            .try_into()
            .map_err(|_| IcpReadError::TransferMismatch)?,
        to: send
            .to
            .ok_or(IcpReadError::UnsupportedBlock)?
            .value
            .try_into()
            .map_err(|_| IcpReadError::TransferMismatch)?,
        amount: send.amount.ok_or(IcpReadError::UnsupportedBlock)?.value,
        fee: send.fee.ok_or(IcpReadError::UnsupportedBlock)?.value,
        memo: transaction
            .memo
            .ok_or(IcpReadError::UnsupportedBlock)?
            .value,
        created_at_time: transaction
            .created_at_time
            .ok_or(IcpReadError::UnsupportedBlock)?
            .value,
    };
    let expected = TransferFields {
        from: account_identifier(intent.authority.operator, &[0; 32]),
        to: account_identifier(
            intent.authority.cmc,
            &principal_subaccount(intent.authority.operator),
        ),
        amount: intent.amount_e8s,
        fee: intent.transfer_fee_e8s,
        memo: MINT_MEMO,
        created_at_time: intent.created_at_time_ns,
    };
    if actual != expected {
        return Err(IcpReadError::TransferMismatch);
    }
    Ok(())
}
