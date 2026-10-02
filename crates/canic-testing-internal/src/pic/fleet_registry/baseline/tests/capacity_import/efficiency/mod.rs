//! Count real mainnet-path import calls and reject placement drift during the same operation.

use super::*;
use canic_core::control_plane_support::policy::pool_import;
use std::time::Instant;

#[test]
pub(in crate::pic::fleet_registry::baseline::tests) fn mainnet_import_counts_calls_and_refreshes_placement()
 {
    journey(false);
    journey(true);
}

#[expect(
    clippy::too_many_lines,
    reason = "one mainnet-shaped journey retains admission, placement drift, exact effects, conservation and replay evidence"
)]
fn journey(inject_drift: bool) {
    let _serial = crate::pic::acquire_pic_unit_test_serial_guard();
    let (root_wasm, registry_wasm) = build_mainnet_refill_wasms();
    let pic = build_management_pic();
    let coordinator = pic.create_canister();
    pic.add_cycles(coordinator, COORDINATOR_INSTALL_CYCLES);
    let fixture = install_capacity_root_with_wasm(&pic, coordinator, None, root_wasm, Some(16));
    let root = fixture.root_id;
    let subnet = pic.get_subnet(root).unwrap();
    let operator = Principal::self_authenticating(b"mainnet import efficiency operator");
    pic.set_controllers(root, None, vec![Principal::anonymous(), operator])
        .unwrap();
    let floor = fixture
        .init_args
        .authority
        .binding
        .limits
        .canister_pool
        .canister_cycles
        .to_u128();
    let sources = (0..8)
        .map(|_| {
            let id = pic.create_canister_on_subnet(None, None, subnet);
            pic.add_cycles(id, floor + 2_000_000_000_000);
            pic.install_canister(id, b"\0asm\x01\0\0\0".to_vec(), Vec::new(), None);
            pic.set_stable_memory(id, vec![0x56; 65_536], BlobCompression::NoCompression);
            pic.set_controllers(id, None, vec![root, operator]).unwrap();
            id
        })
        .collect::<Vec<_>>();
    let routes = fixture
        .init_args
        .canister_pool_imports
        .iter()
        .copied()
        .chain(sources.iter().copied())
        .chain([root])
        .collect::<Vec<_>>();
    install_journey_registry(&pic, &registry_wasm, root, subnet, routes.clone());
    reset_prepaid_pool_assets_for_count(&pic, root, fixture.init_args.canister_pool_imports.len());
    install_fixture_coordinator(&pic, coordinator, build_test_coordinator_wasm(), &fixture);
    let (version, sync) = join_and_synchronize_root(&pic, coordinator, &fixture);
    assert_registry_and_root_runtime_activation(&pic, coordinator, &fixture, version, sync);
    let current = context(&pic, root, operator);
    let maximum_paid_calls = pool_import::recommended_calls(sources.len()).unwrap();
    let observed_root = pic.canister_status(root, Some(operator)).unwrap();
    let mut transitional_controllers = current.binding.authority.binding.root_controllers(root);
    transitional_controllers.push(operator);
    transitional_controllers.sort_unstable();
    transitional_controllers.dedup();
    let reservation = PoolImportReservation {
        sequence: current.next_sequence,
        plan_sha256: [0x85; 32],
        root_authority_sha256: current.root_authority_sha256,
        root,
        operator,
        subnet,
        transitional_controllers,
        final_controllers: current.binding.authority.binding.root_controllers(root),
        sources: sources
            .iter()
            .map(|id| {
                let observed = pic.canister_status(*id, Some(root)).unwrap();
                let mut controllers = observed.settings.controllers;
                controllers.sort_unstable();
                PoolImportSource {
                    canister_id: *id,
                    controllers,
                    module_sha256: Some(observed.module_hash.unwrap().try_into().unwrap()),
                    canister_version: observed.version,
                    stopped: false,
                    disposition_sha256: [0x86; 32],
                    observed_cycles: observed.cycles.0.try_into().unwrap(),
                    observed_reserved_cycles: observed.reserved_cycles.0.try_into().unwrap(),
                    minimum_ready_cycles: floor,
                    maximum_debit_cycles: 1_000_000_000_000,
                }
            })
            .collect(),
        observed_root_cycles: observed_root.cycles.0.try_into().unwrap(),
        observed_root_reserved_cycles: observed_root.reserved_cycles.0.try_into().unwrap(),
        minimum_root_cycles: current
            .binding
            .funding
            .root_funding
            .request_threshold
            .to_u128(),
        maximum_root_debit_cycles: pool_import::required_debit(
            current.maximum_call_debit_cycles,
            maximum_paid_calls,
        )
        .unwrap(),
        maximum_paid_calls,
    };
    let mut insufficient = reservation.clone();
    insufficient.maximum_paid_calls = 72;
    assert_eq!(
        command(
            &pic,
            root,
            operator,
            PoolImportCommand::Reserve(Box::new(insufficient))
        )
        .err(),
        Some(InternalError::resource_exhausted().into())
    );
    assert_eq!(context(&pic, root, operator).active_import, None);
    command(
        &pic,
        root,
        operator,
        PoolImportCommand::Reserve(Box::new(reservation.clone())),
    )
    .unwrap();
    let identity = PoolImportIdentity {
        sequence: reservation.sequence,
        plan_sha256: reservation.plan_sha256,
    };
    let started = Instant::now();
    for (index, id) in sources.iter().enumerate() {
        for expected in [
            PoolImportSourceProgress::StopIssued,
            PoolImportSourceProgress::ControllersIssued,
            PoolImportSourceProgress::UninstallIssued,
        ] {
            let progress = command(
                &pic,
                root,
                operator,
                PoolImportCommand::Advance {
                    identity,
                    canister_id: *id,
                },
            )
            .unwrap();
            assert_eq!(progress.progress[index], expected);
            if inject_drift && index == 0 && expected == PoolImportSourceProgress::StopIssued {
                let before = pic.canister_status(*id, Some(root)).unwrap();
                replace_routes(
                    &pic,
                    &registry_wasm,
                    root,
                    pic.topology().get_ii().unwrap(),
                    routes.clone(),
                );
                assert_eq!(
                    command(
                        &pic,
                        root,
                        operator,
                        PoolImportCommand::Advance {
                            identity,
                            canister_id: *id
                        }
                    )
                    .err(),
                    Some(InternalError::conflict().into())
                );
                let retained = status(&pic, root, operator, identity);
                assert_eq!(
                    retained.progress[index],
                    PoolImportSourceProgress::StopIssued
                );
                assert_eq!(retained.reservation, reservation);
                assert_eq!(
                    pic.canister_status(*id, Some(root)).unwrap().version,
                    before.version
                );
                replace_routes(&pic, &registry_wasm, root, subnet, routes.clone());
            }
        }
        command(
            &pic,
            root,
            operator,
            PoolImportCommand::Advance {
                identity,
                canister_id: *id,
            },
        )
        .unwrap();
        let cleared = pic.canister_status(*id, Some(root)).unwrap();
        assert!(cleared.module_hash.is_none());
        assert_eq!(cleared.memory_metrics.stable_memory_size, Nat::from(0_u8));
        assert_eq!(cleared.settings.controllers, reservation.final_controllers);
    }
    let settled = command(&pic, root, operator, PoolImportCommand::Settle(identity)).unwrap();
    assert_eq!(
        settled.paid_calls,
        pool_import::minimum_calls(sources.len()).unwrap() + u32::from(inject_drift)
    );
    let receipt = settled.root_receipt.as_ref().unwrap();
    assert!(receipt.observed_debit_cycles <= reservation.maximum_root_debit_cycles);
    assert!(receipt.retained_cycles >= reservation.minimum_root_cycles);
    eprintln!(
        "[IMPORT-EFFICIENCY] sources={} paid_calls={} elapsed_ms={} observed_root_debit_cycles={} placement_drift={inject_drift}",
        sources.len(),
        settled.paid_calls,
        started.elapsed().as_millis(),
        receipt.observed_debit_cycles
    );
    let released = command(
        &pic,
        root,
        operator,
        PoolImportCommand::Release {
            identity,
            publication_sha256: [0x87; 32],
        },
    )
    .unwrap();
    for id in sources {
        assert_eq!(
            command(
                &pic,
                root,
                operator,
                PoolImportCommand::Advance {
                    identity,
                    canister_id: id
                }
            )
            .unwrap(),
            released
        );
    }
    assert_eq!(
        command(&pic, root, operator, PoolImportCommand::Settle(identity)).unwrap(),
        released
    );
}

fn replace_routes(
    pic: &PocketIc,
    wasm: &[u8],
    root: Principal,
    subnet: Principal,
    routes: Vec<Principal>,
) {
    pic.reinstall_canister(
        Principal::from_text("rwlgt-iiaaa-aaaaa-aaaaa-cai").unwrap(),
        wasm.to_vec(),
        encode_one(CyclesLedgerStubInitArgs {
            canister_ids: routes,
            expected_controllers_by_index: None,
            expected_root: root,
            expected_subnet: subnet,
            initial_balances: None,
            pending_first_index: None,
            withdrawal_fee: None,
        })
        .unwrap(),
        None,
    )
    .unwrap();
}
