//! Module: fleet_ensure::ops::operator_mint::receipts::certificate
//!
//! Responsibility: project shared bounded ledger evidence into Canic mint validation.
//! Boundary: IC Query owns decoding/authentication; Canic retains spending authority.

use crate::fleet_ensure::ops::operator_mint::receipts::{
    DepositVerificationError as Error, ReceiptVerificationLimits,
};
use candid::Principal;
use ic_agent::{Agent, Certificate};
use ic_query::certificate::{self, CertificateEvidenceLimits};
use icrc_ledger_types::icrc3::blocks::ICRC3DataCertificate;

pub use ic_query::certificate::CertificateVerificationError;
use ic_query::certificate::CertifiedLedgerTip;

pub(super) fn verify(
    agent: &Agent,
    ledger: Principal,
    evidence: &ICRC3DataCertificate,
    limits: ReceiptVerificationLimits,
) -> Result<CertifiedLedgerTip, Error> {
    certificate::authenticate_ledger_tip(
        agent,
        ledger,
        &evidence.certificate,
        &evidence.hash_tree,
        CertificateEvidenceLimits {
            bytes: limits.certificate_bytes,
            depth: limits.depth,
        },
    )
    .map_err(|error| match error {
        CertificateVerificationError::BudgetExceeded => Error::BudgetExceeded,
        CertificateVerificationError::WitnessMismatch
        | CertificateVerificationError::LedgerTipMalformed => Error::WitnessMismatch,
        other => Error::Certificate(other),
    })
}

pub(super) fn authenticate(
    agent: &Agent,
    canister: Principal,
    bytes: &[u8],
    maximum_bytes: usize,
    depth: u8,
) -> Result<Certificate, CertificateVerificationError> {
    certificate::authenticate(agent, canister, bytes, maximum_bytes, depth)
}
