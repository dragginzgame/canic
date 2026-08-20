//! Module: access::auth::identity
//!
//! Responsibility: validate whether a principal may act as a local application subject.
//! Does not own: session resolution, token verification, endpoint predicates, or policy.
//! Boundary: explicit application authorization checks this current topology fact.

use super::ApplicationSubjectRejection;
use crate::{
    cdk::types::Principal,
    ops::{runtime::env::EnvOps, storage::children::CanisterChildrenOps},
};

/// Reject obvious canister and infrastructure identities for local application sessions.
pub(super) fn validate_application_subject(
    subject: Principal,
) -> Result<(), ApplicationSubjectRejection> {
    if subject == Principal::anonymous() {
        return Err(ApplicationSubjectRejection::Anonymous);
    }

    if subject == Principal::management_canister() {
        return Err(ApplicationSubjectRejection::ManagementCanister);
    }

    if try_canister_self().is_some_and(|pid| pid == subject) {
        return Err(ApplicationSubjectRejection::LocalCanister);
    }

    let env = EnvOps::snapshot();
    if env.record.root_pid.is_some_and(|pid| pid == subject) {
        return Err(ApplicationSubjectRejection::RootCanister);
    }
    if env.record.parent_pid.is_some_and(|pid| pid == subject) {
        return Err(ApplicationSubjectRejection::ParentCanister);
    }
    if env.record.subnet_pid.is_some_and(|pid| pid == subject) {
        return Err(ApplicationSubjectRejection::SubnetCanister);
    }
    if env
        .record
        .fleet_subnet_root_pid
        .is_some_and(|pid| pid == subject)
    {
        return Err(ApplicationSubjectRejection::FleetSubnetRootCanister);
    }
    if CanisterChildrenOps::contains_pid(&subject) {
        return Err(ApplicationSubjectRejection::DirectChildCanister);
    }

    Ok(())
}

#[cfg(target_arch = "wasm32")]
#[expect(clippy::unnecessary_wraps)]
fn try_canister_self() -> Option<Principal> {
    Some(ic_cdk::api::canister_self())
}

#[cfg(not(target_arch = "wasm32"))]
const fn try_canister_self() -> Option<Principal> {
    None
}
