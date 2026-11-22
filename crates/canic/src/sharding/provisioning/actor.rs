//! Placement and dispatch under the original authenticated account.
use super::{AuthSubject, Binding, Progress, Provision, Request, Resource, SendError};
use crate::{
    auth::session::Authorization,
    guard, method, role,
    sharding::{self, Allocation, Placement},
};
use ic_platform_runtime::system as api;
use ic_platform_types::Principal;
use std::marker::PhantomData;

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

fn admit(auth: &Authorization) {
    auth.reauthorize().unwrap_or_else(|e| method::fail(e));
}

/// One verified allocation for this signup attempt. The application retains its own row.
pub struct Assignment<'a, R> {
    auth: &'a Authorization,
    root: Principal,
    pool: &'a str,
    key: sharding::Key,
    allocation: Allocation,
    shard: Principal,
    resource: PhantomData<R>,
}

/// Resolve the original shard. A retained reservation permits lookup only.
/// # Errors
/// Returns the pool's pending, full or unavailable outcome.
/// # Panics
/// Traps on a foreign caller, invalid account or expired scope.
pub async fn place<'a, R: Resource>(
    auth: &'a Authorization,
    pool: &'a str,
    retained: Option<Principal>,
) -> Result<Assignment<'a, R>, Progress> {
    admit(auth);
    let account = AuthSubject::from(auth);
    assert!(account.valid(), "invalid provisioning account");
    let root = role::root().expect("installed provisioning hub");
    let key = super::placement_key::<R>(account);
    // Root's assign first looks up the original key in the same execution.
    let result = if retained.is_some() {
        sharding::lookup(root, pool, key).await
    } else {
        sharding::assign(root, pool, key).await
    };
    admit(auth);
    let (allocation, shard) = target(&result.map_err(|_| Progress::PlacementUnavailable)?, root)?;
    if retained.is_some_and(|p| p != shard) {
        return Err(Progress::PlacementUnavailable);
    }

    Ok(Assignment {
        auth,
        root,
        pool,
        key,
        allocation,
        shard,
        resource: PhantomData,
    })
}

impl<R: Resource> Assignment<'_, R> {
    /// Principal to persist when inserting a new reservation.
    #[must_use]
    pub fn shard(&self) -> Principal {
        self.shard
    }

    /// Send the application's retained request, then verify the reply and original allocation.
    /// # Errors
    /// An admitted shard refusal carries the application error.
    /// An uncertain shard reply leaves provisioning pending. A changed allocation is unavailable.
    /// # Panics
    /// Refuses expired authority, invalid request encoding or a conflicting acknowledgement.
    pub async fn send(
        self,
        request: &Request<R::Id, R::Data>,
    ) -> Result<Binding<R::Id>, SendError<R::Refusal>> {
        let expected = &request.binding;
        if expected.account != AuthSubject::from(self.auth) || expected.shard != self.shard {
            return Err(SendError::Progress(Progress::PlacementUnavailable));
        }
        admit(self.auth);
        self.verify().await.map_err(SendError::Progress)?;
        let result = method::call::<Provision<R>>(self.shard, 10, request).await;
        admit(self.auth);
        let receipt = match result {
            Ok(reply) => reply,
            Err(method::CallError::Request(e)) => method::fail(e),
            Err(_) => return Err(SendError::Progress(Progress::ProvisionPending)),
        };
        if let Ok(binding) = &receipt {
            binding.require_same(expected);
        }
        self.verify().await.map_err(SendError::Progress)?;

        receipt.map_err(SendError::Refused)
    }

    async fn verify(&self) -> Result<(), Progress> {
        let observation = sharding::lookup(self.root, self.pool, self.key).await;
        admit(self.auth);
        if !observation.is_ok_and(|p| super::matches_target(&p, &self.allocation, self.shard)) {
            return Err(Progress::PlacementUnavailable);
        }

        Ok(())
    }
}

impl<I> Binding<I> {
    /// Admit a shard request from the installed parent for this exact canister.
    /// # Panics
    /// Traps on a foreign parent.
    /// # Errors
    /// Refuses an anonymous account component or wrong destination.
    pub fn check_target(&self) -> Result<(), &'static str> {
        guard::require_parent().unwrap_or_else(|e| method::fail(e));
        if !self.account.valid() || self.shard != api::canister_self() {
            return Err("invalid provisioning target");
        }

        Ok(())
    }
}
