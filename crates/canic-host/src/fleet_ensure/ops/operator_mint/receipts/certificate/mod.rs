//! Module: fleet_ensure::ops::operator_mint::receipts::certificate
//!
//! Responsibility: project shared bounded ledger evidence into Canic mint validation.
//! Boundary: IC Query owns decoding/authentication; Canic retains spending authority.

use crate::fleet_ensure::ops::operator_mint::receipts::{
    DepositVerificationError as Error, ReceiptVerificationLimits,
};
use ic_platform::{
    agent::Agent, icrc_ledger_types::ICRC3DataCertificate,
    principal::ExternalPrincipal as Principal,
};
use ic_query::certificate::{self, CertificateEvidenceLimits};

use ic_host_tools::ledger_evidence::LedgerTip;
pub use ic_query::certificate::CertificateVerificationError;

pub(super) fn verify(
    agent: &Agent,
    ledger: Principal,
    evidence: &ICRC3DataCertificate,
    limits: ReceiptVerificationLimits,
) -> Result<LedgerTip, Error> {
    certificate::authenticate_ledger_tip(
        agent,
        ledger,
        &evidence.certificate,
        &evidence.hash_tree,
        CertificateEvidenceLimits {
            bytes: limits.certificate_bytes,
        },
    )
    .map(|tip| LedgerTip {
        index: tip.index,
        hash: tip.hash,
    })
    .map_err(|error| match error {
        CertificateVerificationError::BudgetExceeded => Error::BudgetExceeded,
        CertificateVerificationError::WitnessMismatch
        | CertificateVerificationError::LedgerTipMalformed => Error::WitnessMismatch,
        other => Error::Certificate(other),
    })
}
