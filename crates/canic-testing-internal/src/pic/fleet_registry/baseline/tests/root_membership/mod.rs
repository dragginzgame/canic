//! Qualify canonical Root membership discovery through its guarded update.

use super::*;
use canic::dto::component_registry::{RootMembershipRequest, RootMembershipResponse};

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
        let observed = pic
            .update_candid_as::<Result<RootMembershipResponse, Error>, _>(
                root,
                caller,
                canic::protocol::CANIC_ROOT_MEMBERSHIP,
                (RootMembershipRequest {
                    subject: child.canister_id,
                },),
            )
            .expect("child membership lookup transport")
            .expect("active child lookup");
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
    let lookup = |caller, subject| {
        fixture
            .pic()
            .update_candid_as::<Result<RootMembershipResponse, Error>, _>(
                fixture.root,
                caller,
                canic::protocol::CANIC_ROOT_MEMBERSHIP,
                (RootMembershipRequest { subject },),
            )
            .expect("membership lookup transport")
    };
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
