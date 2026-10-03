//! A later Root refusal never turns earlier evidence into a partial successful census.

use super::*;
use canic_control_plane::dto::root::RootPoolReleaseResponse;

pub(super) fn assert_complete(
    pic: &PocketIc,
    agent: &Agent,
    runtime: &tokio::runtime::Runtime,
    review: &FleetReleaseReviewRecord,
    registry: &FleetRegistry,
    first: RootPoolReleaseResponse,
) {
    let (expanded, expanded_registry, second) = add_root(pic, review, registry);
    let replace_registry = |registry: &FleetRegistry| {
        let bytes = candid::encode_one(Ok::<_, Error>(RegistryReply::Registry(Box::new(
            registry.clone(),
        ))))
        .unwrap();
        pic.update_call(
            review.authority.coordinator,
            review.authority.operator,
            "replace",
            bytes,
        )
        .unwrap();
    };
    replace_registry(&expanded_registry);
    let collect = || {
        runtime
            .block_on(pool::collect_with_agent(
                agent,
                &expanded,
                &expanded_registry,
            ))
            .map(assess_pools)
    };
    let assessed = collect().unwrap();
    assert_eq!(assessed.evidence.roots, [first, second.clone()]);
    assert_eq!(assessed.roots[1].facts.root, second.root);
    assert_eq!(assessed.roots[1].import, None);
    assert_eq!(assessed.roots[1].creation, None);
    assert!(assessed.roots[1].facts.custody_candidates.is_empty());
    pic.update_call(
        second.root,
        review.authority.operator,
        "replace",
        b"DIDL".to_vec(),
    )
    .unwrap();
    assert!(
        matches!(collect(), Err(pool::ReleasePoolError::Observation {
        root, stage: pool::ReleasePoolStage::Decode,
    }) if root == second.root)
    );
    let rejection = Error::from_registered(canic_core::diagnostics::codes::AUTHORITY_UNAVAILABLE);
    let refused = pool::tests::refused(rejection);
    pic.update_call(second.root, review.authority.operator, "replace", refused)
        .unwrap();
    assert!(matches!(collect(), Err(pool::ReleasePoolError::Rejected {
        root, rejection: observed,
    }) if root == second.root && observed == rejection));
    replace_registry(registry);
}

fn add_root(
    pic: &PocketIc,
    review: &FleetReleaseReviewRecord,
    registry: &FleetRegistry,
) -> (
    FleetReleaseReviewRecord,
    FleetRegistry,
    RootPoolReleaseResponse,
) {
    let root = pic.create_canister_with_settings(Some(review.authority.operator), None);
    let store = pic.create_canister_with_settings(Some(review.authority.operator), None);
    let second = RootPoolReleaseResponse {
        root,
        bootstrap: None,
        capacity_import: None,
        creation: None,
        handoff: None,
    };
    let wasm = wire_fixture();
    let mut expanded = review.clone();
    for (template, id, role) in [
        (&review.sources[1], root, FleetReleaseRole::Root),
        (&review.sources[2], store, FleetReleaseRole::Store { root }),
    ] {
        pic.install_canister(
            id,
            wasm.clone(),
            pool::tests::wire(second.clone()),
            Some(review.authority.operator),
        );
        let mut source = template.clone();
        source.binding.canister_id = id;
        source.binding.subnet = SubnetId::from_principal(pic.get_subnet(id).unwrap());
        source.binding.module_sha256 = Some(Sha256::digest(&wasm).into());
        source.role = role;
        expanded.sources.push(source);
    }
    let mut expanded_registry = registry.clone();
    let mut entry = registry.fleet_subnet_roots[0].clone();
    entry.fleet_subnet_root = root;
    entry.placement_subnet = SubnetId::from_principal(pic.get_subnet(root).unwrap());
    expanded_registry.fleet_subnet_roots.push(entry);
    (expanded, expanded_registry, second)
}
