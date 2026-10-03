//! Real signed queries and certificates qualify the collector, not runtime role quiescence.

mod funding;

use super::*;
use candid::{CandidType, Principal};
use canic_core::{
    dto::{
        error::Error,
        pool::{
            CanisterPoolAsset, CanisterPoolAssetOrigin, CanisterPoolAssetStatus,
            CanisterPoolResponse,
        },
    },
    ids::{CanonicalNetworkId, SubnetId},
};
use ic_agent::identity::BasicIdentity;
use ic_testkit::pocket_ic::{CanisterSettings, PocketIcBuilder};
use sha2_host::{Digest, Sha256};
use std::{fs, process::Command, time::SystemTime};

#[derive(CandidType)]
enum RegistryReply {
    Registry(Box<FleetRegistry>),
}

#[derive(CandidType)]
enum PoolReply {
    Pool(Box<CanisterPoolResponse>),
}

fn wire_fixture() -> Vec<u8> {
    let directory = crate::test_support::temp_dir("release-inventory-wire");
    fs::create_dir_all(&directory).unwrap();
    let source = directory.join("fixture.rs");
    let output = directory.join("fixture.wasm");
    fs::write(&source, include_str!("../fixture/mod.rs")).unwrap();
    let result = Command::new("rustc")
        .args([
            "--edition=2024",
            "--crate-type=cdylib",
            "--target=wasm32-unknown-unknown",
            "-O",
        ])
        .arg(&source)
        .arg("-o")
        .arg(&output)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let bytes = fs::read(output).unwrap();
    fs::remove_dir_all(directory).unwrap();
    bytes
}

fn pool(review: &FleetReleaseReviewRecord, registry: &FleetRegistry) -> CanisterPoolResponse {
    let mut entries = review.sources[2..]
        .iter()
        .map(|source| CanisterPoolAsset {
            canister_id: source.binding.canister_id,
            creation_receipt: None,
            cycles: Cycles::new(1_000),
            origin: CanisterPoolAssetOrigin::Imported,
            status: if matches!(source.role, FleetReleaseRole::Store { .. }) {
                CanisterPoolAssetStatus::Store
            } else {
                CanisterPoolAssetStatus::PendingReset
            },
            added_at_ns: 1,
            updated_at_ns: 1,
        })
        .collect::<Vec<_>>();
    entries.sort_by_key(|entry| entry.canister_id);
    CanisterPoolResponse {
        config: registry.fleet_subnet_roots[0].limits.canister_pool.clone(),
        tracked: 2,
        store: 1,
        store_deletion_pending: 0,
        pooled: 1,
        workload: 0,
        surplus: 1,
        ready: 0,
        pending_reset: 1,
        claimed: 0,
        recycling: 0,
        handing_off: 0,
        failed: 0,
        completed_handoffs: 0,
        pending_creation: None,
        pending_handoff: None,
        entries,
        next_start_after: None,
    }
}

#[test]
#[ignore = "the workspace runner supplies one shared PocketIC server and serial execution"]
#[expect(
    clippy::too_many_lines,
    reason = "one signed-query journey covers exact closure and independent refusal boundaries"
)]
fn governed_pocketic_release_inventory_binds_complete_ownership() {
    let mut pic = crate::test_support::start_pocket_ic(
        PocketIcBuilder::new()
            .with_nns_subnet()
            .with_application_subnet(),
    );
    pic.set_time(SystemTime::now().into());
    let agent = Agent::builder()
        .with_url(pic.make_live(None))
        .with_identity(BasicIdentity::from_raw_key(&[19; 32]))
        .with_max_response_body_size(inventory::RESPONSE_BYTES)
        .build()
        .unwrap();
    agent.set_root_key(pic.root_key().unwrap());
    let operator = agent.get_principal().unwrap();
    let ids = (0..4)
        .map(|_| pic.create_canister_with_settings(Some(operator), None))
        .collect::<Vec<_>>();
    let (mut review, mut registry) = fixture();
    review.authority.operator = operator;
    review.authority.coordinator = ids[0];
    review.authority.network_root_key_sha256 = Sha256::digest(agent.read_root_key()).into();
    review.authority.fleet.fleet.canonical_network_id =
        CanonicalNetworkId::from_der_root_trust_anchor(&agent.read_root_key()).unwrap();
    let wasm = wire_fixture();
    for (source, id) in review.sources.iter_mut().zip(&ids) {
        source.binding.canister_id = *id;
        source.binding.subnet = SubnetId::from_principal(pic.get_subnet(*id).unwrap());
        source.binding.controllers = vec![operator];
        source.binding.module_sha256 = Some(Sha256::digest(&wasm).into());
        match &mut source.role {
            FleetReleaseRole::Store { root } | FleetReleaseRole::Child { root } => *root = ids[1],
            _ => {}
        }
    }
    registry.authority.binding.fleet = review.authority.fleet.clone();
    registry.authority.binding.coordinator = ids[0];
    registry.authority.binding.coordinator_subnet = review.sources[0].binding.subnet;
    registry.authority.binding.recovery_controllers = vec![operator];
    registry.fleet_subnet_roots[0].fleet_subnet_root = ids[1];
    registry.fleet_subnet_roots[0].placement_subnet = review.sources[1].binding.subnet;
    let registry_reply = candid::encode_one(Ok::<_, Error>(RegistryReply::Registry(Box::new(
        registry.clone(),
    ))))
    .unwrap();
    let pool_reply = candid::encode_one(Ok::<_, Error>(PoolReply::Pool(Box::new(pool(
        &review, &registry,
    )))))
    .unwrap();
    for (index, id) in ids.iter().enumerate() {
        let reply = if index == 0 {
            registry_reply.clone()
        } else {
            pool_reply.clone()
        };
        pic.install_canister(*id, wasm.clone(), reply, Some(operator));
    }
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let collect = |review: &FleetReleaseReviewRecord, registry: &FleetRegistry| {
        runtime.block_on(collect_with_agent(&agent, review, registry))
    };
    // The pending-reset child remains in the census; this result grants no quiescence.
    assert_eq!(
        collect(&review, &registry).unwrap().children,
        expected_ownership(&review).unwrap()
    );
    let mut omitted = review.clone();
    omitted.sources.pop();
    assert!(matches!(
        collect(&omitted, &registry),
        Err(ReleaseInventoryError::Evidence(
            FleetReleaseError::Inventory
        ))
    ));
    let mut wrong_registry = registry.clone();
    wrong_registry.revision += 1;
    assert!(matches!(
        collect(&review, &wrong_registry),
        Err(ReleaseInventoryError::Evidence(
            FleetReleaseError::Authority
        ))
    ));
    let mut wrong_signer = review.clone();
    wrong_signer.authority.operator = Principal::anonymous();
    assert!(matches!(
        collect(&wrong_signer, &registry),
        Err(ReleaseInventoryError::Authentication(
            super::super::super::observation::ReleaseObservationError::Authority
        ))
    ));
    funding::assert_census(&pic, &agent, &runtime, &review, &registry);
    pic.update_call(ids[1], operator, "replace", b"DIDL".to_vec())
        .unwrap();
    assert!(matches!(
        collect(&review, &registry),
        Err(ReleaseInventoryError::Query(
            CapacityImportJournalError::InventoryObservation { .. }
        ))
    ));
    pic.update_call(ids[1], operator, "replace", pool_reply)
        .unwrap();
    pic.update_canister_settings(
        ids[3],
        Some(operator),
        CanisterSettings {
            controllers: Some(vec![operator, Principal::anonymous()]),
            ..CanisterSettings::default()
        },
    )
    .unwrap();
    assert!(matches!(
        collect(&review, &registry),
        Err(ReleaseInventoryError::Evidence(
            FleetReleaseError::Custody { .. }
        ))
    ));
}
