//! Module: pic::fleet_registry::baseline::tests::caller_authority
//!
//! Installed receiver authority, unavailable recipients and post-await effect fencing.

use super::*;
use ic_platform::pocket_ic::common::rest::RawMessageId;

fn probe(pic: &PocketIc, hub: Principal, caller: Principal) -> Result<Principal, Error> {
    ic_testkit::pic::update_wire::<_, Result<ic_platform::Principal, Error>>(
        pic,
        hub,
        caller,
        crate::pic::application_probe::CALLER_PROBE,
        &(),
    )
    .map(|reply| reply.map(Into::into))
    .expect("caller probe transport")
}

pub(super) fn qualify_live_projection(
    pic: &PocketIc,
    root: Principal,
    hub: Principal,
    shard: Principal,
) {
    assert_eq!(probe(pic, hub, shard), Ok(shard));
    let denied = Err(Error::from_registered(
        canic_contracts::diagnostics::codes::AUTHORITY_UNAUTHORIZED,
    ));
    assert_eq!(probe(pic, hub, hub), denied);
    assert_eq!(probe(pic, hub, Principal::anonymous()), denied);
    let target: Result<Principal, Error> =
        ic_testkit::pic::update_wire::<_, Result<ic_platform::Principal, Error>>(
            pic,
            hub,
            root,
            crate::pic::application_probe::CALLER_TARGET,
            &ic_platform::Principal::from(shard),
        )
        .map(|reply| reply.map(Into::into))
        .unwrap();
    assert_eq!(target, Ok(shard));
    let invalid: Result<Principal, Error> =
        ic_testkit::pic::update_wire::<_, Result<ic_platform::Principal, Error>>(
            pic,
            hub,
            root,
            crate::pic::application_probe::CALLER_TARGET,
            &ic_platform::Principal::from(hub),
        )
        .map(|reply| reply.map(Into::into))
        .unwrap();
    assert_eq!(invalid, denied);
    let unauthorized_proxy: Result<Principal, Error> =
        ic_testkit::pic::update_wire::<_, Result<ic_platform::Principal, Error>>(
            pic,
            hub,
            shard,
            crate::pic::application_probe::CALLER_TARGET,
            &ic_platform::Principal::from(shard),
        )
        .map(|reply| reply.map(Into::into))
        .unwrap();
    assert_eq!(
        unauthorized_proxy,
        Err(Error::from_registered(
            canic_contracts::diagnostics::codes::AUTHORITY_UNAVAILABLE
        ))
    );
    pic.stop_canister(root, None).unwrap();
    assert_eq!(probe(pic, hub, shard), Ok(shard));
    pic.start_canister(root, None).unwrap();
    let before = effects(pic, root, hub).1;
    let wasm = build_initial_shard_component_wasms()[&CanisterRole::from("user_hub")].clone();
    pic.upgrade_canister(hub, wasm, crate::pic::upgrade_args(), Some(root))
        .unwrap();
    for _ in 0..8 {
        pic.tick();
    }
    assert_eq!(probe(pic, hub, shard), Ok(shard));
    assert_eq!(effects(pic, root, hub).1, before);
}

pub(super) fn hold_effect(
    pic: &PocketIc,
    root: Principal,
    hub: Principal,
    shard: Principal,
) -> (RawMessageId, u64) {
    assert_eq!(probe(pic, hub, shard), Ok(shard));
    ic_testkit::pic::update_wire::<_, Result<(), Error>>(
        pic,
        hub,
        root,
        crate::pic::application_probe::CALLER_HOLD,
        &true,
    )
    .unwrap()
    .unwrap();
    let before = effects(pic, root, hub).1;
    let message = pic
        .submit_call(
            hub,
            shard,
            "test_caller_effect",
            ic_wire::native::encode_request(
                crate::pic::application_probe::CALLER_EFFECT.method_id,
                &(),
                crate::pic::application_probe::CALLER_EFFECT.request_limits,
            )
            .unwrap(),
        )
        .unwrap();
    for _ in 0..24 {
        pic.tick();
        if effects(pic, root, hub).0 {
            return (message, before);
        }
    }
    panic!("caller effect must reach its real await boundary");
}

pub(super) fn assert_effect_fenced(
    pic: &PocketIc,
    root: Principal,
    hub: Principal,
    shard: Principal,
    held: (RawMessageId, u64),
) {
    ic_testkit::pic::update_wire::<_, Result<(), Error>>(
        pic,
        hub,
        root,
        crate::pic::application_probe::CALLER_HOLD,
        &false,
    )
    .unwrap()
    .unwrap();
    let result: Result<u64, Error> = ic_wire::native::decode_response(
        crate::pic::application_probe::CALLER_EFFECT.method_id,
        &pic.await_call(held.0).unwrap(),
        crate::pic::application_probe::CALLER_EFFECT.response_limits,
    )
    .unwrap();
    assert_eq!(
        result,
        Err(Error::from_registered(
            canic_contracts::diagnostics::codes::AUTHORITY_INACTIVE
        ))
    );
    assert_eq!(effects(pic, root, hub), (false, held.1));
    assert_eq!(
        probe(pic, hub, shard),
        Err(Error::from_registered(
            canic_contracts::diagnostics::codes::AUTHORITY_INACTIVE
        ))
    );
}

fn effects(pic: &PocketIc, root: Principal, hub: Principal) -> (bool, u64) {
    ic_testkit::pic::query_wire::<_, Result<(bool, u64), Error>>(
        pic,
        hub,
        root,
        crate::pic::application_probe::CALLER_EFFECT_STATUS,
        &(),
    )
    .unwrap()
    .unwrap()
}

pub(super) fn hold_during_pending_denial(
    pic: &PocketIc,
    binding: &canic::ids::ComponentChildBinding,
    request: &canic::dto::component_registry::RootComponentSubtreeRemovalRequest,
) -> (RawMessageId, u64) {
    let root = binding.component.fleet_subnet_root;
    let hub = binding.component.canister_id;
    let shard = binding.canister_id;
    pic.stop_canister(hub, Some(root)).unwrap();
    let unavailable_receiver = remove_subtree(pic, root, request.clone()).err().unwrap();
    assert_eq!(
        unavailable_receiver,
        Error::from_registered(canic_contracts::diagnostics::codes::STATE_UNAVAILABLE)
    );
    root_membership::assert_child_discovery(pic, root, &binding.component, binding);
    pic.upgrade_canister(
        root,
        build_initial_shard_root_wasm(),
        crate::pic::upgrade_args(),
        None,
    )
    .unwrap();
    root_membership::assert_child_discovery(pic, root, &binding.component, binding);
    pic.start_canister(hub, Some(root)).unwrap();
    let competing: Result<Principal, Error> =
        ic_testkit::pic::update_wire::<_, Result<ic_platform::Principal, Error>>(
            pic,
            hub,
            root,
            crate::pic::application_probe::CREATE_FIXTURE_CHILD,
            &[0xd9_u8; 32],
        )
        .map(|reply| reply.map(Into::into))
        .unwrap();
    assert_eq!(
        competing,
        Err(Error::from_registered(
            canic_contracts::diagnostics::codes::STATE_CONFLICT
        ))
    );
    hold_effect(pic, root, hub, shard)
}
