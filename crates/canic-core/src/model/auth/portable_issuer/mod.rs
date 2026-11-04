//! Public issuer preparation history; secret signing material has no record shape.

use canic_contracts::{
    dto::portable_enrollment::{
        PortableEnrollmentSnapshot, PortableIssuerChallenge, PreparePortableIssuerKey,
    },
    dto::portable_issuance::{IssuePortableCredential, IssuedPortableCredential},
    ids::FleetKey,
};
use ic_platform::Principal;
use serde::{Deserialize, Serialize};

/// Installed issuer domain, captured before an external effect.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PortableIssuerDomain {
    pub authority: Principal,
    pub fleet: FleetKey,
    pub issuer: Principal,
}

/// Bounded public journal; restoring it never restores a signer.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PortableIssuerJournalRecord {
    #[serde(deserialize_with = "canic_contracts::serialization::required_option")]
    pub domain: Option<PortableIssuerDomain>,
    pub boot: u64,
    pub issuance_epoch: u64,
    #[serde(deserialize_with = "canic_contracts::serialization::required_option")]
    pub active_sequence: Option<u64>,
    pub keys: Vec<PortableIssuerKeyRecord>,
    #[serde(deserialize_with = "canic_contracts::serialization::required_option")]
    pub enrollment: Option<PortableEnrollmentSnapshot>,
    pub issuances: Vec<PortableIssuanceRecord>,
    pub logins: Vec<PortableIssuerLoginRecord>,
}

/// Exact public intent and original Root result; no seed or signature is retained.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PortableIssuerKeyRecord {
    pub request: PreparePortableIssuerKey,
    pub boot: u64,
    #[serde(deserialize_with = "canic_contracts::serialization::required_option")]
    pub public_key: Option<[u8; 32]>,
    #[serde(deserialize_with = "canic_contracts::serialization::required_option")]
    pub challenge: Option<PortableIssuerChallenge>,
    #[serde(deserialize_with = "canic_contracts::serialization::required_option")]
    pub enrollment: Option<PortableIssuerEnrollmentReceiptRecord>,
}

/// Exact Root result commitment and original activation window; grants stay with Root authority.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PortableIssuerEnrollmentReceiptRecord {
    pub response_hash: [u8; 32],
    pub not_before_ns: u64,
    pub expires_at_ns: u64,
    pub deployment_approval_expires_at_ns: u64,
}

/// Callback ticket binds one preparation to its original lifecycle and domain.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PortableIssuerKeyTicket {
    pub domain: PortableIssuerDomain,
    pub boot: u64,
    pub sequence: u64,
    pub operation_id: [u8; 32],
}

/// Key preparation either reuses the current key or awaits fresh randomness.
pub enum PortableIssuerKeyPreparation {
    Pending(PortableIssuerKeyTicket),
    Ready(canic_contracts::dto::portable_enrollment::PortableIssuerPreparedKey),
}

/// Application-selected issuance limits; approval of every requested grant is separate.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PortableIssuancePolicy {
    pub login_ttl_ns: u64,
    pub proof_ttl_ns: u64,
    pub receipt_timeout_ns: u64,
    pub reconciliation_retention_ns: u64,
}

/// Immutable live browser login, retained independently of individual credentials.
#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct PortableIssuerLoginBinding {
    pub caller: Principal,
    pub epoch: u64,
    pub login_id: [u8; 32],
}

/// One original login window and browser key under its account/epoch binding.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PortableIssuerLoginRecord {
    pub binding: PortableIssuerLoginBinding,
    pub browser_public_key: [u8; 32],
    pub started_at_ns: u64,
    pub deadline_ns: u64,
}

/// Exact authenticated account request and original public result across interruption.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PortableIssuanceRecord {
    pub caller: Principal,
    pub request: IssuePortableCredential,
    pub policy: PortableIssuancePolicy,
    pub key: PortableIssuerKeyTicket,
    pub login_started_at_ns: u64,
    pub login_deadline_ns: u64,
    pub issued_at_ns: u64,
    pub admission_deadline_ns: u64,
    pub receipt_deadline_ns: u64,
    pub remove_at_ns: u64,
    #[serde(deserialize_with = "canic_contracts::serialization::required_option")]
    pub challenge: Option<[u8; 32]>,
    #[serde(deserialize_with = "canic_contracts::serialization::required_option")]
    pub result: Option<IssuedPortableCredential>,
}

/// Lookup key plus the original key/lifecycle binding for an async issuance operation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PortableIssuanceTicket {
    pub caller: Principal,
    pub epoch: u64,
    pub operation_id: [u8; 32],
    pub key: PortableIssuerKeyTicket,
}

/// Completed retries return original bytes; pending intent alone may fetch authority.
pub enum PortableIssuancePreparation {
    Pending(PortableIssuanceTicket),
    Ready(IssuedPortableCredential),
}
