//! Qualify canonical Root membership discovery through its guarded update.

use super::*;
use canic::dto::component_registry::{RootMembershipRequest, RootMembershipResponse};
use ic_platform::candid::contract::types::TypeInner;
use ic_platform::candid::contract::types::{Function, internal::TypeContainer, subtype};
use ic_testkit::pocket_ic::ErrorCode;

pub(super) fn assert_candid_contract(candid: &[u8]) {
    let source = std::str::from_utf8(candid).expect("Root Candid UTF-8");
    let program = source
        .parse::<ic_platform::candid_parser::IDLProg>()
        .expect("parse generated Root Candid");
    let mut environment = ic_platform::candid::contract::TypeEnv::new();
    let actor = ic_platform::candid_parser::check_prog(&mut environment, &program)
        .expect("type-check generated Root Candid");
    let actor = actor.expect("Root service");
    let actual = environment
        .get_method(&actor, canic::protocol::CANIC_ROOT_MEMBERSHIP)
        .expect("canonical Root membership endpoint")
        .clone();
    let mut rust = TypeContainer::new();
    let expected = TypeInner::Func(Function {
        args: vec![rust.add::<RootMembershipRequest>()],
        rets: vec![rust.add::<Result<RootMembershipResponse, Error>>()],
        modes: vec![],
    })
    .into();
    let expected = environment.merge_type(rust.env, expected);
    subtype::equal(
        &mut std::collections::HashSet::default(),
        &environment,
        &TypeInner::Func(actual).into(),
        &expected,
    )
    .expect("Root lookup is an update with the exact public request and result");
}

pub(super) fn assert_prepared_root_refuses_discovery(pic: &PocketIc, root: Principal) {
    // The ordinary Fleet fence runs before the lookup's endpoint guard and handler.
    let rejected =
        (|| -> Result<(Result<RootMembershipResponse, Error>,), Box<dyn std::error::Error + Send + Sync>> {

       let request_bytes = ic_platform::candid::contract::encode_args((RootMembershipRequest {
                subject: Principal::management_canister(),
            },))?;
       let reply = ic_testkit::pic::try_pocket_ic_operation(|| (pic).update_call(root, Principal::anonymous(), canic::protocol::CANIC_ROOT_MEMBERSHIP, request_bytes))
           .map_err(ic_testkit::pic::PocketIcCallFailure::Operation)?
           .map_err(ic_testkit::pic::PocketIcCallFailure::Rejected)?;
       ic_platform::candid::decode_args(&reply, canic::testing::CANDID_REPLY_LIMITS).map_err(Into::into)
   })()
        .map(|(reply,)| reply)
        .expect_err("Prepared Root must keep the lookup behind its Fleet fence");
    let Some(ic_testkit::pic::PocketIcCallFailure::Rejected(response)) =
        rejected.downcast_ref::<ic_testkit::pic::PocketIcCallFailure>()
    else {
        panic!("expected canister rejection");
    };
    assert_eq!(response.error_code, ErrorCode::CanisterCalledTrap,);
}

pub(super) fn assert_removed_child_absent(
    pic: &PocketIc,
    root: Principal,
    binding: &canic::ids::ComponentChildBinding,
) {
    for caller in [Principal::anonymous(), binding.component.canister_id] {
        assert_eq!(
            lookup(pic, root, caller, binding.canister_id)
                .expect("removed child is an ordinary negative")
                .member,
            None
        );
    }
    assert_eq!(
        lookup(
            pic,
            root,
            binding.canister_id,
            binding.component.canister_id
        ),
        Err(Error::from(
            canic_core::access::AccessError::ActiveComponentRequired,
        ))
    );
}

fn lookup(
    pic: &PocketIc,
    root: Principal,
    caller: Principal,
    subject: Principal,
) -> Result<RootMembershipResponse, Error> {
    (|| -> Result<_, Box<dyn std::error::Error + Send + Sync>> {
        let request_bytes =
            ic_platform::candid::contract::encode_args((RootMembershipRequest { subject },))?;
        let reply = ic_testkit::pic::try_pocket_ic_operation(|| {
            (pic).update_call(
                root,
                caller,
                canic::protocol::CANIC_ROOT_MEMBERSHIP,
                request_bytes,
            )
        })
        .map_err(ic_testkit::pic::PocketIcCallFailure::Operation)?
        .map_err(ic_testkit::pic::PocketIcCallFailure::Rejected)?;
        ic_platform::candid::decode_args(&reply, canic::testing::CANDID_REPLY_LIMITS)
            .map_err(Into::into)
    })()
    .map(|(reply,)| reply)
    .expect("membership lookup transport")
}

pub(super) fn assert_child_discovery(
    pic: &PocketIc,
    root: Principal,
    parent: &ComponentBinding,
    child: &canic::ids::ComponentChildBinding,
) {
    for caller in [
        Principal::anonymous(),
        parent.canister_id,
        child.canister_id,
    ] {
        let observed = lookup(pic, root, caller, child.canister_id).expect("active child lookup");
        assert_eq!(
            observed.member,
            Some(ManagedCanisterBinding::ComponentChild(child.clone()))
        );
    }
}

pub(super) fn assert_membership_discovery(fixture: &ActiveComponentRegistryFixture) {
    let unused = root_pool_status(fixture.pic(), fixture.root)
        .entries
        .into_iter()
        .find(|entry| matches!(entry.status, CanisterPoolAssetStatus::Ready))
        .expect("fixture retains ready unallocated capacity")
        .canister_id;
    let lookup = |caller, subject| lookup(fixture.pic(), fixture.root, caller, subject);
    for caller in [Principal::anonymous(), fixture.issuer.canister_id] {
        let observed = lookup(caller, fixture.verifier.canister_id).unwrap();
        assert_eq!(
            observed.member,
            Some(ManagedCanisterBinding::Component(fixture.verifier.clone()))
        );
        assert_eq!(
            lookup(caller, Principal::management_canister())
                .unwrap()
                .member,
            None
        );
        // Use observed unused capacity; original pool positions may now be workloads.
        assert_eq!(lookup(caller, unused).unwrap().member, None);
    }
    let outsider = Principal::self_authenticating(b"unregistered membership requester");
    let denied = lookup(outsider, fixture.verifier.canister_id).unwrap_err();
    assert_eq!(
        denied,
        Error::from(canic_core::access::AccessError::ActiveComponentRequired)
    );
}
