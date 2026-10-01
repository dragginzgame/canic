//! Real running-application import clears code and stable state once, then replays.

use super::super::*;
use ic_testkit::pocket_ic::common::rest::BlobCompression;

#[test]
pub(in crate::pic::fleet_registry::baseline) fn running_application_import_clears_state_once() {
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
