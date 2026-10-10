//! Qualify runtime placement after real asynchronous application initialization.
//!
//! Root readiness waits retain one child and its exact operation on exhaustion.

use super::*;
use canic::dto::placement::index::PlacementIndexStatusResponse;
use canic_contracts::dto::root::RootComponentChildOperationStatus;

#[test]
pub(super) fn runtime_index_child_waits_for_startup_and_preserves_bounded_retry() {
    let _serial = crate::pic::acquire_pic_unit_test_serial_guard();
    let workspace = workspace_root_for(env!("CARGO_MANIFEST_DIR"))
        .expect("discover Canic test workspace through Cargo");
    let config_path = initial_shard_root_canister_config_path(&workspace);
    let config = AppConfigSnapshot::load(&config_path).unwrap();
    let pic = build_pic();
    let coordinator = pic.create_canister();
    pic.add_cycles(coordinator, COORDINATOR_INSTALL_CYCLES);
    let fixture = install_initial_shard_fixture(&pic, coordinator, &config_path, &[]);
    reset_prepaid_pool_assets(&pic, fixture.root_id);
    super::state_cascade::activate_components(&pic, coordinator, &fixture, &config);
    let root = fixture.root_id;
    let parent = root_pool_status(&pic, root)
        .entries
        .into_iter()
        .filter(|entry| matches!(entry.status, CanisterPoolAssetStatus::Workload { .. }))
        .find_map(
            |entry| match managed_binding_status(&pic, root, entry.canister_id) {
                ManagedCanisterBinding::Component(binding)
                    if binding.role.as_str() == "index_hub" =>
                {
                    Some(binding.canister_id)
                }
                _ => None,
            },
        )
        .expect("active index parent");
    let before = root_pool_status(&pic, root).workload;
    let first = resolve(&pic, parent, "first");
    if first.is_err() {
        for entry in root_pool_status(&pic, root).entries {
            if matches!(entry.status, CanisterPoolAssetStatus::Workload { .. })
                && matches!(managed_binding_status(&pic, root, entry.canister_id), ManagedCanisterBinding::ComponentChild(binding) if binding.role.as_str() == "index_child")
            {
                eprintln!(
                    "runtime child readiness: {:?}",
                    fixture_readiness(&pic, root, entry.canister_id)
                );
            }
        }
    }
    let first = first.unwrap();
    let second = resolve(&pic, parent, "second").unwrap();
    let child = bound_child(&first);
    assert_ne!(child, bound_child(&second));
    assert_eq!(resolve(&pic, parent, "first").unwrap(), first);
    assert_eq!(resolve(&pic, parent, "second").unwrap(), second);
    assert_eq!(root_pool_status(&pic, root).workload, before + 2);
    for child in [child, bound_child(&second)] {
        assert_eq!(startup_calls(&pic, root, child), 4);
        assert_eq!(
            fixture_readiness(&pic, root, child).status,
            canic::dto::runtime::ReadinessStatus::Ready
        );
    }
    qualify_automatic_startup_continuation(&pic, root, parent, before + 2);
    qualify_bounded_startup_retry(&pic, root, parent, before + 3);
}

fn qualify_automatic_startup_continuation(
    pic: &PocketIc,
    root: Principal,
    parent: Principal,
    before: u32,
) {
    let operation_id = [0x96; 32];
    let request = |rounds| {
        let message = pic
            .submit_call(
                parent,
                root,
                "test_create_startup_child",
                candid::encode_args((operation_id, rounds)).unwrap(),
            )
            .unwrap();
        candid::decode_one::<Result<Principal, Error>>(&complete_ingress(pic, message)).unwrap()
    };
    assert_eq!(
        request(256_u32),
        Err(Error::from_registered(
            canic_contracts::diagnostics::codes::STATE_UNAVAILABLE
        ))
    );
    let pending = child_operation(pic, root, operation_id);
    let child = pending
        .allocation
        .creation
        .as_ref()
        .unwrap()
        .canister
        .unwrap();
    assert_eq!(root_pool_status(pic, root).workload, before + 1);
    assert_eq!(
        request(255_u32),
        Err(Error::from_registered(
            canic_contracts::diagnostics::codes::CODEC_CONFLICT
        ))
    );
    assert_eq!(child_operation(pic, root, operation_id), pending);
    for _ in 0..2048 {
        let current = child_operation(pic, root, operation_id);
        if current.allocation.last_failure.is_none()
            && current.allocation.creation.as_ref().unwrap().canister == Some(child)
        {
            break;
        }
        pic.advance_time(Duration::from_millis(10));
        pic.tick();
    }
    let complete = child_operation(pic, root, operation_id);
    assert!(complete.allocation.last_failure.is_none());
    assert_eq!(startup_calls(pic, root, child), 256);
    assert_eq!(request(256_u32).unwrap(), child);
    assert_eq!(request(256_u32).unwrap(), child);
    assert_eq!(child_operation(pic, root, operation_id), complete);
    assert_eq!(root_pool_status(pic, root).workload, before + 1);
}

fn resolve(
    pic: &PocketIc,
    parent: Principal,
    key: &str,
) -> Result<PlacementIndexStatusResponse, Error> {
    let message = pic
        .submit_call(
            parent,
            Principal::anonymous(),
            "resolve_item",
            candid::encode_args((key.to_owned(),)).unwrap(),
        )
        .expect("submit placement");
    candid::decode_one(&complete_ingress(pic, message)).expect("placement response")
}

fn bound_child(status: &PlacementIndexStatusResponse) -> Principal {
    let PlacementIndexStatusResponse::Bound { instance_pid, .. } = status else {
        panic!("runtime placement must bind during its first request")
    };
    *instance_pid
}

fn startup_calls(pic: &PocketIc, root: Principal, child: Principal) -> u32 {
    pic.query_candid_as::<Result<u32, Error>, _>(child, root, "test_startup_calls", ())
        .unwrap()
        .unwrap()
}

fn child_operation(
    pic: &PocketIc,
    root: Principal,
    operation_id: [u8; 32],
) -> RootComponentChildOperationStatus {
    let RootStatusResponseFragment::Operation(RootOperationStatusResponse::ProvisionChild(status)) =
        root_status(
            pic,
            root,
            RootStatusRequestFragment::Operation(OperationStatusRequest { operation_id }),
        )
        .unwrap()
    else {
        panic!("exact child operation")
    };
    status
}

fn qualify_bounded_startup_retry(pic: &PocketIc, root: Principal, parent: Principal, before: u32) {
    let operation_id = [0x95; 32];
    let request = |rounds| {
        let message = pic
            .submit_call(
                parent,
                root,
                "test_create_startup_child",
                candid::encode_args((operation_id, rounds)).unwrap(),
            )
            .unwrap();
        candid::decode_one::<Result<Principal, Error>>(&complete_ingress(pic, message)).unwrap()
    };
    let blocked = request(256_u32);
    assert_eq!(
        blocked,
        Err(Error::from_registered(
            canic_contracts::diagnostics::codes::STATE_UNAVAILABLE
        ))
    );
    let pending = child_operation(pic, root, operation_id);
    assert_eq!(
        pending.allocation.phase,
        RootComponentAllocationPhase::Committed
    );
    let child = pending
        .allocation
        .creation
        .as_ref()
        .unwrap()
        .canister
        .unwrap();
    assert_eq!(
        pending
            .allocation
            .last_failure
            .as_ref()
            .unwrap()
            .diagnostic_code,
        canic_contracts::diagnostics::codes::STATE_UNAVAILABLE
            .raw_code()
            .raw()
    );
    assert_eq!(root_pool_status(pic, root).workload, before + 1);
    assert_eq!(
        request(255_u32),
        Err(Error::from_registered(
            canic_contracts::diagnostics::codes::CODEC_CONFLICT
        ))
    );
    assert_eq!(child_operation(pic, root, operation_id), pending);
    assert_eq!(root_pool_status(pic, root).workload, before + 1);
    pic.stop_canister(root, None).unwrap();
    pic.upgrade_canister(
        root,
        build_initial_shard_root_wasm(),
        crate::pic::upgrade_args(),
        None,
    )
    .unwrap();
    pic.start_canister(root, None).unwrap();
    assert_eq!(child_operation(pic, root, operation_id), pending);
    assert_eq!(root_pool_status(pic, root).workload, before + 1);
    for _ in 0..1024 {
        if fixture_readiness(pic, root, child).status == canic::dto::runtime::ReadinessStatus::Ready
        {
            break;
        }
        pic.advance_time(Duration::from_millis(10));
        pic.tick();
    }
    assert_eq!(startup_calls(pic, root, child), 256);
    pic.advance_time(Duration::from_secs(60));
    pic.tick();
    assert_eq!(request(256_u32).unwrap(), child);
    let complete = child_operation(pic, root, operation_id);
    assert!(complete.allocation.last_failure.is_none());
    assert_eq!(request(256_u32).unwrap(), child);
    assert_eq!(child_operation(pic, root, operation_id), complete);
    assert_eq!(root_pool_status(pic, root).workload, before + 1);
    assert_eq!(startup_calls(pic, root, child), 256);
}

// PocketIC's synchronous update waiter holds time fixed. Advance it between real
// rounds so deferred startup can execute while the original ingress is pending.
fn complete_ingress(
    pic: &PocketIc,
    message: ic_testkit::pocket_ic::common::rest::RawMessageId,
) -> Vec<u8> {
    for _ in 0..2048 {
        pic.advance_time(Duration::from_millis(10));
        pic.tick();
        if let Some(reply) = pic.ingress_status(message.clone()) {
            return reply.expect("startup request transport");
        }
    }
    panic!("startup request must reach a bounded transport outcome");
}
