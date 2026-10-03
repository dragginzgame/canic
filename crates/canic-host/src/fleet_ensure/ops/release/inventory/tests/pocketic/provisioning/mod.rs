//! Signed query collection and whole-result refusals for the provisioning census wire.

use super::*;
use crate::fleet_ensure::ops::release::{
    observation::ReleaseObservationError,
    provisioning::{self, ReleaseProvisioningError as Failure, ReleaseProvisioningStage as Stage},
};
use crate::fleet_ensure::{
    view::release::provisioning::{
        FleetReleaseProvisioningAssessment, ReleaseProvisioningDisposition,
        ReleaseProvisioningIdentity, ReleaseProvisioningOwner,
    },
    workflow::release::assess_provisioning_evidence,
};
use canic_control_plane::dto::root::{
    RootProvisioningReleasePhase, RootProvisioningReleaseResponse,
};
use ic_testkit::pocket_ic::PocketIc;

pub(super) fn assert_census(
    pic: &PocketIc,
    agent: &Agent,
    runtime: &tokio::runtime::Runtime,
    review: &FleetReleaseReviewRecord,
    registry: &FleetRegistry,
) {
    let root = registry.fleet_subnet_roots[0].fleet_subnet_root;
    let page = delivery_fixture(root);
    let replace = |bytes| {
        pic.update_call(root, review.authority.operator, "replace", bytes)
            .unwrap();
    };
    let collect = |review, registry| {
        runtime
            .block_on(provisioning::collect_with_agent(agent, review, registry))
            .map(assess_provisioning_evidence)
    };
    replace(provisioning::tests::wire(page.clone()));
    let balances = review
        .sources
        .iter()
        .map(|source| pic.cycle_balance(source.binding.canister_id))
        .collect::<Vec<_>>();
    let observed = collect(review, registry).unwrap();
    assert_assessment(&observed, &page);
    assert_eq!(collect(review, registry).unwrap(), observed);
    assert_eq!(
        review
            .sources
            .iter()
            .map(|source| pic.cycle_balance(source.binding.canister_id))
            .collect::<Vec<_>>(),
        balances
    );
    let mut wrong = review.clone();
    wrong.authority.operator = Principal::anonymous();
    assert!(matches!(
        collect(&wrong, registry),
        Err(Failure::Authentication(ReleaseObservationError::Authority))
    ));
    let mut wrong = review.clone();
    wrong.authority.network_root_key_sha256 = [0; 32];
    assert!(matches!(
        collect(&wrong, registry),
        Err(Failure::Authentication(ReleaseObservationError::Authority))
    ));
    let mut wrong = registry.clone();
    wrong.revision += 1;
    assert!(matches!(
        collect(review, &wrong),
        Err(Failure::Inventory(_))
    ));
    let mut wrong = page.clone();
    wrong.root = Principal::anonymous();
    replace(provisioning::tests::wire(wrong));
    assert!(matches!(
        collect(review, registry),
        Err(Failure::Observation {
            stage: Stage::Binding,
            ..
        })
    ));
    // The static wire repeats its first page; a collector must reject instead of looping.
    let mut repeated = page.clone();
    repeated.next_after = Some(repeated.entry.as_ref().unwrap().key);
    replace(provisioning::tests::wire(repeated));
    assert!(matches!(
        collect(review, registry),
        Err(Failure::Observation {
            stage: Stage::Pagination,
            ..
        })
    ));
    replace(b"DIDL".to_vec());
    assert!(matches!(
        collect(review, registry),
        Err(Failure::Observation {
            stage: Stage::Decode,
            ..
        })
    ));
    let mut empty = page;
    empty.entry = None;
    replace(provisioning::tests::wire(empty.clone()));
    assert_empty(&collect(review, registry).unwrap(), &empty);
}

fn delivery_fixture(root: Principal) -> RootProvisioningReleaseResponse {
    let mut page = provisioning::tests::fixture(root);
    page.active_provisioning = Some([7; 32]);
    page.active_directory_synchronization = Some([7; 32]);
    let entry = page.entry.as_mut().unwrap();
    entry.phase = RootProvisioningReleasePhase::Publishing;
    entry.delivery_in_flight = Some(Principal::from_slice(&[41]));
    page
}

fn identity(owner: ReleaseProvisioningOwner) -> ReleaseProvisioningIdentity {
    ReleaseProvisioningIdentity {
        owner,
        operation_id: [7; 32],
    }
}

fn assert_assessment(
    observed: &FleetReleaseProvisioningAssessment,
    page: &RootProvisioningReleaseResponse,
) {
    assert_eq!(
        observed.evidence.roots[0].pages.as_slice(),
        std::slice::from_ref(page)
    );
    let root = &observed.roots[0];
    assert_eq!(root.root, page.root);
    assert_eq!(
        root.unmatched_active,
        [identity(ReleaseProvisioningOwner::DirectorySynchronization)].into()
    );
    let [operation] = root.operations.as_slice() else {
        panic!("one observed provisioning record");
    };
    assert_eq!(
        operation.facts.identity,
        identity(ReleaseProvisioningOwner::Provisioning)
    );
    assert_eq!(
        operation.disposition,
        ReleaseProvisioningDisposition::DeliveryReconciliation {
            recipient: Principal::from_slice(&[41])
        }
    );
}

fn assert_empty(
    observed: &FleetReleaseProvisioningAssessment,
    page: &RootProvisioningReleaseResponse,
) {
    assert_eq!(
        observed.evidence.roots[0].pages.as_slice(),
        std::slice::from_ref(page)
    );
    let root = &observed.roots[0];
    assert!(root.operations.is_empty());
    assert_eq!(
        root.unmatched_active,
        [
            identity(ReleaseProvisioningOwner::Provisioning),
            identity(ReleaseProvisioningOwner::DirectorySynchronization)
        ]
        .into()
    );
}
