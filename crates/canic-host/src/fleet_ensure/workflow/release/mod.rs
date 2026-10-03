//! Combine authenticated operation-owner evidence with pure release assessment.
//!
//! No release effect, producer fence, reconciliation call or reset authority is issued here.

use crate::{
    fleet_ensure::{
        model::release::FleetReleaseReviewRecord,
        ops::release::{
            funding::{self, ReleaseFundingError},
            pool::{self, ReleasePoolError},
            provisioning::{self, ReleaseProvisioningError},
            receipts::{self, ReleaseReceiptsError},
        },
        policy::release::{
            funding::assess_refill, pool::assess_pool, provisioning::assess_provisioning,
            receipts::assess_receipt,
        },
        view::release::{
            FleetReleaseFundingView, FleetReleasePoolView, FleetReleaseProvisioningView,
            FleetReleaseReceiptsView,
            funding::{
                FleetReleaseFundingAssessment, ReleaseRefillAssessment,
                ReleaseRootFundingAssessment,
            },
            pool::FleetReleasePoolAssessment,
            provisioning::FleetReleaseProvisioningAssessment,
            receipts::FleetReleaseReplayAssessment,
        },
    },
    icp::IcpCli,
};
use canic_core::dto::fleet_registry::FleetRegistry;

/// Observe provisioning journals without requiring completion of a disposable installation.
pub async fn observe_provisioning(
    icp: &IcpCli,
    review: &FleetReleaseReviewRecord,
    registry: &FleetRegistry,
) -> Result<FleetReleaseProvisioningAssessment, ReleaseProvisioningError> {
    Ok(assess_provisioning_evidence(
        provisioning::collect(icp, review, registry).await?,
    ))
}

pub(in crate::fleet_ensure) fn assess_provisioning_evidence(
    evidence: FleetReleaseProvisioningView,
) -> FleetReleaseProvisioningAssessment {
    let roots = evidence
        .roots
        .iter()
        .map(|root| assess_provisioning(provisioning::assessment_facts(root)))
        .collect();
    FleetReleaseProvisioningAssessment { evidence, roots }
}

/// Observe original pool obligations without granting replacement spending or reset authority.
pub async fn observe_pool(
    icp: &IcpCli,
    review: &FleetReleaseReviewRecord,
    registry: &FleetRegistry,
) -> Result<FleetReleasePoolAssessment, ReleasePoolError> {
    Ok(assess_pools(pool::collect(icp, review, registry).await?))
}

pub(in crate::fleet_ensure) fn assess_pools(
    evidence: FleetReleasePoolView,
) -> FleetReleasePoolAssessment {
    let roots = evidence
        .roots
        .iter()
        .map(|root| assess_pool(pool::assessment_facts(root)))
        .collect();
    FleetReleasePoolAssessment { evidence, roots }
}

/// Observe retained replay work without treating local accounting as proof of payment outcome.
pub async fn observe_receipts(
    icp: &IcpCli,
    review: &FleetReleaseReviewRecord,
    registry: &FleetRegistry,
) -> Result<FleetReleaseReplayAssessment, ReleaseReceiptsError> {
    Ok(assess_receipts(
        receipts::collect(icp, review, registry).await?,
    ))
}

pub(in crate::fleet_ensure) fn assess_receipts(
    evidence: FleetReleaseReceiptsView,
) -> FleetReleaseReplayAssessment {
    let owners = evidence
        .owners
        .iter()
        .map(|(owner, pages)| {
            let assessments = pages
                .iter()
                .filter_map(|page| page.entry.as_ref())
                .map(|entry| assess_receipt(&receipts::assessment_facts(entry)))
                .collect();
            (*owner, assessments)
        })
        .collect();
    FleetReleaseReplayAssessment { evidence, owners }
}

/// Observe exact Root funding history and describe what each retained effect still needs.
///
/// A recorded receipt never substitutes for account balances, recovery artifacts or quiescence.
pub async fn observe_funding(
    icp: &IcpCli,
    review: &FleetReleaseReviewRecord,
    registry: &FleetRegistry,
) -> Result<FleetReleaseFundingAssessment, ReleaseFundingError> {
    assess_funding(funding::collect(icp, review, registry).await?)
}

pub(in crate::fleet_ensure) fn assess_funding(
    evidence: FleetReleaseFundingView,
) -> Result<FleetReleaseFundingAssessment, ReleaseFundingError> {
    let roots = funding::assessment_facts(&evidence)?
        .into_iter()
        .map(|root| ReleaseRootFundingAssessment {
            root: root.root,
            coordinator_operations: root.coordinator_operations,
            rotation_operation: root.rotation_operation,
            refills: root
                .refills
                .into_iter()
                .map(|facts| ReleaseRefillAssessment {
                    record_id: facts.record_id,
                    operation_id: facts.operation_id,
                    disposition: assess_refill(&facts),
                })
                .collect(),
        })
        .collect();
    Ok(FleetReleaseFundingAssessment {
        coordinator_rotation_operation: evidence
            .coordinator
            .rotation
            .as_ref()
            .map(|rotation| rotation.operation_id),
        evidence,
        roots,
    })
}
