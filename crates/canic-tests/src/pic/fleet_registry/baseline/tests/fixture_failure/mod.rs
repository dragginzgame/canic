//! Module: pic::fleet_registry::baseline::tests::fixture_failure
//!
//! Responsibility: qualify bounded callback failure and exact same-release recovery.
//! Boundary: real Store calls and application writes exercise IC message rollback.

use super::*;
use canic::dto::fixture_provisioning::{FixtureImportError, FixtureImportFailure};
use canic_contracts::values::fixture_import::FixtureImportRecoveryStatus;
use serde::Deserialize;

#[derive(Debug, Deserialize, Eq, PartialEq)]
struct ImportProbe {
    applied_chunks: u32,
    next_chunk: Option<u32>,
    complete: bool,
    recovery: Option<FixtureImportRecoveryStatus>,
    failure: Option<FixtureImportFailure>,
}

#[test]
pub(super) fn fixture_callback_failures_stop_without_duplicate_chunk_effects() {
    let _serial = crate::pic::acquire_pic_unit_test_serial_guard();
    let workspace = crate::workspace::discover().unwrap();
    let config_path = initial_shard_root_canister_config_path(&workspace);
    let config = AppConfigSnapshot::load(&config_path).unwrap();
    let pic = build_pic();
    let coordinator = pic.create_canister().unwrap();
    pic.add_cycles(coordinator, COORDINATOR_INSTALL_CYCLES)
        .unwrap();
    let fixture = install_initial_shard_fixture(&pic, coordinator, &config_path, &["user_hub"]);
    let root = fixture.root_id;
    let (version, sync) = join_and_synchronize_root(&pic, coordinator, &fixture);
    activate_registry_and_prepare_component_registry(&pic, coordinator, &fixture, version, sync);
    let registry =
        crate::pic::registry_discovery::snapshot(&pic, coordinator, Principal::anonymous())
            .unwrap();
    let plan = fixture_fresh_component_plan(config.model(), &registry, [0x63; 32]);
    crate::pic::batch_control::prepare(&pic, coordinator, Principal::anonymous(), plan.request)
        .unwrap();
    let hub = (0..240)
        .find_map(|_| {
            let selected = selected_fixture_targets(&pic, root)
                .into_iter()
                .find(|(_, role)| role.as_str() == "user_hub");
            if let Some((hub, _)) = selected {
                return Some(hub);
            }
            pic.advance_time(Duration::from_secs(1)).unwrap();
            pic.tick().unwrap();
            None
        })
        .expect("selected pending Hub");
    configure(&pic, root, hub, 1, false);
    let begin = await_failure(&pic, root, hub, None);
    assert_eq!(begin.applied_chunks, 0);
    assert_eq!(begin.next_chunk, None);
    let authority = begin.recovery.unwrap().assignment_hash;

    // An unavailable source must not consume the callback interruption budget.
    pic.stop_canister(fixture.response.wasm_store, Some(root))
        .unwrap();
    configure(&pic, root, hub, 2, true);
    for _ in 0..4 {
        pic.advance_time(Duration::from_mins(6)).unwrap();
        for _ in 0..8 {
            pic.tick().unwrap();
        }
        let outage = probe(&pic, root, hub);
        assert_eq!(outage.failure, None);
        assert_eq!(outage.applied_chunks, 0);
        assert!(outage.recovery.unwrap().unfinished_attempts <= 1);
    }
    pic.start_canister(fixture.response.wasm_store, Some(root))
        .unwrap();
    let chunk = await_failure(&pic, root, hub, Some(0));
    assert_eq!(chunk.applied_chunks, 0);
    assert_eq!(chunk.next_chunk, Some(0));
    // Same-release reconstruction must preserve the stopped budget and checkpoint.
    let wasm = build_initial_shard_component_wasms()[&CanisterRole::from("user_hub")].clone();
    pic.upgrade_canister(hub, wasm, crate::pic::upgrade_args(), Some(root))
        .unwrap();
    pic.advance_time(Duration::from_mins(30)).unwrap();
    for _ in 0..12 {
        pic.tick().unwrap();
    }
    assert_eq!(probe(&pic, root, hub), chunk);
    for fault in [4, 5] {
        configure(&pic, root, hub, fault, true);
        let failed = await_failure(&pic, root, hub, Some(0));
        assert_eq!(failed.applied_chunks, 0);
        assert_eq!(failed.next_chunk, Some(0));
        assert_eq!(failed.recovery.unwrap().assignment_hash, authority);
    }
    configure(&pic, root, hub, 3, true);
    let validation = await_failure(&pic, root, hub, Some(1));
    assert_eq!(validation.applied_chunks, 1);
    assert!(!validation.complete);
    // A returned read-only application error retains its typed code rather than
    // being confused with a callback whose message was rolled back.
    configure(&pic, root, hub, 6, true);
    let application = await_probe(&pic, root, hub, |status| status.failure.is_some());
    assert_eq!(
        application.failure,
        Some(FixtureImportFailure::Application { code: 42 })
    );
    assert_eq!(application.applied_chunks, 1);
    configure(&pic, root, hub, 0, true);
    let complete = await_probe(&pic, root, hub, |status| status.complete);
    assert_eq!(complete.applied_chunks, 1);
    assert_eq!(complete.next_chunk, Some(1));
    assert_eq!(complete.failure, None);
    assert_eq!(complete.recovery.unwrap().assignment_hash, authority);
}

fn await_failure(
    pic: &PocketIc,
    root: Principal,
    hub: Principal,
    position: Option<u32>,
) -> ImportProbe {
    let failed = await_probe(pic, root, hub, |status| status.failure.is_some());
    assert_eq!(
        failed.failure,
        Some(FixtureImportFailure::Interrupted {
            next_chunk: position,
            attempts: 3
        })
    );
    let attempts = failed.recovery.as_ref().unwrap();
    assert_eq!(attempts.unfinished_attempts, attempts.attempt_limit);
    assert!(!attempts.active);
    pic.advance_time(Duration::from_mins(30)).unwrap();
    for _ in 0..12 {
        pic.tick().unwrap();
    }
    assert_eq!(
        probe(pic, root, hub),
        failed,
        "stopped work must not refetch or mutate application data"
    );
    failed
}

fn await_probe(
    pic: &PocketIc,
    root: Principal,
    hub: Principal,
    ready: impl Fn(&ImportProbe) -> bool,
) -> ImportProbe {
    for _ in 0..80 {
        let status = probe(pic, root, hub);
        if ready(&status) {
            return status;
        }
        pic.advance_time(Duration::from_mins(1)).unwrap();
        for _ in 0..4 {
            pic.tick().unwrap();
        }
    }
    panic!("import did not converge: {:?}", probe(pic, root, hub));
}

fn spec(method: &'static str, method_id: u64) -> ic_testkit::pic::WireCallSpec {
    let limits = ic_wire::Limits {
        bytes: 4096,
        allocation: 16_384,
        nodes: 256,
        depth: 16,
    };
    ic_testkit::pic::WireCallSpec {
        method,
        method_id,
        request_limits: limits,
        response_limits: limits,
    }
}

fn configure(pic: &PocketIc, root: Principal, hub: Principal, fault: u8, retry: bool) {
    let response: Result<(), FixtureImportError> = ic_testkit::pic::update_wire(
        pic,
        hub,
        root,
        spec("test_fixture_fault", 0x5445_5354_0000_0901),
        &(fault, retry),
    )
    .unwrap();
    response.unwrap();
}

fn probe(pic: &PocketIc, root: Principal, hub: Principal) -> ImportProbe {
    ic_testkit::pic::query_wire(
        pic,
        hub,
        root,
        spec("test_fixture_probe", 0x5445_5354_0000_0902),
        &(),
    )
    .unwrap()
}
