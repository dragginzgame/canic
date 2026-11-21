//! Independent worker allowance. Ordinals are permanent within the approved pool.
use super::{Pools, STATE, api, create, placement, validate_instance};
use canic::sharding::{Allocation, Placement};
use canic_management_protocol::{CREATION_CYCLES, Refusal, children::ChildId, stored};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub(super) struct Workers {
    pub grant: stored::InstancePool,
    pub claims: Vec<WorkerClaim>,
}

#[derive(Serialize, Deserialize)]
pub(super) struct WorkerClaim {
    pub generation: [u8; 32],
    pub principal: Option<String>,
}

impl Workers {
    pub fn claim_at(&self, slot: ChildId) -> Option<&WorkerClaim> {
        self.claims.get(worker_index(slot)?)
    }

    pub fn validate(&self, pools: &Pools) {
        validate_instance(&self.grant).expect("retained worker grant");
        assert!(compatible(pools, &self.grant));
        assert!(self.claims.len() <= usize::from(self.grant.max_instances));
        assert!(self.claims.len() as u64 * CREATION_CYCLES <= self.grant.creation_budget);
        assert!(self.claims.len() + pools.projects.len() <= ChildId::INSTANCES.len());
    }
}

pub(super) fn worker_index(slot: ChildId) -> Option<usize> {
    ChildId::INSTANCES.iter().rev().position(|id| *id == slot)
}

pub(super) fn separate(first: &stored::InstancePool, other: &stored::InstancePool) -> bool {
    first.root == other.root
        && first.reference.store == other.reference.store
        && first.hub != other.hub
        && first.component != other.component
}

fn compatible(pools: &Pools, grant: &stored::InstancePool) -> bool {
    pools
        .capacity
        .as_ref()
        .is_some_and(|c| grant.max_instances <= c.max_children)
        && pools
            .instances
            .as_ref()
            .is_none_or(|other| separate(grant, other))
        && pools
            .shards
            .as_ref()
            .is_none_or(|other| super::compatible(&other.approval, grant))
}

fn installed_hub(state: &super::super::State, hub: &str) -> bool {
    ChildId::ALL.into_iter().any(|slot| {
        let child = &state.journal.0[&slot].0;
        state.pools.permanent[slot.index()]
            && child.phase == canic_management_protocol::Phase::Ready
            && child.worker.as_deref() == Some(hub)
    })
}

/// Approve an independent worker hub using the existing bounded creation contract.
/// # Errors
/// Refuses another owner, invalid release, overlapping grant or expired deadline.
/// # Panics
/// Requires Root's controller and opened placement state.
pub fn configure_workers(grant: stored::InstancePool) -> Result<(), Refusal> {
    assert!(api::is_controller(&api::caller()));
    if let Some(result) = STATE.with_borrow(|s| {
        let state = s.as_ref().unwrap();
        let pools = &state.pools;
        if let Some(workers) = &pools.workers {
            return Some(if workers.grant == grant {
                Ok(())
            } else {
                Err(Refusal::Occupied)
            });
        }
        (!installed_hub(state, &grant.hub) || !compatible(pools, &grant))
            .then_some(Err(Refusal::Binding))
    }) {
        return result;
    }
    super::admit_pool_change(false)?;
    validate_instance(&grant)?;
    if api::time() >= grant.expires_at_ns {
        return Err(Refusal::Time);
    }
    STATE.with_borrow_mut(|s| {
        let state = s.as_mut().unwrap();
        state.pools.workers = Some(Workers {
            grant,
            claims: Vec::new(),
        });
        state.pool.set(state.pools.encode());
    });
    Ok(())
}

/// Admit the actual caller bound to the worker allowance.
/// # Errors
/// Refuses any other caller.
pub fn worker_hub() -> Result<(), &'static str> {
    STATE.with_borrow(|s| {
        s.as_ref()
            .and_then(|s| s.pools.workers.as_ref())
            .filter(|w| w.grant.hub == api::caller().to_text())
            .map(|_| ())
            .ok_or("configured worker hub required")
    })
}

/// Resolve a worker ordinal, optionally reserving the next one.
/// # Panics
/// Requires the approved hub and intact allocation custody.
#[must_use]
pub fn worker(ordinal: u8, reserve: bool) -> Placement {
    worker_hub().expect("worker hub");
    let index = usize::from(ordinal);
    let Some(&slot) = ChildId::INSTANCES.iter().rev().nth(index) else {
        return Placement::Full;
    };
    let grant = match STATE.with_borrow(|s| {
        let state = s.as_ref().unwrap();
        let workers = state.pools.workers.as_ref().unwrap();
        if let Some(claim) = workers.claims.get(index) {
            return Err(placement::resolve(
                Allocation {
                    slot: slot.index() as u32,
                    generation: claim.generation,
                },
                claim.principal.as_deref(),
                state.artifact.0[&slot]
                    .sources()
                    .map(|s| s.creation.operation),
                &state.journal.0[&slot].0,
            ));
        }
        if !reserve {
            return Err(Placement::Unknown);
        }
        if ordinal >= workers.grant.max_instances || state.pools.full() {
            return Err(Placement::Full);
        }
        if index != workers.claims.len() {
            return Err(Placement::Busy);
        }
        Ok(workers.grant.clone())
    }) {
        Ok(grant) => grant,
        Err(outcome) => return outcome,
    };
    let now = api::time();
    if now >= grant.expires_at_ns
        || (u64::from(ordinal) + 1) * CREATION_CYCLES > grant.creation_budget
    {
        return Placement::GrowthRefused;
    }
    let Some(generation) = create(
        slot,
        grant.reference,
        grant.arguments,
        now,
        grant.expires_at_ns,
    ) else {
        return Placement::GrowthRefused;
    };
    STATE.with_borrow_mut(|s| {
        let state = s.as_mut().unwrap();
        state
            .pools
            .workers
            .as_mut()
            .unwrap()
            .claims
            .push(WorkerClaim {
                generation,
                principal: None,
            });
        state.pool.set(state.pools.encode());
    });
    Placement::Pending(Allocation {
        slot: slot.index() as u32,
        generation,
    })
}
