//! Convert an authenticated terminal response into a non-serializable rejection witness.

use crate::fleet_ensure::{
    model::capacity_import::{
        CapacityImportHandoffRequestRecord, CapacityImportPlanRecord,
        rejection::CapacityImportHandoffRejectionRecord,
    },
    ops::capacity_import::{
        journal::CapacityImportJournalError,
        transport::{CompletedHandoff, completed},
    },
};
use candid::Principal;
use ic_agent::{Certificate, agent::RequestStatusResponse};
use sha2_host::{Digest, Sha256};

/// Authenticated outcome of the exact issued controller request.
pub enum HandoffOutcome {
    Completed(CompletedHandoff),
    Rejected(RejectedHandoff),
}

/// Only certified transport can produce this witness; persistence owns its durable receipt.
pub struct RejectedHandoff {
    pub(in crate::fleet_ensure::ops::capacity_import) receipt: CapacityImportHandoffRejectionRecord,
    plan_sha256: [u8; 32],
    canister_id: Principal,
}

impl RejectedHandoff {
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
    if let RequestStatusResponse::Rejected(rejection) = status {
        let mut bytes = Vec::new();
        ciborium::ser::into_writer(certificate, &mut bytes)
            .map_err(|_| CapacityImportJournalError::RequestInvalid)?;
        Ok(HandoffOutcome::Rejected(RejectedHandoff {
            receipt: CapacityImportHandoffRejectionRecord {
                request: request.clone(),
                reject_code: rejection.reject_code as u8,
                reject_message_sha256: Sha256::digest(rejection.reject_message.as_bytes()).into(),
                certificate_sha256: Sha256::digest(&bytes).into(),
            },
            plan_sha256: plan.plan_sha256,
            canister_id,
        }))
    } else {
        completed(plan, canister_id, request, status).map(HandoffOutcome::Completed)
    }
}
