//! Stable pool formats and the custody checks performed when Root reopens them.
use super::{
    Pools, api, capacity, check_shard_topology, compatible, standby, valid_limits,
    validate_instance,
};
use canic::sharding::standby::Source;
use canic_management_protocol::{CREATION_CYCLES, children::ChildId, stored};
use ic_platform_types::Principal;

pub(super) fn validate(pools: &Pools) {
    for (&slot, generation) in &pools.draining {
        let ledger = pools.shard_at(slot).expect("draining pool");
        assert!(
            ChildId::SHARDS.contains(&slot)
                && ledger.generations[slot.index()].as_ref() == Some(generation)
                && ledger.principals[slot.index()].is_some()
        );
    }
    if !pools.permanent.is_empty() {
        pools
            .capacity
            .as_ref()
            .expect("permanent children require subnet capacity");
    }
    if let Some(capacity) = &pools.capacity {
        assert!(
            capacity::valid_capacity(capacity)
                && pools.occupied_slots() <= capacity.max_children as usize
        );
    }
    for (initial, increases, family_limit) in std::iter::once((
        pools.instances.as_ref().map(|a| a.max_instances),
        &pools.instance_increases,
        stored::MAX_INSTANCES,
    ))
    .chain(pools.shards.values().map(|l| {
        (
            Some(l.approval.max_shards),
            &l.increases,
            stored::MAX_SHARDS,
        )
    })) {
        if !increases.is_empty() {
            let cap = pools
                .capacity
                .as_ref()
                .expect("approved subnet ceiling")
                .max_children;
            let original = initial.expect("original pool grant");
            assert!(
                original < increases[0]
                    && increases.windows(2).all(|w| w[0] < w[1])
                    && increases
                        .last()
                        .is_some_and(|&count| count <= family_limit && u32::from(count) <= cap)
            );
        }
    }
    if let Some(grant) = &pools.instances {
        assert!(grant.max_instances <= pools.pool_limit());
        validate_instance(grant).expect("retained instance grant");
        assert!(pools.projects.len() <= pools.instance_slots(grant).len());
        for shared in pools.shards.values() {
            assert!(
                shared.approval.hub != grant.hub && compatible(&shared.approval, grant),
                "overlapping placement grants"
            );
        }
    }
    let mut owned = std::collections::BTreeSet::new();
    for (name, l) in &pools.shards {
        let ordinals = l
            .ordinals
            .iter()
            .copied()
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(ordinals.len(), l.ordinals.len());
        assert_eq!(ordinals, l.allocated_slots().collect());
        assert_eq!(name, &l.approval.pool);
        for slot in l.allocated_slots() {
            assert!(owned.insert(slot));
        }
        assert!(valid_limits(&l.approval) && l.approval.max_shards <= pools.pool_limit());
        check_shard_topology(&l.approval).expect("retained shard topology");
        for (index, principal) in l.principals.iter().enumerate() {
            assert!(principal.is_none() || l.generations[index].is_some());
        }
    }
    pools.receipts.validate(|child| {
        pools
            .funding
            .values()
            .find(|grant| grant.rule.child == child)
            .map(super::funding::Grant::settled)
    });

    assert!(
        pools
            .evacuated
            .iter()
            .all(|slot| pools.draining.contains_key(slot) && pools.shard_at(*slot).is_some())
    );
    validate_inventory(pools);
    super::payment::validate(pools);
    for (slot, grant) in &pools.funding {
        assert!(slot.service_index().is_some() && grant.valid());
    }
    let mut claimed = std::collections::BTreeSet::new();
    assert!(pools.workers.len() <= usize::from(pools.pool_limit()));
    for (name, pool) in &pools.workers {
        assert_eq!(name, &pool.grant.component);
        pool.validate(pools);
        for claim in pool.claims.iter().filter(|claim| !claim.returned) {
            assert!(
                ChildId::INSTANCES[pools.projects.len()..].contains(&claim.slot)
                    && claimed.insert(claim.slot)
            );
        }
    }
}

fn worker_claim<'a>(
    pools: &'a Pools,
    target: &canic::sharding::ShardTarget,
) -> (&'a super::workers::Workers, &'a super::workers::WorkerClaim) {
    pools
        .workers
        .values()
        .find_map(|pool| {
            pool.claims
                .iter()
                .find(|c| {
                    c.slot.index() as u32 == target.allocation.slot
                        && c.generation == target.allocation.generation
                })
                .map(|c| (pool, c))
        })
        .expect("original worker claim")
}

fn validate_inventory(pools: &Pools) {
    let standby = &pools.standby;
    let returns = &pools.returns;
    if pools.capacity.is_none() {
        assert!(
            standby.is_empty()
                && returns.is_empty()
                && pools.retirements.is_empty()
                && pools.workers.is_empty()
                && pools.reserve_policy.is_none()
        );
    }
    // Each acquisition origin has its own lifetime bound. Exported rows remain receipts.
    assert!(standby.len() <= 5 * usize::from(stored::MAX_STANDBY_POOL_ENTRIES));
    let mut refill_sequence = 0;
    let mut principals = std::collections::BTreeSet::new();
    let mut slots = std::collections::BTreeSet::new();
    for (origin, asset) in standby {
        use canic::sharding::standby::Asset;
        let principal = match asset {
            Asset::Issued => None,
            Asset::Transfer(id, destination, deadline, _) => {
                assert!(
                    *deadline > 0
                        && destination != id
                        && *destination != api::canister_self()
                        && destination.as_slice().last() == Some(&1)
                );
                Some(id)
            }
            Asset::Empty(id)
            | Asset::AwaitingRelease(id)
            | Asset::AwaitingHandoff(id)
            | Asset::Quarantined(id, _) => Some(id),
            Asset::Available(id, balance) => {
                if matches!(origin, Source::Imported(_))
                    && !returns.iter().any(|r| r.target.shard == *id)
                {
                    assert!(
                        *balance
                            >= u128::from(
                                pools
                                    .reserve_policy
                                    .as_ref()
                                    .unwrap()
                                    .minimum_import_cycles
                                    .get()
                            )
                    );
                } else {
                    let last = returns
                        .iter()
                        .rev()
                        .find(|r| r.target.shard == *id)
                        .expect("returned asset");
                    assert_eq!(*balance, last.balance);
                }
                Some(id)
            }
            Asset::Claimed(id, allocation)
            | Asset::Released(id, allocation)
            | Asset::Reset(id, allocation, ..) => {
                let slot = ChildId::from_raw(allocation.slot);
                assert!(slots.insert(slot));
                if matches!(asset, Asset::Released(..) | Asset::Reset(..)) {
                    let (_, claim) = pools.worker_at(slot).expect("released worker claim");
                    assert_eq!(claim.generation, allocation.generation);
                    assert_eq!(claim.principal.as_deref(), Some(id.to_text().as_str()));
                }
                Some(id)
            }
        };
        if let Some(id) = principal {
            assert!(
                principals.insert(*id)
                    && *id != api::canister_self()
                    && *id != Principal::anonymous()
                    && *id != Principal::management_canister()
            );
        }
        match origin {
            Source::Refill(approval) => {
                assert!(
                    approval.root == api::canister_self()
                        && approval.sequence == refill_sequence
                        && approval.cycles == CREATION_CYCLES
                );
                refill_sequence += 1;
            }
            Source::Imported(id) => {
                assert_eq!(principal, Some(id));
                assert!(
                    pools
                        .reserve_policy
                        .as_ref()
                        .is_some_and(|p| p.imports.iter().any(|i| i.principal == id.to_text()))
                );
            }
            Source::Automatic(a) => assert!(
                pools
                    .reserve_policy
                    .as_ref()
                    .is_some_and(|p| a.root == api::canister_self()
                        && a.expires_at_ns == p.expires_at_ns
                        && a.cycles == CREATION_CYCLES)
            ),
            Source::Worker(target) => {
                assert_eq!(principal, Some(&target.shard));

                let (workers, claim) = worker_claim(pools, target);
                workers
                    .minimum_reuse_cycles()
                    .expect("worker recycling approved");
                assert_eq!(claim.generation, target.allocation.generation);
                assert_eq!(
                    claim.principal.as_deref(),
                    Some(target.shard.to_text().as_str())
                );
                assert!(
                    standby::released_binding(asset) == Some((target.shard, &target.allocation))
                        || returns.iter().any(|r| r.target == *target),
                    "worker return custody"
                );
            }
            Source::Shard(target) => {
                assert_eq!(principal, Some(&target.shard));
                let retirement = &pools.retirements
                    [&ChildId::placement(target.allocation.slot as usize).expect("source slot")];
                assert_eq!(retirement.target(), *target);
                assert_eq!(retirement.phase, stored::RetirementPhase::Returned);
            }
        }
    }
    for receipt in returns {
        let target = &receipt.target;
        let slot = ChildId::placement(target.allocation.slot as usize).expect("returned slot");
        if let Some(r) = pools.retirements.get(&slot) {
            assert_eq!(r.target(), *target);
            assert_eq!(r.phase, stored::RetirementPhase::Returned);
            assert!(principals.contains(&target.shard) && slots.insert(slot));
            assert_eq!(r.input.approval.expires_at_ns, receipt.deadline);
            assert!(receipt.balance >= u128::from(r.input.approval.minimum_reuse_cycles.get()));
            continue;
        }
        let (workers, claim) = worker_claim(pools, target);
        assert!(claim.returned);
        assert!(principals.contains(&target.shard));
        assert_eq!(claim.generation, target.allocation.generation);
        assert_eq!(
            claim.principal.as_deref(),
            Some(target.shard.to_text().as_str())
        );
        assert!(
            receipt.balance
                >= u128::from(workers.minimum_reuse_cycles().expect("return grant").get())
        );
    }
    for (&slot, r) in &pools.retirements {
        let target = r.target();
        let ledger = pools
            .shard_at(slot)
            .expect("retirement requires original shard pool");
        assert!(
            ChildId::SHARDS.contains(&slot)
                && slot.index() as u32 == target.allocation.slot
                && pools.draining.get(&slot) == Some(&target.allocation.generation)
                && ledger.principals[slot.index()].as_deref()
                    == Some(r.input.approval.child.as_str())
                && pools.evacuated.contains(&slot)
                && r.input.approval.root == api::canister_self().to_text()
                && r.input.reference.store == ledger.approval.reference.store
                && r.input.reference.manifest.valid()
                && r.proof_deadline < r.input.approval.expires_at_ns,
            "retained shard retirement binding"
        );
        let returned = r.phase == stored::RetirementPhase::Returned;
        assert_eq!(
            returned,
            returns.iter().any(|receipt| receipt.target == target)
        );
    }
    if let Some(policy) = &pools.reserve_policy {
        standby::policy::validate(policy, pools.capacity.as_ref().unwrap().max_children)
            .expect("retained policy");
        let mut automatic = 0;
        for (source, asset) in standby {
            if let Source::Automatic(a) = source {
                assert_eq!(a.sequence, automatic);
                automatic += 1;
            }
            if matches!(
                asset,
                canic::sharding::standby::Asset::AwaitingRelease(_)
                    | canic::sharding::standby::Asset::AwaitingHandoff(_)
                    | canic::sharding::standby::Asset::Quarantined(..)
            ) {
                assert!(matches!(source, Source::Imported(_)));
            }
        }
        assert!(automatic <= policy.maximum_creations);
        assert_eq!(
            policy.imports.len(),
            standby
                .iter()
                .filter(|(s, _)| matches!(s, Source::Imported(_)))
                .count()
        );
    }
}
