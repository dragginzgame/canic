//! Signed dual-owner receipt collection refuses a late Root failure without returning partial evidence.

use super::*;
use crate::fleet_ensure::{
    ops::release::{
        observation::ReleaseObservationError,
        receipts::{self, ReleaseReceiptsError as Failure, ReleaseReceiptsStage as Stage},
    },
    view::release::receipts::ReleaseReplayDisposition,
    workflow::release::assess_receipts,
};
use ic_testkit::pocket_ic::PocketIc;

pub(super) fn assert_census(
    pic: &PocketIc,
    agent: &Agent,
    runtime: &tokio::runtime::Runtime,
    review: &FleetReleaseReviewRecord,
    registry: &FleetRegistry,
) {
    let coordinator = review.authority.coordinator;
    let root = registry.fleet_subnet_roots[0].fleet_subnet_root;
    let coordinator_page = receipts::tests::fixture(coordinator);
    let root_page = receipts::tests::fixture(root);
    pic.update_call(
        coordinator,
        review.authority.operator,
        "replace_funding",
        receipts::tests::wire(coordinator_page.clone()),
    )
    .unwrap();
    let replace = |page| {
        pic.update_call(
            root,
            review.authority.operator,
            "replace",
            receipts::tests::wire(page),
        )
        .unwrap()
    };
    replace(root_page.clone());
    let collect = |review, registry| {
        runtime
            .block_on(receipts::collect_with_agent(agent, review, registry))
            .map(assess_receipts)
    };
    let balances = [pic.cycle_balance(coordinator), pic.cycle_balance(root)];
    let observed = collect(review, registry).unwrap();
    assert_eq!(
        observed.evidence.owners,
        [
            (coordinator, vec![coordinator_page]),
            (root, vec![root_page.clone()])
        ]
        .into()
    );
    for owner in [coordinator, root] {
        assert_eq!(
            observed.owners[&owner][0].disposition,
            ReleaseReplayDisposition::AccountingRecovery
        );
        assert_eq!(observed.owners[&owner][0].pending_intents, vec![6]);
        assert!(observed.owners[&owner][0].missing_intents.is_empty());
    }
    assert_eq!(collect(review, registry).unwrap(), observed);
    assert_eq!(
        [pic.cycle_balance(coordinator), pic.cycle_balance(root)],
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
    let mut wrong_registry = registry.clone();
    wrong_registry.revision += 1;
    assert!(matches!(
        collect(review, &wrong_registry),
        Err(Failure::Inventory(_))
    ));
    let mut wrong = root_page.clone();
    wrong.owner = coordinator;
    replace(wrong);
    assert!(
        matches!(collect(review, registry), Err(Failure::Observation { owner, stage: Stage::Binding }) if owner == root)
    );
    let mut repeated = root_page.clone();
    repeated.next_after = Some(repeated.entry.as_ref().unwrap().slot);
    replace(repeated);
    assert!(
        matches!(collect(review, registry), Err(Failure::Observation { owner, stage: Stage::Pagination }) if owner == root)
    );
    pic.update_call(root, review.authority.operator, "replace", b"DIDL".to_vec())
        .unwrap();
    assert!(
        matches!(collect(review, registry), Err(Failure::Observation { owner, stage: Stage::Decode }) if owner == root)
    );
    let mut empty = root_page;
    empty.entry = None;
    replace(empty.clone());
    let assessed = collect(review, registry).unwrap();
    assert_eq!(assessed.evidence.owners[&root], vec![empty]);
    assert!(assessed.owners[&root].is_empty());
}
