//! Module: fleet_ensure::ops::operator_mint::receipts::certificate
//!
//! Responsibility: authenticate Ledger certificates and the Cycles Ledger chain tip.
//! Boundary: upstream Agent owns signatures, delegation ranges and certificate time.

use crate::fleet_ensure::ops::operator_mint::receipts::{
    DepositVerificationError as Error, ReceiptVerificationLimits,
};
use candid::Principal;
use ciborium::Value;
use ic_agent::{Agent, AgentError};
use ic_certification::{
    Certificate, Delegation, HashTree, LookupResult, empty, fork, labeled, leaf as tree_leaf,
    pruned,
};
use icrc_ledger_types::icrc3::blocks::ICRC3DataCertificate;
use serde::{Deserialize, de::DeserializeOwned};
use serde_bytes::ByteBuf;
use thiserror::Error;

/// Shared receipt-certificate failures; no failure establishes payment outcome.
#[derive(Debug, Error)]
pub enum CertificateVerificationError {
    #[error("receipt certificate exceeds its byte budget")]
    BudgetExceeded,
    #[error("receipt certificate or witness encoding is invalid")]
    Malformed,
    #[error("receipt certificate CBOR decoding failed: {0}")]
    Decode(#[source] ciborium::de::Error<std::io::Error>),
    #[error("receipt certificate authentication failed")]
    Invalid(#[source] AgentError),
}

/// Authenticated chain anchor; scoped to the receipt verifier.
pub(super) struct CertifiedTip {
    pub index: u128,
    pub hash: [u8; 32],
}

pub(super) fn verify(
    agent: &Agent,
    ledger: Principal,
    evidence: &ICRC3DataCertificate,
    limits: ReceiptVerificationLimits,
) -> Result<CertifiedTip, Error> {
    if evidence
        .certificate
        .len()
        .checked_add(evidence.hash_tree.len())
        .is_none_or(|bytes| bytes > limits.certificate_bytes)
    {
        return Err(Error::BudgetExceeded);
    }
    let certificate = authenticate(
        agent,
        ledger,
        &evidence.certificate,
        limits.certificate_bytes,
        limits.depth,
    )?;
    let witness = decode::<OwnedTree>(&evidence.hash_tree, limits.depth)?.0;
    let certified_data = leaf(
        &certificate.tree,
        &[b"canister", ledger.as_slice(), b"certified_data"],
    )?;
    if certified_data != witness.digest() {
        return Err(Error::WitnessMismatch);
    }
    let hash = leaf(&witness, &[b"last_block_hash"])?
        .try_into()
        .map_err(|_| Error::WitnessMismatch)?;
    let index = decode_index(leaf(&witness, &[b"last_block_index"])?)?;
    Ok(CertifiedTip { index, hash })
}

pub(super) fn authenticate(
    agent: &Agent,
    canister: Principal,
    bytes: &[u8],
    maximum_bytes: usize,
    depth: u8,
) -> Result<Certificate, CertificateVerificationError> {
    if bytes.len() > maximum_bytes {
        return Err(CertificateVerificationError::BudgetExceeded);
    }
    let wire: CertificateWire = decode(bytes, depth)?;
    let certificate = Certificate {
        tree: wire.tree.0,
        signature: wire.signature.into_vec(),
        delegation: wire.delegation,
    };
    agent
        .verify(&certificate, canister)
        .map_err(CertificateVerificationError::Invalid)?;
    Ok(certificate)
}

// ic-certification's pruned-node deserializer requires borrowed bytes, which a
// streaming CBOR reader cannot provide. Decode the bounded owned tree and map its
// exact IC tags without weakening the Agent's signature/delegation verification.
struct OwnedTree(HashTree);

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CertificateWire {
    tree: OwnedTree,
    signature: ByteBuf,
    #[serde(default)]
    delegation: Option<Delegation>,
}

impl<'de> Deserialize<'de> for OwnedTree {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        owned_tree(Value::deserialize(deserializer)?)
            .map(Self)
            .ok_or_else(|| serde::de::Error::custom("invalid IC hash tree"))
    }
}

fn owned_tree(value: Value) -> Option<HashTree> {
    let Value::Array(values) = value else {
        return None;
    };
    let mut values = values.into_iter();
    let Value::Integer(tag) = values.next()? else {
        return None;
    };
    let bytes = |value| {
        if let Value::Bytes(bytes) = value {
            Some(bytes)
        } else {
            None
        }
    };
    let tree = match u8::try_from(tag).ok()? {
        0 => empty(),
        1 => fork(owned_tree(values.next()?)?, owned_tree(values.next()?)?),
        2 => labeled(bytes(values.next()?)?, owned_tree(values.next()?)?),
        3 => tree_leaf(bytes(values.next()?)?),
        4 => pruned(<[u8; 32]>::try_from(bytes(values.next()?)?).ok()?),
        _ => return None,
    };
    values.next().is_none().then_some(tree)
}

fn decode<T: DeserializeOwned>(
    mut bytes: &[u8],
    depth: u8,
) -> Result<T, CertificateVerificationError> {
    let value = ciborium::de::from_reader_with_recursion_limit(&mut bytes, usize::from(depth))
        .map_err(CertificateVerificationError::Decode)?;
    if !bytes.is_empty() {
        return Err(CertificateVerificationError::Malformed);
    }
    Ok(value)
}

fn leaf<'a>(tree: &'a HashTree, path: &[&[u8]]) -> Result<&'a [u8], Error> {
    match tree.lookup_path(path) {
        LookupResult::Found(bytes) => Ok(bytes),
        _ => Err(Error::WitnessMismatch),
    }
}

// This Ledger certifies the last index as unsigned LEB128, not fixed-width bytes.
fn decode_index(bytes: &[u8]) -> Result<u128, Error> {
    let mut value = 0_u128;
    for (offset, byte) in bytes.iter().copied().enumerate() {
        if offset > 18 || (offset == 18 && byte > 3) {
            return Err(Error::WitnessMismatch);
        }
        value |= u128::from(byte & 0x7f) << (offset * 7);
        if byte < 0x80 {
            if offset + 1 != bytes.len() || (offset > 0 && byte == 0) {
                return Err(Error::WitnessMismatch);
            }
            return Ok(value);
        }
    }
    Err(Error::WitnessMismatch)
}
