//! Signed Host collection retains pool history without pretending the wire fixture owns effects.

mod multiple;

use super::*;
use crate::fleet_ensure::ops::release::{observation::ReleaseObservationError, pool};
use crate::fleet_ensure::{
    view::release::pool::{
        FleetReleasePoolAssessment, ReleasePoolCreationDisposition, ReleasePoolImportDisposition,
    },
    workflow::release::assess_pools,
};
use canic_control_plane::dto::root::RootPoolReleaseResponse;
use ic_testkit::pocket_ic::PocketIc;

pub(super) fn assert_census(
    pic: &PocketIc,
    agent: &Agent,
    runtime: &tokio::runtime::Runtime,
    review: &FleetReleaseReviewRecord,
    registry: &FleetRegistry,
) {
    let entry = &registry.fleet_subnet_roots[0];
    let root = entry.fleet_subnet_root;
    let status = pool::tests::fixture(root, *entry.placement_subnet.as_principal());
    let replace = |bytes| {
        pic.update_call(root, review.authority.operator, "replace", bytes)
            .unwrap();
    };
    let collect = |review, registry| {
        runtime
            .block_on(pool::collect_with_agent(agent, review, registry))
            .map(assess_pools)
    };
    replace(pool::tests::wire(status.clone()));
    let balances = review
        .sources
        .iter()
        .map(|s| pic.cycle_balance(s.binding.canister_id))
        .collect::<Vec<_>>();
    let observed = collect(review, registry).unwrap();
    assert_assessment(&observed, &status);
    assert_eq!(collect(review, registry).unwrap(), observed);
    assert_eq!(
        review
            .sources
            .iter()
            .map(|s| pic.cycle_balance(s.binding.canister_id))
            .collect::<Vec<_>>(),
        balances
    );
    multiple::assert_complete(pic, agent, runtime, review, registry, status.clone());

    let mut wrong = review.clone();
    wrong.authority.network_root_key_sha256 = [0; 32];
    assert!(matches!(
        collect(&wrong, registry),
        Err(pool::ReleasePoolError::Authentication(
            ReleaseObservationError::Authority
        ))
    ));
    let mut wrong = review.clone();
    wrong.authority.operator = Principal::anonymous();
    assert!(matches!(
        collect(&wrong, registry),
        Err(pool::ReleasePoolError::Authentication(
            ReleaseObservationError::Authority
        ))
    ));
    let mut wrong = review.clone();
    wrong.sources[1].binding.module_sha256 = Some([0; 32]);
    assert!(matches!(
        collect(&wrong, registry),
        Err(pool::ReleasePoolError::Inventory(
            ReleaseInventoryError::Evidence(FleetReleaseError::Custody { .. })
        ))
    ));
    let mut wrong = registry.clone();
    wrong.revision += 1;
    assert!(matches!(
        collect(review, &wrong),
        Err(pool::ReleasePoolError::Inventory(
            ReleaseInventoryError::Evidence(FleetReleaseError::Authority)
        ))
    ));

    let mut wrong = status;
    wrong.root = Principal::anonymous();
    replace(pool::tests::wire(wrong));
    assert!(matches!(
        collect(review, registry),
        Err(pool::ReleasePoolError::Observation {
            stage: pool::ReleasePoolStage::Binding,
            ..
        })
    ));
    replace(b"DIDL".to_vec());
    assert!(matches!(
        collect(review, registry),
        Err(pool::ReleasePoolError::Observation {
            stage: pool::ReleasePoolStage::Decode,
            ..
        })
    ));
}

fn assert_assessment(observed: &FleetReleasePoolAssessment, status: &RootPoolReleaseResponse) {
    assert_eq!(
        observed.evidence.roots.as_slice(),
        std::slice::from_ref(status)
    );
    let assessment = &observed.roots[0];
    assert_eq!(
        assessment.import,
        Some(ReleasePoolImportDisposition::ImportRecovery)
    );
    assert!(
        assessment
            .facts
            .import
            .as_ref()
            .unwrap()
            .call_budget_exhausted
    );
    assert_eq!(
        assessment.creation,
        Some(ReleasePoolCreationDisposition::LedgerReconciliation)
    );
    assert_eq!(
        assessment.facts.handoff.as_ref().unwrap().recipient,
        status.handoff.as_ref().unwrap().recipient
    );
}
