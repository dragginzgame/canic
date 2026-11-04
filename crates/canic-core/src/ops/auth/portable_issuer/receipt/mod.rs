//! Compact exact-result registration evidence; fresh Root snapshots own grant authority.

use super::{E, JOURNAL_LIMITS};
use crate::model::auth::portable_issuer::{
    PortableIssuerEnrollmentReceiptRecord, PortableIssuerKeyTicket,
};
use canic_contracts::dto::portable_enrollment::PortableIssuerEnrollment;
use serde::Serialize;
use sha2::{Digest, Sha256};

#[derive(Serialize)]
struct ReceiptInput<'a> {
    ticket: &'a PortableIssuerKeyTicket,
    result: &'a PortableIssuerEnrollment,
}

pub(super) fn from_result(
    ticket: &PortableIssuerKeyTicket,
    result: &PortableIssuerEnrollment,
) -> Result<PortableIssuerEnrollmentReceiptRecord, E> {
    let encoded = ic_wire::native::encode(&ReceiptInput { ticket, result }, JOURNAL_LIMITS)
        .map_err(|_| E::Capacity)?;
    Ok(PortableIssuerEnrollmentReceiptRecord {
        response_hash: Sha256::new()
            .chain_update(b"canic/portable-issuer-registration-result/v1\0")
            .chain_update(encoded)
            .finalize()
            .into(),
        not_before_ns: result.not_before_ns,
        expires_at_ns: result.expires_at_ns,
        deployment_approval_expires_at_ns: result.deployment_approval_expires_at_ns,
    })
}

pub(super) const fn valid_window(receipt: &PortableIssuerEnrollmentReceiptRecord) -> bool {
    receipt.not_before_ns < receipt.expires_at_ns
        && receipt.expires_at_ns <= receipt.deployment_approval_expires_at_ns
}
