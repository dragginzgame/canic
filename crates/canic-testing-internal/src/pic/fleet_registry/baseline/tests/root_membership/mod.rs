//! Qualify canonical Root membership discovery through its guarded update.

use super::*;
use candid::types::{Function, TypeInner, internal::TypeContainer, subtype};
use canic::dto::component_registry::{RootMembershipRequest, RootMembershipResponse};
use ic_testkit::{pic::CandidCallError, pocket_ic::ErrorCode};

pub(super) fn assert_candid_contract(candid: &[u8]) {
    let source = std::str::from_utf8(candid).expect("Root Candid UTF-8");
    let program = source
        .parse::<candid_parser::IDLProg>()
        .expect("parse generated Root Candid");
    let mut environment = candid::TypeEnv::new();
    let actor = candid_parser::check_prog(&mut environment, &program)
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
        &mut subtype::Gamma::new(),
        &environment,
        &TypeInner::Func(actual).into(),
        &expected,
    )
    .expect("Root lookup is an update with the exact public request and result");
}

pub(super) fn assert_prepared_root_refuses_discovery(pic: &PocketIc, root: Principal) {
    // The ordinary Fleet fence runs before the lookup's endpoint guard and handler.
    let rejected =
        ::ic_testkit::pic::update_candid_args::<(Result<RootMembershipResponse, Error>,), _>(
            pic,
            root,
            Principal::anonymous(),
            canic::protocol::CANIC_ROOT_MEMBERSHIP,
            (RootMembershipRequest {
                subject: Principal::management_canister(),
            },),
            ::canic::testing::CANDID_REPLY_LIMITS,
        )
        .map(|(reply,)| reply)
        .expect_err("Prepared Root must keep the lookup behind its Fleet fence");
    let CandidCallError::CanisterReject { response, .. } = &rejected else {
        panic!("expected the Prepared Root Fleet fence rejection: {rejected:?}");
    };
    assert_eq!(response.error_code, ErrorCode::CanisterCalledTrap);
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
    ::ic_testkit::pic::update_candid_args(
        pic,
        root,
        caller,
        canic::protocol::CANIC_ROOT_MEMBERSHIP,
        (RootMembershipRequest { subject },),
        ::canic::testing::CANDID_REPLY_LIMITS,
    )
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
