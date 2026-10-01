//! Construct and verify immutable capacity reviews before workflow persists or applies them.
//! Hashes bind local review content; authenticated observations remain a transport responsibility.

pub mod admission;
mod destination;
mod evidence;
pub(in crate::fleet_ensure) mod funding;
pub mod journal;
pub mod observation;
pub mod publication;
#[cfg(test)]
mod tests;
pub mod transport;

use crate::fleet_ensure::{
    model::{
        FLEET_ENSURE_SCHEMA_VERSION,
        capacity_import::{
            CapacityImportAuthority, CapacityImportPlanRecord, CapacityImportRootBudget,
            CapacityImportSourceRecord,
        },
    },
    policy::capacity_import::{CapacityImportPolicyError, validate_plan},
};
use sha2_host::{Digest, Sha256};
use thiserror::Error;

pub use destination::{CapacityImportPrerequisiteError, validate_destination_authority};
pub use evidence::validate_root_status;

/// Retained content or policy failed before granting import authority.
#[derive(Debug, Error)]
pub enum CapacityImportReviewError {
    #[error(
        "capacity import admission evidence is incomplete or differs from its reviewed authority"
    )]
    AdmissionInvalid,
    #[error(transparent)]
    Policy(#[from] CapacityImportPolicyError),
    #[error("capacity import plan cannot be encoded")]
    Encode(#[from] serde_json::Error),
    #[error("capacity import plan digest differs from its retained content")]
    DigestMismatch,
    #[error("Root reservation differs from the exact reviewed import")]
    ReservationMismatch,
    #[error("Root import progress or cycle receipts differ from the reviewed operation")]
    RootEvidenceMismatch,
}

/// Build deterministic controller transitions and bind every reviewed source field.
pub fn prepare_review(
    mut authority: CapacityImportAuthority,
    mut sources: Vec<CapacityImportSourceRecord>,
    root_budget: CapacityImportRootBudget,
) -> Result<CapacityImportPlanRecord, CapacityImportReviewError> {
    // Sort representations only. Duplicate input remains visible to policy.
    authority.recovery_controllers.sort_unstable();
    sources.sort_by_key(|source| source.binding.canister_id);
    for source in &mut sources {
        source.binding.controllers.sort_unstable();
    }
    let mut final_controllers = authority.recovery_controllers.clone();
    final_controllers.push(authority.root);
    final_controllers.sort_unstable();
    final_controllers.dedup();
    let mut transitional_controllers = final_controllers.clone();
    transitional_controllers.push(authority.operator);
    transitional_controllers.sort_unstable();
    transitional_controllers.dedup();
    let mut plan = CapacityImportPlanRecord {
        funding_credits: Vec::new(),
        schema_version: FLEET_ENSURE_SCHEMA_VERSION,
        admission: None,
        authority,
        sources,
        transitional_controllers,
        final_controllers,
        root_budget,
        plan_sha256: [0; 32],
    };
    validate_plan(&plan)?;
    plan.plan_sha256 = review_digest(&plan)?;
    Ok(plan)
}

/// Verify both retained content and explicit approval; a digest never bypasses policy.
pub fn verify_review(
    plan: &CapacityImportPlanRecord,
    approved_sha256: [u8; 32],
) -> Result<(), CapacityImportReviewError> {
    if plan.plan_sha256 != approved_sha256 || review_digest(plan)? != approved_sha256 {
        return Err(CapacityImportReviewError::DigestMismatch);
    }
    validate_plan(plan)?;
    admission::validate(plan)?;
    Ok(())
}

/// Bind verified host admission before publishing any review or signed request.
pub fn with_admission(
    mut plan: CapacityImportPlanRecord,
    admission: crate::fleet_ensure::model::capacity_import::admission::CapacityImportAdmissionRecord,
) -> Result<CapacityImportPlanRecord, CapacityImportReviewError> {
    verify_review(&plan, plan.plan_sha256)?;
    if plan.admission.is_some() {
        return Err(CapacityImportReviewError::DigestMismatch);
    }
    plan.admission = Some(admission);
    admission::validate(&plan)?;
    plan.plan_sha256 = review_digest(&plan)?;
    Ok(plan)
}

/// Seal the separately observed credits into the exact approval digest.
pub(in crate::fleet_ensure) fn with_funding(
    mut plan: CapacityImportPlanRecord,
    credits: Vec<
        crate::fleet_ensure::model::capacity_import::funding::CapacityImportFundingCreditRecord,
    >,
) -> Result<CapacityImportPlanRecord, CapacityImportReviewError> {
    verify_review(&plan, plan.plan_sha256)?;
    if !plan.funding_credits.is_empty() {
        return Err(CapacityImportReviewError::DigestMismatch);
    }
    plan.funding_credits = credits;
    plan.funding_credits
        .sort_by_key(|credit| credit.before.binding.canister_id);
    validate_plan(&plan)?;
    plan.plan_sha256 = review_digest(&plan)?;
    Ok(plan)
}

fn review_digest(plan: &CapacityImportPlanRecord) -> Result<[u8; 32], serde_json::Error> {
    let mut content = plan.clone();
    content.plan_sha256 = [0; 32];
    let mut hash = Sha256::new();
    hash.update(b"canic:fleet-capacity-import:review:v1\0");
    hash.update(serde_json::to_vec(&content)?);
    Ok(hash.finalize().into())
}

/// Construct the exact Root reservation from verified retained review content.
pub fn root_reservation(
    plan: &CapacityImportPlanRecord,
) -> Result<canic_core::dto::pool_import::PoolImportReservation, CapacityImportReviewError> {
    verify_review(plan, plan.plan_sha256)?;
    Ok(canic_core::dto::pool_import::PoolImportReservation {
        sequence: plan.authority.import_sequence,
        plan_sha256: plan.plan_sha256,
        root_authority_sha256: plan.authority.root_authority_sha256,
        root: plan.authority.root,
        operator: plan.authority.operator,
        subnet: plan.authority.subnet.into_principal(),
        transitional_controllers: plan.transitional_controllers.clone(),
        final_controllers: plan.final_controllers.clone(),
        sources: plan
            .sources
            .iter()
            .map(|source| canic_core::dto::pool_import::PoolImportSource {
                canister_id: source.binding.canister_id,
                controllers: source.binding.controllers.clone(),
                module_sha256: source.binding.module_sha256,
                canister_version: source.binding.canister_version,
                stopped: source.binding.stopped,
                disposition_sha256:
                    crate::fleet_ensure::policy::capacity_import::disposition_digest(source),
                observed_cycles: source.observed_cycles,
                observed_reserved_cycles: source.observed_reserved_cycles,
                minimum_ready_cycles: source.minimum_ready_cycles,
                maximum_debit_cycles: source.maximum_debit_cycles,
            })
            .collect(),
        observed_root_cycles: plan.root_budget.observed_cycles,
        observed_root_reserved_cycles: plan.root_budget.observed_reserved_cycles,
        minimum_root_cycles: plan.root_budget.minimum_retained_cycles,
        maximum_root_debit_cycles: plan.root_budget.maximum_debit_cycles,
        maximum_paid_calls: plan.root_budget.maximum_paid_calls,
    })
}

/// Admit authenticated Root reservation evidence only when every reviewed bound matches.
/// Authentication of this protected response remains the transport owner's responsibility.
pub fn reservation_evidence(
    plan: &CapacityImportPlanRecord,
    response: &canic_core::dto::pool_import::PoolImportStatus,
) -> Result<
    crate::fleet_ensure::model::capacity_import::CapacityImportReservationRecord,
    CapacityImportReviewError,
> {
    validate_root_status(plan, response)?;
    Ok(
        crate::fleet_ensure::model::capacity_import::CapacityImportReservationRecord {
            plan_sha256: plan.plan_sha256,
            authority: plan.authority.clone(),
            sources: plan
                .sources
                .iter()
                .map(|source| source.binding.canister_id)
                .collect(),
        },
    )
}
