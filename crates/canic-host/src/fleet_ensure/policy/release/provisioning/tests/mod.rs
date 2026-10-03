//! Terminal history, retained deliveries and owner-qualified pointers are independent observations.

use super::*;
use crate::fleet_ensure::view::release::provisioning::{
    ReleaseProvisioningIdentity, ReleaseProvisioningOwner,
};
use candid::Principal;

fn operation(state: ReleaseProvisioningState) -> ReleaseProvisioningFacts {
    ReleaseProvisioningFacts {
        identity: ReleaseProvisioningIdentity {
            owner: ReleaseProvisioningOwner::Provisioning,
            operation_id: [1; 32],
        },
        plan_hash: [2; 32],
        state,
        delivery_in_flight: None,
    }
}

#[test]
fn every_stage_preserves_uncertain_delivery_before_terminal_classification() {
    for (state, expected) in [
        (
            ReleaseProvisioningState::Accepted,
            ReleaseProvisioningDisposition::OwnerReconciliation,
        ),
        (
            ReleaseProvisioningState::Provisioned,
            ReleaseProvisioningDisposition::OwnerReconciliation,
        ),
        (
            ReleaseProvisioningState::Publishing,
            ReleaseProvisioningDisposition::OwnerReconciliation,
        ),
        (
            ReleaseProvisioningState::Published,
            ReleaseProvisioningDisposition::OwnerReconciliation,
        ),
        (
            ReleaseProvisioningState::Activating,
            ReleaseProvisioningDisposition::OwnerReconciliation,
        ),
        (
            ReleaseProvisioningState::RuntimesActive,
            ReleaseProvisioningDisposition::RecordedCompletion,
        ),
        (
            ReleaseProvisioningState::DirectoryPlanned,
            ReleaseProvisioningDisposition::OwnerReconciliation,
        ),
        (
            ReleaseProvisioningState::DirectorySynchronizing,
            ReleaseProvisioningDisposition::OwnerReconciliation,
        ),
        (
            ReleaseProvisioningState::DirectorySynchronized,
            ReleaseProvisioningDisposition::RecordedCompletion,
        ),
    ] {
        let mut facts = operation(state);
        assert_eq!(disposition(&facts), expected);
        let recipient = Principal::from_slice(&[3]);
        facts.delivery_in_flight = Some(recipient);
        assert_eq!(
            disposition(&facts),
            ReleaseProvisioningDisposition::DeliveryReconciliation { recipient }
        );
    }
}

#[test]
fn stale_active_pointer_never_revives_a_terminal_row_or_matches_another_owner() {
    let completed = operation(ReleaseProvisioningState::RuntimesActive);
    let directory = ReleaseProvisioningIdentity {
        owner: ReleaseProvisioningOwner::DirectorySynchronization,
        operation_id: completed.identity.operation_id,
    };
    let assessed = assess_provisioning(ReleaseRootProvisioningFacts {
        root: Principal::from_slice(&[4]),
        active: [completed.identity, directory].into(),
        operations: vec![completed.clone()],
    });
    assert_eq!(assessed.unmatched_active, [directory].into());
    assert_eq!(assessed.operations[0].facts, completed);
    assert_eq!(
        assessed.operations[0].disposition,
        ReleaseProvisioningDisposition::RecordedCompletion
    );
}

#[test]
fn empty_journal_preserves_missing_active_identities() {
    let identity = operation(ReleaseProvisioningState::Accepted).identity;
    for active in [std::collections::BTreeSet::new(), [identity].into()] {
        let facts = ReleaseRootProvisioningFacts {
            root: Principal::from_slice(&[4]),
            active: active.clone(),
            operations: vec![],
        };
        let assessed = assess_provisioning(facts);
        assert!(assessed.operations.is_empty());
        assert_eq!(assessed.unmatched_active, active);
    }
}
