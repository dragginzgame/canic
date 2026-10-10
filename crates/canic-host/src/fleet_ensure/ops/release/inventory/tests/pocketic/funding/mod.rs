//! Reuse the certified ownership fixture to qualify Host funding transport and refusals.

mod assessment;

use super::*;
use crate::fleet_ensure::ops::release::{funding, observation::ReleaseObservationError};
use canic_contracts::dto::{
    fleet_coordinator::CoordinatorFundingStatusResponse,
    root::RootFundingReleaseResponse,
    wire::projection::{
        release_coordinator_funding::Response as CoordinatorReply,
        release_funding::Response as RootReply,
    },
};
use canic_core::shared_support::fleet_funding_policy::fleet_subnet_root_funding_policy_hash;
use ic_testkit::pocket_ic::PocketIc;

pub(super) fn assert_census(
    pic: &PocketIc,
    agent: &Agent,
    runtime: &tokio::runtime::Runtime,
    review: &FleetReleaseReviewRecord,
    registry: &FleetRegistry,
) {
    let coordinator_status = funding::tests::coordinator_status(registry);
    replace_coordinator(pic, review, &coordinator_status);
    let root = registry.fleet_subnet_roots[0].fleet_subnet_root;
    let authority = &registry.fleet_subnet_roots[0].funding;
    let page = RootFundingReleaseResponse {
        fleet_subnet_root: root,
        policy_generation: 1,
        policy_hash: fleet_subnet_root_funding_policy_hash(authority),
        icp_refill_policy: authority.icp_refill.clone(),
        current_request: None,
        accepted_grant: None,
        rotation_current: None,
        icp_refills: vec![],
        next_after: None,
    };
    let encode = root_funding_reply;
    let replace = |bytes| {
        pic.update_call(root, review.authority.operator, "replace", bytes)
            .unwrap();
    };
    replace(encode(page.clone()));
    let balance = pic.cycle_balance(root);
    let coordinator_balance = pic.cycle_balance(review.authority.coordinator);
    let collect =
        |review, registry| runtime.block_on(funding::collect_with_agent(agent, review, registry));
    let result = collect(review, registry).unwrap();
    assert_eq!(result.coordinator, coordinator_status);
    assert_eq!(result.roots.len(), 1);
    assert_eq!(result.roots[0].root, root);
    assert_eq!(result.roots[0].pages.len(), 1);
    assert_eq!(
        encode(result.roots[0].pages[0].clone()),
        encode(page.clone())
    );
    let repeated = collect(review, registry).unwrap();
    assert_eq!(repeated.coordinator, coordinator_status);
    assert_eq!(
        encode(repeated.roots[0].pages[0].clone()),
        encode(page.clone())
    );
    assert_eq!(pic.cycle_balance(root), balance);
    assert_eq!(
        pic.cycle_balance(review.authority.coordinator),
        coordinator_balance
    );
    assessment::assert_assessment(pic, agent, runtime, review, registry, page.clone());

    assert_coordinator_refusals(pic, agent, runtime, review, registry, coordinator_status);

    let mut wrong = review.clone();
    wrong.authority.network_root_key_sha256 = [0; 32];
    assert!(matches!(
        collect(&wrong, registry),
        Err(funding::ReleaseFundingError::Authentication(
            ReleaseObservationError::Authority
        ))
    ));
    let mut wrong = review.clone();
    wrong.sources[1].binding.module_sha256 = Some([0; 32]);
    assert!(matches!(
        collect(&wrong, registry),
        Err(funding::ReleaseFundingError::Inventory(
            ReleaseInventoryError::Evidence(FleetReleaseError::Custody { .. })
        ))
    ));
    let mut wrong_registry = registry.clone();
    wrong_registry.revision += 1;
    assert!(matches!(
        collect(review, &wrong_registry),
        Err(funding::ReleaseFundingError::Inventory(
            ReleaseInventoryError::Evidence(FleetReleaseError::Authority)
        ))
    ));

    let mut wrong = page.clone();
    wrong.policy_hash = [0; 32];
    replace(encode(wrong));
    assert!(matches!(
        collect(review, registry),
        Err(funding::ReleaseFundingError::Observation {
            stage: funding::ReleaseFundingStage::Binding,
            ..
        })
    ));
    let mut wrong = page;
    wrong.next_after = Some(1);
    replace(encode(wrong));
    assert!(matches!(
        collect(review, registry),
        Err(funding::ReleaseFundingError::Observation {
            stage: funding::ReleaseFundingStage::Pagination,
            ..
        })
    ));
    replace(b"DIDL".to_vec());
    assert!(matches!(
        collect(review, registry),
        Err(funding::ReleaseFundingError::Observation {
            stage: funding::ReleaseFundingStage::Decode,
            ..
        })
    ));
}

fn root_funding_reply(page: RootFundingReleaseResponse) -> Vec<u8> {
    candid::encode_one(Ok::<_, Error>(RootReply::FundingRelease(Box::new(page)))).unwrap()
}

fn assert_coordinator_refusals(
    pic: &PocketIc,
    agent: &Agent,
    runtime: &tokio::runtime::Runtime,
    review: &FleetReleaseReviewRecord,
    registry: &FleetRegistry,
    coordinator_status: CoordinatorFundingStatusResponse,
) {
    let collect =
        |review, registry| runtime.block_on(funding::collect_with_agent(agent, review, registry));
    let set_coordinator = |status| replace_coordinator(pic, review, &status);
    for change in [
        |status: &mut CoordinatorFundingStatusResponse| status.roots.clear(),
        |status: &mut CoordinatorFundingStatusResponse| status.coordinator = Principal::anonymous(),
        |status: &mut CoordinatorFundingStatusResponse| status.roots[0].policy_hash = [0; 32],
    ] {
        let mut wrong = coordinator_status.clone();
        change(&mut wrong);
        set_coordinator(wrong);
        assert!(
            matches!(collect(review, registry), Err(funding::ReleaseFundingError::Observation { root: rejected, stage: funding::ReleaseFundingStage::Binding }) if rejected == review.authority.coordinator)
        );
    }
    pic.update_call(
        review.authority.coordinator,
        review.authority.operator,
        "replace_funding",
        b"DIDL".to_vec(),
    )
    .unwrap();
    assert!(
        matches!(collect(review, registry), Err(funding::ReleaseFundingError::Observation { root: rejected, stage: funding::ReleaseFundingStage::Decode }) if rejected == review.authority.coordinator)
    );
    set_coordinator(coordinator_status);
}

fn replace_coordinator(
    pic: &PocketIc,
    review: &FleetReleaseReviewRecord,
    status: &CoordinatorFundingStatusResponse,
) {
    let bytes = candid::encode_one(Ok::<_, Error>(CoordinatorReply::Funding(Box::new(
        status.clone(),
    ))))
    .unwrap();
    pic.update_call(
        review.authority.coordinator,
        review.authority.operator,
        "replace_funding",
        bytes,
    )
    .unwrap();
}
