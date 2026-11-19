//! Installed hub coordination. Stable writes occur only inside synchronous store scopes.
use super::{
    Account, Binding, Data, HubRoute, Progress, Provision, ShardRecord, Ticket,
    rows::{accept, confirm, find, reserve, ticket},
};
use crate::{
    auth::{protocol::SessionView, session},
    db::role,
    method,
    sharding::{self, Allocation, Placement},
    topology,
};

use ic_platform_runtime::system as api;
use ic_platform_types::Principal;

fn target(placement: &Placement, root: Principal) -> Result<(Allocation, Principal), Progress> {
    match placement {
        Placement::Pending(_) | Placement::Busy => Err(Progress::PlacementPending),
        Placement::Full => Err(Progress::PoolFull),
        _ => placement
            .assigned_target()
            .ok()
            .flatten()
            .filter(|(_, p)| *p != root && *p != api::canister_self())
            .map(|(allocation, p)| (allocation.clone(), p))
            .ok_or(Progress::PlacementUnavailable),
    }
}

async fn still_assigned(
    root: Principal,
    key: sharding::Key,
    allocation: &Allocation,
    shard: Principal,
) -> bool {
    sharding::lookup(root, key)
        .await
        .is_ok_and(|p| super::matches_target(&p, allocation, shard))
}

fn admit<E: HubRoute>(view: &SessionView) {
    session::reauthorize(view, E::SCOPE).unwrap_or_else(|e| method::fail(e));
}

/// Perform one bounded attempt. A retry reads the winning reservation and the
/// immutable Root allocation. A completed signup needs no network call.
/// # Panics
/// Traps on invalid application data, expired authority, storage errors or conflicting bindings.
pub async fn provision<E: HubRoute>(
    view: &SessionView,
    input: Data<E>,
    new_id: impl FnOnce() -> super::Id<E>,
) -> Progress {
    // Capture ingress authority before any suspension; callers cannot supply another user's view.
    assert_eq!(api::caller(), view.caller, "session caller mismatch");
    admit::<E>(view);
    let account = Account::from(view);
    assert!(account.valid(), "invalid provisioning account");
    let input = E::normalize(input);
    let existing = E::with_store(|db| find::<E::Row>(db, account));
    if existing.as_ref().is_some_and(E::ready) {
        return Progress::Ready;
    }
    let root = role::root().expect("installed provisioning hub");
    let key = super::placement_key::<E::Resource>(account);
    let Ok(mut placement) = sharding::lookup(root, key).await else {
        return Progress::PlacementUnavailable;
    };
    if existing.is_none() && placement == Placement::Unknown {
        if let Some(winner) = E::with_store(|db| find::<E::Row>(db, account)) {
            return if E::ready(&winner) {
                Progress::Ready
            } else {
                Progress::PlacementUnavailable
            };
        }
        admit::<E>(view);
        let Ok(assigned) = sharding::assign(root, key).await else {
            return Progress::PlacementUnavailable;
        };
        placement = assigned;
    }
    let (allocation, shard) = match target(&placement, root) {
        Ok(target) => target,
        Err(progress) => return progress,
    };
    let row = if let Some(row) = existing {
        row
    } else {
        admit::<E>(view);
        let id = new_id();
        if !still_assigned(root, key, &allocation, shard).await {
            return Progress::PlacementUnavailable;
        }
        admit::<E>(view);
        E::with_store(|db| reserve::<E>(db, &Binding { id, account, shard }, input))
    };
    let request = ticket::<E>(&row);
    let expected = request.binding.clone();
    if expected.shard != shard || expected.account != account {
        return Progress::PlacementUnavailable;
    }
    if E::ready(&row) {
        return Progress::Ready;
    }
    admit::<E>(view);
    match method::call::<Provision<E::Resource>>(shard, 10, &request).await {
        Ok(receipt) => assert!(receipt == expected, "conflicting provisioning receipt"),
        Err(method::CallError::Request(e)) => method::fail(e),
        Err(_) => return Progress::ProvisionPending,
    }
    // Reconciliation of the original effect, never authority to start another.
    if !still_assigned(root, key, &allocation, shard).await {
        return Progress::PlacementUnavailable;
    }
    E::with_store(|db| confirm::<E>(db, &expected));
    Progress::Ready
}

/// Synchronously provision or acknowledge the exact original private row.
/// Mount with `guard = canic::topology::require_parent` for pre-decode admission.
/// # Panics
/// Refuses a foreign caller, wrong target, ID conflict or invalid initial data.
#[allow(clippy::needless_pass_by_value)]
pub fn receive<E: ShardRecord>(request: Ticket<E>) -> Binding<super::Id<E>> {
    topology::require_parent().unwrap_or_else(|e| method::fail(e));
    let expected = request.binding.clone();
    if !expected.account.valid() || expected.shard != api::canister_self() {
        method::fail("BindingConflict");
    }
    if !E::with_store(|db| accept::<E>(db, request)) {
        method::fail("BindingConflict");
    }
    expected
}
