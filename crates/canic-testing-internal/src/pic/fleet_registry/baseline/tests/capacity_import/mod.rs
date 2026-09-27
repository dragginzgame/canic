//! Exercise reviewed Root custody, destructive reset and retained completion on PocketIC.

mod bootstrap;
mod transport;

use super::*;
use canic::dto::pool_import::{
    PoolImportCommand, PoolImportContext, PoolImportIdentity, PoolImportPhase,
    PoolImportReservation, PoolImportSource, PoolImportSourceProgress, PoolImportStatus,
};
use canic_core::control_plane_support::error::InternalError;
use ic_testkit::pocket_ic::common::rest::BlobCompression;

pub(super) use bootstrap::supplied_capacity_fences_bootstrap_until_publication;
pub(super) use transport::host_import_transport_recovers_signed_handoff_and_root_progress;

#[derive(CandidType)]
enum Command {
    ImportPoolCapacity(PoolImportCommand),
}

#[derive(CandidType, Deserialize)]
enum Response {
    ImportPoolCapacity(PoolImportStatus),
}

#[derive(CandidType)]
enum StatusRequest {
    PoolImport(PoolImportIdentity),
    PoolImportContext,
}

#[derive(CandidType, Deserialize)]
enum StatusResponse {
    PoolImport(Box<PoolImportStatus>),
    PoolImportContext(Box<PoolImportContext>),
}

#[test]
pub(super) fn reviewed_capacity_import_retains_exact_ids_and_reset_receipts() {
    for root_owned in [false, true] {
        retained_capacity_journey(root_owned);
    }
}

#[expect(
    clippy::too_many_lines,
    reason = "one real management journey retains each custody, wipe, accounting and replay boundary"
)]
fn retained_capacity_journey(root_owned: bool) {
    let _serial = crate::pic::acquire_pic_unit_test_serial_guard();
    let (pic, root) = setup(build_pic);
    let pic = &pic;
    let operator = Principal::self_authenticating(b"capacity import operator");
    let previous_owner = Principal::self_authenticating(b"capacity previous owner");
    pic.set_controllers(root, None, vec![Principal::anonymous(), operator])
        .unwrap();
    let context = context(pic, root, operator);
    let source = pic.create_canister();
    // A real Wasm installation and stable bytes make the destructive boundary observable.
    pic.install_canister(source, b"\0asm\x01\0\0\0".to_vec(), Vec::new(), None);
    pic.set_stable_memory(source, vec![0x6d; 65_536], BlobCompression::NoCompression);
    if !root_owned {
        pic.stop_canister(source, None).unwrap();
    }
    let controller = if root_owned { root } else { operator };
    pic.set_controllers(source, None, vec![controller, previous_owner])
        .unwrap();
    let observed = pic.canister_status(source, Some(controller)).unwrap();
    let root_observed = pic.canister_status(root, Some(operator)).unwrap();
    let final_controllers = context.binding.authority.binding.root_controllers(root);
    let mut transitional_controllers = final_controllers.clone();
    transitional_controllers.push(operator);
    transitional_controllers.sort_unstable();
    transitional_controllers.dedup();
    let reservation = PoolImportReservation {
        sequence: context.next_sequence,
        plan_sha256: [0x43; 32],
        root_authority_sha256: context.root_authority_sha256,
        root,
        operator,
        subnet: pic.get_subnet(root).unwrap(),
        transitional_controllers,
        final_controllers,
        sources: vec![PoolImportSource {
            canister_id: source,
            controllers: {
                let mut controllers = observed.settings.controllers;
                controllers.sort_unstable();
                controllers
            },
            module_sha256: Some(observed.module_hash.unwrap().try_into().unwrap()),
            canister_version: observed.version,
            stopped: !root_owned,
            disposition_sha256: [0x91; 32],
            observed_cycles: observed.cycles.0.try_into().unwrap(),
            observed_reserved_cycles: observed.reserved_cycles.0.try_into().unwrap(),
            minimum_ready_cycles: context
                .binding
                .limits
                .canister_pool
                .canister_cycles
                .to_u128(),
            maximum_debit_cycles: 1_000_000_000_000,
        }],
        observed_root_cycles: root_observed.cycles.0.try_into().unwrap(),
        observed_root_reserved_cycles: root_observed.reserved_cycles.0.try_into().unwrap(),
        minimum_root_cycles: context
            .binding
            .funding
            .root_funding
            .request_threshold
            .to_u128(),
        maximum_root_debit_cycles: 2_000_000_000_000,
        maximum_paid_calls: 24,
    };
    let identity = PoolImportIdentity {
        sequence: reservation.sequence,
        plan_sha256: reservation.plan_sha256,
    };
    assert_eq!(
        command(
            pic,
            root,
            Principal::anonymous(),
            PoolImportCommand::Reserve(Box::new(reservation.clone()))
        )
        .err(),
        Some(InternalError::forbidden().into())
    );
    let before = root_pool_status(pic, root);
    assert!(before.tracked - before.store < before.config.maximum_size);
    let reserved = command(
        pic,
        root,
        operator,
        PoolImportCommand::Reserve(Box::new(reservation.clone())),
    )
    .unwrap();
    assert_eq!(reserved.phase, PoolImportPhase::Reserved);
    assert_eq!(root_pool_status(pic, root).entries, before.entries);
    if !root_owned {
        pic.set_controllers(
            source,
            Some(operator),
            reservation.transitional_controllers.clone(),
        )
        .unwrap();
    }
    let mut steps = Vec::new();
    if root_owned {
        steps.extend([
            PoolImportSourceProgress::StopIssued,
            PoolImportSourceProgress::Stopped,
        ]);
    }
    steps.extend([
        PoolImportSourceProgress::ControllersIssued,
        PoolImportSourceProgress::ControllersConfirmed,
        PoolImportSourceProgress::UninstallIssued,
    ]);
    for expected in steps {
        // The next invocation uses protected retained evidence, as after a lost host reply.
        command(
            pic,
            root,
            operator,
            PoolImportCommand::Advance {
                identity,
                canister_id: source,
            },
        )
        .unwrap_or_else(|error| panic!("capacity step {expected:?}, root_owned={root_owned}: {error:?}; retained={:?}; source={:?}", status(pic, root, operator, identity), pic.canister_status(source, Some(root))));
        assert_eq!(
            status(pic, root, operator, identity).progress,
            vec![expected]
        );
    }
    assert_eq!(
        pic.canister_status(source, Some(operator))
            .unwrap_err()
            .error_code,
        ic_testkit::pic::ErrorCode::CanisterStatusAccessDenied
    );
    let ready = command(
        pic,
        root,
        operator,
        PoolImportCommand::Advance {
            identity,
            canister_id: source,
        },
    )
    .unwrap();
    assert_eq!(ready.phase, PoolImportPhase::Ready);
    assert!(ready.root_receipt.is_none());
    let PoolImportSourceProgress::Ready(receipt) = &ready.progress[0] else {
        panic!("import must retain the exact cleared source receipt");
    };
    assert_eq!(receipt.canister_id, source);
    assert_eq!(
        receipt.canister_version,
        receipt.before_uninstall_canister_version + 1
    );
    assert_eq!(
        receipt.retained_cycles + receipt.retained_reserved_cycles + receipt.observed_debit_cycles,
        reservation.sources[0].observed_cycles + reservation.sources[0].observed_reserved_cycles
    );
    assert!(receipt.observed_debit_cycles <= reservation.sources[0].maximum_debit_cycles);
    let cleared = pic.canister_status(source, Some(root)).unwrap();
    assert!(cleared.module_hash.is_none());
    assert_eq!(cleared.settings.controllers, reservation.final_controllers);
    assert_eq!(cleared.memory_metrics.stable_memory_size, Nat::from(0_u8));
    assert_eq!(
        command(
            pic,
            root,
            operator,
            PoolImportCommand::Release {
                identity,
                publication_sha256: [0x52; 32]
            }
        )
        .err(),
        Some(InternalError::conflict().into())
    );
    let settled = command(pic, root, operator, PoolImportCommand::Settle(identity)).unwrap();
    let root_receipt = settled.root_receipt.as_ref().unwrap();
    assert_eq!(
        root_receipt.retained_cycles
            + root_receipt.retained_reserved_cycles
            + root_receipt.observed_debit_cycles,
        reservation.observed_root_cycles + reservation.observed_root_reserved_cycles
    );
    let released = command(
        pic,
        root,
        operator,
        PoolImportCommand::Release {
            identity,
            publication_sha256: [0x52; 32],
        },
    )
    .unwrap();
    assert_eq!(
        released.phase,
        PoolImportPhase::Released {
            publication_sha256: [0x52; 32]
        }
    );
    let pool = root_pool_status(pic, root);
    assert_eq!(pool.entries.len(), before.entries.len() + 1);
    let imported = pool
        .entries
        .iter()
        .find(|entry| entry.canister_id == source)
        .unwrap();
    assert_eq!(imported.origin, CanisterPoolAssetOrigin::Imported);
    assert_eq!(imported.status, CanisterPoolAssetStatus::Ready);
    // Reuse after release must never make a retained import wipe this ID again.
    pic.install_canister(source, b"\0asm\x01\0\0\0".to_vec(), Vec::new(), Some(root));
    let replacement_state = vec![0x39; 65_536];
    pic.set_stable_memory(
        source,
        replacement_state.clone(),
        BlobCompression::NoCompression,
    );
    let reused = pic.canister_status(source, Some(root)).unwrap();
    for replay in [
        PoolImportCommand::Advance {
            identity,
            canister_id: source,
        },
        PoolImportCommand::Settle(identity),
        PoolImportCommand::Release {
            identity,
            publication_sha256: [0x52; 32],
        },
    ] {
        assert_eq!(command(pic, root, operator, replay).unwrap(), released);
    }
    assert_eq!(
        pic.canister_status(source, Some(root)).unwrap().version,
        reused.version
    );
    assert_eq!(pic.get_stable_memory(source), replacement_state);
    assert_eq!(
        pic.canister_status(source, Some(root)).unwrap().module_hash,
        reused.module_hash
    );
}

pub(super) fn setup(new_pic: fn() -> PocketIc) -> (PocketIc, Principal) {
    let pic = new_pic();
    let coordinator = pic.create_canister();
    setup_coordinator(pic, coordinator)
}

fn setup_separate_coordinator() -> (PocketIc, Principal) {
    let pic = build_management_pic();
    let coordinator = pic.create_canister_on_subnet(None, None, pic.topology().get_ii().unwrap());
    let (pic, root) = setup_coordinator(pic, coordinator);
    assert_ne!(pic.get_subnet(coordinator), pic.get_subnet(root));
    (pic, root)
}

fn setup_coordinator(pic: PocketIc, coordinator: Principal) -> (PocketIc, Principal) {
    pic.add_cycles(coordinator, COORDINATOR_INSTALL_CYCLES);
    let fixture = install_capacity_root(&pic, coordinator, None);
    reset_prepaid_pool_assets_for_count(
        &pic,
        fixture.root_id,
        fixture.init_args.canister_pool_imports.len(),
    );
    install_fixture_coordinator(&pic, coordinator, build_test_coordinator_wasm(), &fixture);
    let (version, sync) = join_and_synchronize_root(&pic, coordinator, &fixture);
    assert_registry_and_root_runtime_activation(&pic, coordinator, &fixture, version, sync);
    (pic, fixture.root_id)
}

fn install_capacity_root(
    pic: &PocketIc,
    coordinator: Principal,
    hold: Option<canic::dto::pool_import::PoolImportBootstrap>,
) -> BootstrappedRootFixture {
    let root_id = pic.create_canister();
    pic.add_cycles(root_id, ROOT_INSTALL_CYCLES);
    let subnet = pic.get_subnet(root_id).unwrap();
    let wasm_store = pic.create_canister_on_subnet(None, None, subnet);
    pic.add_cycles(wasm_store, ROOT_INSTALL_CYCLES);
    let initial_imports = if hold.is_some() { 0 } else { 4 };
    let imports = (0..initial_imports)
        .map(|_| {
            let id = pic.create_canister_on_subnet(None, None, subnet);
            pic.add_cycles(id, PREPAID_POOL_ASSET_CYCLES);
            pic.set_controllers(id, None, vec![root_id]).unwrap();
            id
        })
        .collect();
    let root_wasm = build_test_root_wasm();
    let mut store_fixture = build_root_store_fixture();
    let store_wasm = store_fixture
        .wasm
        .take()
        .unwrap_or_else(build_test_wasm_store_wasm);
    let installation_controller = Principal::from_slice(&[0x46; 29]);
    let workspace = workspace_root_for(env!("CARGO_MANIFEST_DIR"));
    let mut installed = prepare_current_root_fixture(
        pic,
        &root_wasm,
        &store_wasm,
        coordinator,
        root_id,
        wasm_store,
        installation_controller,
        store_fixture,
        &BootstrappedRootPlacement {
            canister_pool_maximum_size: None,
            canister_pool_minimum_size: None,
            canister_pool_cycles: None,
            coordinator_subnet: pic.get_subnet(coordinator),
            existing_root: Some(root_id),
            existing_wasm_store: Some(wasm_store),
            root_subnet: Some(subnet),
            component_admission_limits: None,
            fleet_id: None,
            funding: None,
            coordinator_root_funding: None,
        },
        &root_canister_config_path(&workspace),
        imports,
    );
    installed.init_args.capacity_import_bootstrap = hold;
    if let Some(key) = pic.root_key() {
        let network = canic::ids::CanonicalNetworkId::from_der_root_trust_anchor(&key).unwrap();
        let init = &mut installed.init_args;
        for fleet in [
            &mut init.authority.binding.authority.binding.fleet,
            &mut init.authority.wasm_store_authority.authority.binding.fleet,
            &mut init.wasm_store_activation.fleet,
        ] {
            fleet.fleet.canonical_network_id = network;
        }
    }
    prepare_sibling_wasm_store_controllers(pic, wasm_store, installation_controller, root_id);
    pic.install_canister(
        wasm_store,
        store_wasm,
        encode_one(FleetSubnetWasmStoreInitArgs {
            authority: installed.init_args.authority.wasm_store_authority.clone(),
            install_id: installed.init_args.wasm_store_activation.operation_id,
        })
        .unwrap(),
        Some(installation_controller),
    );
    pic.install_canister(
        root_id,
        root_wasm,
        encode_one(&installed.init_args).unwrap(),
        None,
    );
    let (request, response) = bootstrap_root_store_release_set(
        pic,
        root_id,
        wasm_store,
        installation_controller,
        &installed.init_args,
        &installed.manifest,
        installed.artifacts,
        &installed.manifest_bytes,
        installed.digest,
    );
    BootstrappedRootFixture {
        root_id,
        init_args: installed.init_args,
        coordinator_root_funding: installed.coordinator_root_funding,
        request,
        response,
    }
}

fn command(
    pic: &PocketIc,
    root: Principal,
    caller: Principal,
    action: PoolImportCommand,
) -> Result<PoolImportStatus, Error> {
    let response: Result<Response, Error> = pic
        .update_candid_as(
            root,
            caller,
            canic::protocol::CANIC_ROOT_COMMAND,
            (Command::ImportPoolCapacity(action),),
        )
        .unwrap();
    response.map(|Response::ImportPoolCapacity(status)| status)
}

pub(super) fn context(pic: &PocketIc, root: Principal, caller: Principal) -> PoolImportContext {
    let response: Result<StatusResponse, Error> = pic
        .query_candid_as(
            root,
            caller,
            canic::protocol::CANIC_ROOT_STATUS,
            (StatusRequest::PoolImportContext,),
        )
        .unwrap();
    let StatusResponse::PoolImportContext(context) = response.unwrap() else {
        panic!("Root import context response");
    };
    *context
}

fn status(
    pic: &PocketIc,
    root: Principal,
    caller: Principal,
    identity: PoolImportIdentity,
) -> PoolImportStatus {
    let response: Result<StatusResponse, Error> = pic
        .query_candid_as(
            root,
            caller,
            canic::protocol::CANIC_ROOT_STATUS,
            (StatusRequest::PoolImport(identity),),
        )
        .unwrap();
    let StatusResponse::PoolImport(status) = response.unwrap() else {
        panic!("Root import status response");
    };
    *status
}
