//! Classify authenticated terminal status without trusting the host clock or HTTP failures.

use crate::fleet_ensure::{
    model::capacity_import::{
        CapacityImportHandoffRequestRecord, CapacityImportPlanRecord,
        retirement::{
            CapacityImportHandoffRetirementReason, CapacityImportHandoffRetirementRecord,
        },
    },
    ops::capacity_import::{
        journal::CapacityImportJournalError,
        transport::{CompletedHandoff, completed},
    },
    policy::capacity_import::expired_absence_proves_retirement,
};
use candid::Principal;
use ic_agent::{Certificate, agent::RequestStatusResponse};
use sha2_host::{Digest, Sha256};

/// Authenticated outcome of the exact issued controller request.
pub enum HandoffOutcome {
    Completed(CompletedHandoff),
    Retired(RetiredHandoff),
}

/// Only certified transport can produce this witness; persistence owns its durable receipt.
pub struct RetiredHandoff {
    pub(in crate::fleet_ensure::ops::capacity_import) receipt:
        CapacityImportHandoffRetirementRecord,
    plan_sha256: [u8; 32],
    canister_id: Principal,
}

impl RetiredHandoff {
    pub(in crate::fleet_ensure::ops::capacity_import) fn matches(
        &self,
        plan: &CapacityImportPlanRecord,
        canister: Principal,
        request: &CapacityImportHandoffRequestRecord,
    ) -> bool {
        self.plan_sha256 == plan.plan_sha256
            && self.canister_id == canister
            && self.receipt.request == *request
    }
}

// Agent authenticates this certificate for the exact effective canister and request first.
pub(super) fn outcome(
    plan: &CapacityImportPlanRecord,
    canister_id: Principal,
    request: &CapacityImportHandoffRequestRecord,
    status: RequestStatusResponse,
    certificate: &Certificate,
) -> Result<HandoffOutcome, CapacityImportJournalError> {
    let reason = match status {
        RequestStatusResponse::Rejected(rejection) => {
            CapacityImportHandoffRetirementReason::Rejected {
                reject_code: rejection.reject_code as u8,
                reject_message_sha256: Sha256::digest(rejection.reject_message.as_bytes()).into(),
            }
        }
        RequestStatusResponse::Done => CapacityImportHandoffRetirementReason::Done,
        RequestStatusResponse::Unknown => {
            // Agent returns Unknown only for certified absence, never an incomplete witness.
            let certified_at_ns = certified_time(certificate)?;
            if !expired_absence_proves_retirement(request.ingress_expiry, certified_at_ns) {
                return Err(CapacityImportJournalError::Unresolved);
            }
            CapacityImportHandoffRetirementReason::Absent { certified_at_ns }
        }
        _ => return completed(plan, canister_id, request, status).map(HandoffOutcome::Completed),
    };
    let mut bytes = Vec::new();
    ciborium::ser::into_writer(certificate, &mut bytes)
        .map_err(|_| CapacityImportJournalError::RequestInvalid)?;
    Ok(HandoffOutcome::Retired(RetiredHandoff {
        receipt: CapacityImportHandoffRetirementRecord {
            request: request.clone(),
            reason,
            certificate_sha256: Sha256::digest(&bytes).into(),
        },
        plan_sha256: plan.plan_sha256,
        canister_id,
    }))
}

// The IC certifies time as an unsigned LEB128 natural; retain only bounded nanoseconds.
fn certified_time(certificate: &Certificate) -> Result<u64, CapacityImportJournalError> {
    let bytes = ic_agent::lookup_value(certificate, [b"time".as_slice()])
        .map_err(|_| CapacityImportJournalError::Unresolved)?;
    let mut value = 0_u64;
    for (index, byte) in bytes.iter().copied().enumerate().take(10) {
        if index == 9 && byte > 1 {
            break;
        }
        value |= u64::from(byte & 0x7f) << (index * 7);
        if byte & 0x80 == 0 {
            return (index + 1 == bytes.len())
                .then_some(value)
                .ok_or(CapacityImportJournalError::Unresolved);
        }
    }
    Err(CapacityImportJournalError::Unresolved)
}
