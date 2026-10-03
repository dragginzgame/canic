//! Combine authenticated funding evidence with pure receipt assessment.
//!
//! No release effect, producer fence, reconciliation call or reset authority is issued here.

use crate::{
    fleet_ensure::{
        model::release::FleetReleaseReviewRecord,
        ops::release::funding::{self, ReleaseFundingError},
        policy::release::funding::assess_refill,
        view::release::{
            FleetReleaseFundingView,
            funding::{
                FleetReleaseFundingAssessment, ReleaseRefillAssessment,
                ReleaseRootFundingAssessment,
            },
        },
    },
    icp::IcpCli,
};
use canic_core::dto::fleet_registry::FleetRegistry;

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
