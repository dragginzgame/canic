//! Typed bootstrap holds survive Store setup and release only through reviewed capacity import.

use super::*;
use canic::dto::pool_import::PoolImportBootstrap;
use canic_host::fleet_ensure::ops::capacity_import::root_reservation;

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one production Root journey binds initialization, import publication and workload reuse"
)]
pub(in crate::pic::fleet_registry::baseline::tests) fn supplied_capacity_fences_bootstrap_until_publication()
 {
    let _serial = crate::pic::acquire_pic_unit_test_serial_guard();
    let pic = build_management_pic();
    let operator = Principal::self_authenticating(b"bootstrap capacity operator");
    let subnet = pic.topology().get_app_subnets()[0];
    let mut sources = (0..2)
        .map(|_| {
            let id = pic.create_canister_on_subnet(None, None, subnet);
            // Preserve the reviewed 1T debit allowance after setup pays installation costs.
            pic.add_cycles(id, PREPAID_POOL_ASSET_CYCLES + 1_000_000_000_000);
            pic.install_canister(id, b"\0asm\x01\0\0\0".to_vec(), Vec::new(), None);
            pic.set_stable_memory(id, vec![0x43; 65_536], BlobCompression::NoCompression);
            pic.stop_canister(id, None).unwrap();
            pic.set_controllers(id, None, vec![operator]).unwrap();
            id
        })
        .collect::<Vec<_>>();
    sources.sort_unstable();
    let hold = PoolImportBootstrap {
        review_sha256: [43; 32],
        operator,
        sources: sources.clone(),
    };
    let coordinator = pic.create_canister_on_subnet(None, None, pic.topology().get_ii().unwrap());
    pic.add_cycles(coordinator, COORDINATOR_INSTALL_CYCLES);
    let fixture = install_capacity_root(&pic, coordinator, Some(hold.clone()));
    let root = fixture.root_id;
    assert_eq!(pic.get_subnet(root), Some(subnet));
    assert_ne!(pic.get_subnet(root), pic.get_subnet(coordinator));
    pic.set_controllers(root, None, vec![Principal::anonymous(), operator])
        .unwrap();
    let initial = root_pool_status(&pic, root);
    assert_eq!(initial.tracked, initial.store);
    assert!(initial.config.minimum_size > 0);
    root_command(&pic, root, RootCommandFragment::MaintainPool).unwrap();
    assert_eq!(root_pool_status(&pic, root).entries, initial.entries);
    let unregistered: Result<StatusResponse, Error> = pic
        .query_candid_as(
            root,
            operator,
            canic::protocol::CANIC_ROOT_STATUS,
            (StatusRequest::PoolImportContext,),
        )
        .unwrap();
    assert!(matches!(unregistered, Err(error) if error == InternalError::unavailable().into()));
    install_fixture_coordinator(&pic, coordinator, build_test_coordinator_wasm(), &fixture);
    let (version, sync) = join_and_synchronize_root(&pic, coordinator, &fixture);
    let preparation = activate_registry_and_prepare_component_registry(
        &pic,
        coordinator,
        &fixture,
        version,
        sync,
    );
    let current = context(&pic, root, operator);
    assert_eq!(current.bootstrap, Some(hold.clone()));
    let plan = transport::review(&pic, root, &sources, operator, &current);
    let reservation = root_reservation(&plan).unwrap();
    let identity = PoolImportIdentity {
        sequence: reservation.sequence,
        plan_sha256: reservation.plan_sha256,
    };
    let mut wrong = reservation.clone();
    wrong.sources.pop();
    assert_eq!(
        command(
            &pic,
            root,
            operator,
            PoolImportCommand::Reserve(Box::new(wrong))
        )
        .err(),
        Some(InternalError::conflict().into())
    );
    assert_eq!(context(&pic, root, operator).bootstrap, Some(hold.clone()));
    command(
        &pic,
        root,
        operator,
        PoolImportCommand::Reserve(Box::new(reservation.clone())),
    )
    .unwrap();
    for id in &sources {
        pic.set_controllers(
            *id,
            Some(operator),
            reservation.transitional_controllers.clone(),
        )
        .unwrap();
        for _ in 0..4 {
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
        }
        let cleared = pic.canister_status(*id, Some(root)).unwrap();
        assert!(cleared.module_hash.is_none());
        assert_eq!(cleared.memory_metrics.stable_memory_size, Nat::from(0_u8));
    }
    assert_eq!(context(&pic, root, operator).bootstrap, Some(hold));
    assert!(
        command(
            &pic,
            root,
            operator,
            PoolImportCommand::Release {
                identity,
                publication_sha256: [44; 32]
            }
        )
        .is_err()
    );
    command(&pic, root, operator, PoolImportCommand::Settle(identity)).unwrap();
    let released = command(
        &pic,
        root,
        operator,
        PoolImportCommand::Release {
            identity,
            publication_sha256: [44; 32],
        },
    )
    .unwrap();
    let workload = assert_component_allocation(&pic, &fixture, preparation);
    let mut assigned = vec![workload.issuer.canister_id, workload.verifier.canister_id];
    assigned.sort_unstable();
    assert_eq!(assigned, sources);
    assert!(context(&pic, root, operator).bootstrap.is_none());
    let versions = sources
        .iter()
        .map(|id| pic.canister_status(*id, Some(root)).unwrap().version)
        .collect::<Vec<_>>();
    for id in &sources {
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
            .unwrap(),
            released
        );
    }
    assert_eq!(
        sources
            .iter()
            .map(|id| pic.canister_status(*id, Some(root)).unwrap().version)
            .collect::<Vec<_>>(),
        versions
    );
}
