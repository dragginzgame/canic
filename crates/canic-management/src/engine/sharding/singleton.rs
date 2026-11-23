//! A ready registry instance can acquire its own child under one immutable grant.
use super::super::{Artifact, State};
use super::{STATE, api, create, delivery, payment, placement};
use canic::sharding::{Allocation, Placement};
use canic_management_protocol::application::Arguments;
use canic_management_protocol::{Phase, Refusal, children::ChildId, stored};
use ic_platform_types::Principal;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Serialize, Deserialize)]
pub(super) struct Registry {
    pub(super) grant: stored::SingletonGrant,
    // Original parent allocation generation maps to its child's custody and generation.
    children: BTreeMap<[u8; 32], (ChildId, [u8; 32])>,
}

impl Registry {
    pub(super) fn slots(&self) -> impl Iterator<Item = ChildId> {
        self.children.values().map(|(slot, _)| *slot)
    }
}

impl super::Pools {
    pub(super) fn project(&self, caller: Principal) -> Option<(usize, &super::ProjectClaim)> {
        let caller = caller.to_text();
        self.projects
            .iter()
            .enumerate()
            .find(|(_, claim)| claim.principal.as_ref() == Some(&caller))
    }

    pub(in crate::engine) fn singleton_owns(&self, binding: &canic::topology::Install) -> bool {
        self.singleton
            .as_ref()
            .is_some_and(|r| r.grant.role == binding.role)
            && self.project(binding.parent).is_some()
    }
}

fn arguments(grant: &stored::SingletonGrant, parent: Principal) -> Result<Arguments, Refusal> {
    let bytes = canic::topology::Install {
        root: api::canister_self(),
        parent,
        role: grant.role.clone(),
        application: grant.application.as_slice().to_vec(),
    }
    .encode()
    .map_err(|_| Refusal::Binding)?;
    Arguments::new(bytes).map_err(|_| Refusal::Binding)
}

/// Approve an exact release and payer for one child per registry instance.
/// # Errors
/// Rejects a conflicting approval, unknown registry or invalid limits.
/// # Panics
/// Requires Root controller authority.
pub fn configure_singleton(grant: stored::SingletonGrant) -> Result<(), Refusal> {
    assert!(api::is_controller(&api::caller()), "controller required");
    super::admit_pool_change(false)?;
    STATE.with_borrow_mut(|cell| {
        let state = cell.as_mut().expect("open Root");
        let pools = &mut state.pools;
        if let Some(r) = &pools.singleton {
            return if r.grant == grant {
                Ok(())
            } else {
                Err(Refusal::Occupied)
            };
        }
        let registry = pools.instances.as_ref().ok_or(Refusal::Binding)?;
        if registry.hub != grant.registry
            || registry.reference.store != grant.reference.store
            || grant.maximum == 0
            || pools
                .capacity
                .as_ref()
                .is_none_or(|c| grant.maximum > c.max_children)
            || grant.role.is_empty()
            || grant.role.len() > 64
        {
            return Err(Refusal::Binding);
        }
        if api::time() >= grant.expires_at_ns {
            return Err(Refusal::Time);
        }
        if state.artifact.0.values().any(|artifact| {
            let Artifact::Application(_, lifecycle) = artifact else {
                return false;
            };
            canic::topology::Install::approved(lifecycle.creation.as_slice())
                .ok()
                .flatten()
                .is_some_and(|binding| {
                    binding.role == grant.role && pools.project(binding.parent).is_some()
                })
        }) {
            return Err(Refusal::Occupied);
        }
        arguments(
            &grant,
            Principal::from_text(&grant.registry).map_err(|_| Refusal::Binding)?,
        )?;
        delivery::admit(&grant.reference, &grant.reference.manifest.hash)?;
        pools.singleton = Some(Registry {
            grant,
            children: BTreeMap::new(),
        });
        state.pool.set(pools.encode());
        Ok(())
    })
}

/// Reserve or recover the calling project's child.
/// # Panics
/// Requires an opened Root.
#[must_use]
pub fn request_singleton(role: &str) -> Placement {
    request(role, None)
}

pub(super) fn request(role: &str, paid: Option<payment::Settlement>) -> Placement {
    let caller = api::caller();
    let selected = STATE.with_borrow(|cell| {
        let state = cell.as_ref().unwrap();
        let pools = &state.pools;
        let r = pools
            .singleton
            .as_ref()
            .filter(|r| r.grant.role == role)
            .ok_or(Placement::GrowthRefused)?;
        let (index, parent) = pools.project(caller).ok_or(Placement::GrowthRefused)?;
        let parent_slot = ChildId::INSTANCES[index];
        let parent_journal = &state.journal.0[&parent_slot].0;
        if parent_journal.phase != Phase::Ready
            || parent_journal.worker != parent.principal
            || state.artifact.0[&parent_slot]
                .sources()
                .map(|s| s.creation.operation)
                != Some(parent.generation)
        {
            return Err(Placement::GrowthRefused);
        }
        if let Some(&(slot, generation)) = r.children.get(&parent.generation) {
            let journal = &state.journal.0[&slot].0;
            return Err(placement::resolve(
                Allocation {
                    slot: slot.index() as u32,
                    generation,
                },
                journal.worker.as_deref(),
                state.artifact.0[&slot]
                    .sources()
                    .map(|s| s.creation.operation),
                journal,
            ));
        }
        if r.children.len() >= usize::from(r.grant.maximum) {
            return Err(Placement::Full);
        }
        if api::time() >= r.grant.expires_at_ns {
            return Err(Placement::GrowthRefused);
        }
        let slot = ChildId::ALL
            .into_iter()
            .rev()
            .find(|s| state.journal.0[s].0.phase == Phase::Empty)
            .ok_or(Placement::Full)?;
        let origin = pools.admit_growth(
            r.slots(),
            r.grant.creation_budget,
            r.grant.creation_payer,
            paid,
        )?;
        Ok((r.grant.clone(), slot, parent.generation, origin))
    });
    let (grant, slot, parent, origin) = match selected {
        Ok(v) => v,
        Err(outcome) => return outcome,
    };
    let Ok(args) = arguments(&grant, caller) else {
        return Placement::GrowthRefused;
    };
    let Some(generation) = create(
        origin,
        slot,
        grant.reference,
        args,
        api::time(),
        grant.expires_at_ns,
    ) else {
        return Placement::GrowthRefused;
    };
    STATE.with_borrow_mut(|cell| {
        let state = cell.as_mut().unwrap();
        state.pools.permanent[slot.service_index().unwrap()] = true;
        assert!(
            state
                .pools
                .singleton
                .as_mut()
                .unwrap()
                .children
                .insert(parent, (slot, generation))
                .is_none()
        );
        state.pool.set(state.pools.encode());
    });
    Placement::Pending(Allocation {
        slot: slot.index() as u32,
        generation,
    })
}

pub(super) fn validate(state: &State) {
    let Some(r) = &state.pools.singleton else {
        return;
    };
    assert_eq!(
        r.grant.registry,
        state.pools.instances.as_ref().expect("registry grant").hub
    );
    assert!(r.children.len() <= usize::from(r.grant.maximum));
    assert!(state.pools.creation_debit(r.slots()) <= r.grant.creation_budget);
    let mut slots = std::collections::BTreeSet::new();
    for (parent, &(slot, generation)) in &r.children {
        let id = state
            .pools
            .projects
            .iter()
            .find(|p| p.generation == *parent)
            .and_then(|p| p.principal.as_deref())
            .expect("installed project");
        assert!(slots.insert(slot) && state.pools.permanent[slot.service_index().unwrap()]);
        let artifact = &state.artifact.0[&slot];
        let source = &artifact.sources().expect("singleton release").creation;
        assert_eq!(source.operation, generation);
        assert_eq!(source.reference, r.grant.reference);
        let Artifact::Application(_, lifecycle) = artifact else {
            panic!("singleton install arguments")
        };
        assert_eq!(
            lifecycle.creation,
            arguments(&r.grant, Principal::from_text(id).unwrap()).unwrap()
        );
    }
}
