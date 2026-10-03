//! Interpret provisioning obligations without turning history or bookkeeping drift into blockers.

#[cfg(test)]
mod tests;

use crate::fleet_ensure::view::release::provisioning::{
    ReleaseProvisioningAssessment, ReleaseProvisioningDisposition, ReleaseProvisioningFacts,
    ReleaseProvisioningState, ReleaseRootProvisioningAssessment, ReleaseRootProvisioningFacts,
};

/// Retain original owners and report dangling pointers without declaring release readiness.
pub(in crate::fleet_ensure) fn assess_provisioning(
    facts: ReleaseRootProvisioningFacts,
) -> ReleaseRootProvisioningAssessment {
    let mut unmatched_active = facts.active;
    let operations = facts
        .operations
        .into_iter()
        .map(|facts| {
            unmatched_active.remove(&facts.identity);
            let disposition = disposition(&facts);
            ReleaseProvisioningAssessment { facts, disposition }
        })
        .collect();
    ReleaseRootProvisioningAssessment {
        root: facts.root,
        operations,
        unmatched_active,
    }
}

const fn disposition(facts: &ReleaseProvisioningFacts) -> ReleaseProvisioningDisposition {
    if let Some(recipient) = facts.delivery_in_flight {
        return ReleaseProvisioningDisposition::DeliveryReconciliation { recipient };
    }
    match facts.state {
        ReleaseProvisioningState::RuntimesActive
        | ReleaseProvisioningState::DirectorySynchronized => {
            ReleaseProvisioningDisposition::RecordedCompletion
        }
        ReleaseProvisioningState::Accepted
        | ReleaseProvisioningState::Provisioned
        | ReleaseProvisioningState::Publishing
        | ReleaseProvisioningState::Published
        | ReleaseProvisioningState::Activating
        | ReleaseProvisioningState::DirectoryPlanned
        | ReleaseProvisioningState::DirectorySynchronizing => {
            ReleaseProvisioningDisposition::OwnerReconciliation
        }
    }
}
