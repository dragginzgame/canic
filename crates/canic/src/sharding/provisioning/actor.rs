//! Placement and dispatch under the original authenticated account.
use super::{AuthSubject, Binding, Progress, Provision, Request, Resource, SendError};
use crate::{
    auth::session::Authorization,
    guard, method, role,
    sharding::{self, Placement, PlacementOutcome, ShardTarget},
};
use ic_platform_runtime::system as api;
use ic_platform_types::Principal;
use std::marker::PhantomData;

fn target(placement: Placement, root: Principal) -> Result<ShardTarget, Progress> {
    match placement
        .into_outcome()
        .map_err(|_| Progress::PlacementUnavailable)?
    {
        PlacementOutcome::Pending(_) | PlacementOutcome::Busy => Err(Progress::PlacementPending),
        PlacementOutcome::Full => Err(Progress::PoolFull),
        PlacementOutcome::Assigned(allocation, shard)
            if shard != root && shard != api::canister_self() =>
        {
            Ok(ShardTarget { allocation, shard })
        }
        _ => Err(Progress::PlacementUnavailable),
    }
}

fn admit(auth: &Authorization) {
    auth.reauthorize().unwrap_or_else(|e| method::fail(e));
}

/// One verified allocation for this signup attempt. The application retains its own row.
pub struct Assignment<'a, R> {
    auth: &'a Authorization,
    pool: &'a str,
    subject: Principal,
    target: ShardTarget,
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
    // This local index key includes the resource and issuer as well as the subject.
    let subject = Principal::self_authenticating(super::placement_key::<R>(account));
    let target = if retained.is_some() {
        sharding::routes::get_shard(pool, subject).ok_or(Progress::PlacementUnavailable)?
    } else {
        let placement = sharding::routes::assign_shard(pool, subject, || {
            admit(auth);
            Ok::<(), std::convert::Infallible>(())
        })
        .await
        .map_err(|_| Progress::PlacementUnavailable)?;
        target(placement, root)?
    };
    if retained.is_some_and(|p| p != target.shard) {
        return Err(Progress::PlacementUnavailable);
    }

    Ok(Assignment {
        auth,
        pool,
        subject,
        target,
        resource: PhantomData,
    })
}

impl<R: Resource> Assignment<'_, R> {
    /// Principal to persist when inserting a new reservation.
    #[must_use]
    pub fn shard(&self) -> Principal {
        self.target.shard
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
        if expected.account != AuthSubject::from(self.auth) || expected.shard != self.target.shard {
            return Err(SendError::Progress(Progress::PlacementUnavailable));
        }
        admit(self.auth);
        self.verify().map_err(SendError::Progress)?;
        let result = method::call::<Provision<R>>(self.target.shard, 10, request).await;
        admit(self.auth);
        let receipt = match result {
            Ok(reply) => reply,
            Err(method::CallError::Request(e)) => method::fail(e),
            Err(_) => return Err(SendError::Progress(Progress::ProvisionPending)),
        };
        if let Ok(binding) = &receipt {
            binding.require_same(expected);
        }
        self.verify().map_err(SendError::Progress)?;

        receipt.map_err(SendError::Refused)
    }

    fn verify(&self) -> Result<(), Progress> {
        verify_target(
            sharding::routes::get_shard(self.pool, self.subject).as_ref(),
            &self.target,
        )
    }
}

pub(super) fn verify_target(
    current: Option<&ShardTarget>,
    expected: &ShardTarget,
) -> Result<(), Progress> {
    if current == Some(expected) {
        Ok(())
    } else {
        Err(Progress::PlacementUnavailable)
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
