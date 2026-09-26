//! Qualify the maintained seal transport against production Coordinator and Root canisters.
//!
//! Source admission and local intent are qualified in the host; this case owns actual IC effects.

use super::*;
use canic_host::fleet_ensure::{
    model::DesiredCanisterKind,
    ops::{EnsurePaths, read_state},
};
use ic_testkit::pocket_ic::common::rest::BlobCompression;

#[test]
pub(super) fn completed_reset_stops_and_clears_retained_application_once() {
    let _serial = crate::pic::acquire_pic_unit_test_serial_guard();
    let (pic, root) = capacity_import::setup(build_management_pic);
    let context = capacity_import::context(&pic, root, Principal::anonymous());
    let source = pic.create_canister_on_subnet(None, None, pic.get_subnet(root).unwrap());
    let floor = context
        .binding
        .limits
        .canister_pool
        .canister_cycles
        .to_u128();
    pic.add_cycles(source, floor + 2_000_000_000_000);
    pic.install_canister(source, b"\0asm\x01\0\0\0".to_vec(), Vec::new(), None);
    pic.set_stable_memory(source, vec![0x43; 65_536], BlobCompression::NoCompression);
    pic.set_controllers(source, None, vec![root, Principal::anonymous()])
        .unwrap();
    let before = pic.canister_status(source, Some(root)).unwrap();
    let running: canic_core::dto::canister::CanisterStatusType =
        candid::decode_one(&encode_one(before.status).unwrap()).unwrap();
    assert_eq!(
        running,
        canic_core::dto::canister::CanisterStatusType::Running
    );
    let root_before = pic.cycle_balance(root);
    let import = || {
        let response: Result<RootCommandResponseFragment, Error> = pic
            .update_candid_as(
                root,
                Principal::anonymous(),
                canic_core::protocol::CANIC_ROOT_COMMAND,
                (RootCommandFragment::ImportPoolCanister(
                    PoolCanisterRequest {
                        canister_id: source,
                    },
                ),),
            )
            .unwrap();
        assert!(
            matches!(response.unwrap(), RootCommandResponseFragment::ImportPoolCanister(PoolImportResponse::Imported { canister_id }) if canister_id == source)
        );
    };
    // Discard the successful reply as an interrupted host would; Root retains Ready ownership.
    import();
    let cleared = pic.canister_status(source, Some(root)).unwrap();
    let stopped: canic_core::dto::canister::CanisterStatusType =
        candid::decode_one(&encode_one(cleared.status).unwrap()).unwrap();
    assert_eq!(
        stopped,
        canic_core::dto::canister::CanisterStatusType::Stopped
    );
    assert!(cleared.module_hash.is_none());
    assert_eq!(cleared.memory_metrics.stable_memory_size, Nat::from(0_u8));
    assert_eq!(
        cleared.settings.controllers,
        context.binding.authority.binding.root_controllers(root)
    );
    assert!(pic.cycle_balance(source) >= floor);
    assert!(root_before.saturating_sub(pic.cycle_balance(root)) < 1_000_000_000_000);
    import();
    let replayed = pic.canister_status(source, Some(root)).unwrap();
    assert_eq!(replayed.version, cleared.version);
    assert_eq!(replayed.module_hash, cleared.module_hash);
    assert_eq!(replayed.settings.controllers, cleared.settings.controllers);
    assert_eq!(
        root_pool_status(&pic, root)
            .entries
            .iter()
            .filter(|asset| asset.canister_id == source)
            .count(),
        1
    );
}

#[test]
pub(super) fn completed_preparation_seals_reconcile_lost_responses() {
    let _serial = crate::pic::acquire_pic_unit_test_serial_guard();
    let workspace = workspace_root_for(env!("CARGO_MANIFEST_DIR"));
    let directory = literal_zero_adapter_root(&workspace).join("completed-preparation");
    let _cleanup = TestDirectoryCleanup(directory.clone());
    let (wrapper, operator, _) = prepare_isolated_icp(&directory);
    let (mut pic, root) = capacity_import::setup(build_management_pic);
    let context = capacity_import::context(&pic, root, Principal::anonymous());
    let coordinator = context.binding.authority.binding.coordinator;
    for principal in [coordinator, root] {
        pic.set_controllers(principal, None, vec![Principal::anonymous(), operator])
            .unwrap();
    }
    std::fs::write(
        directory.join("root.did"),
        operator_cli_root_candid(&workspace),
    )
    .unwrap();
    std::fs::copy(
        workspace.join("crates/canic/candid/fleet_coordinator.did"),
        directory.join("coordinator.did"),
    )
    .unwrap();
    // Minimal current desired input is sufficient for the existing single-effect adapter;
    // no historical desired authority or completed journal is manufactured.
    let desired: DesiredFleet = serde_json::from_value(serde_json::json!({
        "schema_version": 1, "bootstrap": null, "canisters": [], "protocol": null,
        "cycles_ledger": operator.to_text(), "environment": "local", "fleet": "seal-transport",
        "ledger_fee_cycles": "0", "management_creation_fee_cycles": "0", "material_cycle_threshold": "0",
        "maximum_observation_burn_cycles": "1000000000000", "maximum_update_burn_cycles": "1000000000000",
        "maximum_stalled_observations": 2, "operator": operator.to_text(), "treasury": "coordinator",
    })).unwrap();
    let state = read_state(
        &EnsurePaths::under(&directory, "local", &desired.fleet),
        &desired.fleet,
    )
    .unwrap();
    let url = pic.make_live(None);
    let replica = LocalReplicaTarget {
        environment: "local".into(),
        root_key: hex_bytes(pic.root_key().unwrap()),
        url: url.to_string(),
    };
    let platform = || {
        IcpEnsurePlatform::new(desired.clone(), wrapper.to_str().unwrap(), &directory)
            .with_local_replica(replica.clone())
    };
    let operation = "43".repeat(32);
    for (name, principal, kind) in [
        ("coordinator", coordinator, DesiredCanisterKind::Coordinator),
        ("root", root, DesiredCanisterKind::Root),
    ] {
        let candid = format!("{name}.did");
        let action = EnsureAction::SealAuthority {
            authority_kind: kind,
            name: name.into(),
            principal: principal.to_text(),
            candid_sha256: canic_core::cdk::utils::hash::sha256_hex(
                &std::fs::read(directory.join(&candid)).unwrap(),
            ),
            candid,
        };
        let intent = fixture_effect_intent(&action);
        let before = pic.canister_status(principal, Some(operator)).unwrap();
        let mut first = platform();
        assert!(
            !first
                .observe_effect(&operation, &action, &intent, &state)
                .unwrap()
                .applied
        );
        // Deliver the real seal, then discard its response and all adapter memory.
        first.apply(&operation, &action, &intent, &state).unwrap();
        drop(first);
        let sealed = pic.canister_status(principal, Some(operator)).unwrap();
        assert!(sealed.version > before.version);
        assert_eq!(sealed.settings.controllers, before.settings.controllers);
        assert_eq!(sealed.module_hash, before.module_hash);
        assert!(pic.cycle_balance(principal) <= u128::try_from(before.cycles.0).unwrap());
        for _ in 0..2 {
            let mut restarted = platform();
            assert!(
                restarted
                    .observe_effect(&operation, &action, &intent, &state)
                    .unwrap()
                    .applied
            );
        }
        let foreign = platform().observe_effect(&"44".repeat(32), &action, &intent, &state);
        assert!(matches!(
            foreign,
            Err(IcpEnsurePlatformError::CurrentProtocol(_))
        ));
        assert_eq!(
            pic.canister_status(principal, Some(operator))
                .unwrap()
                .version,
            sealed.version
        );
    }
}
