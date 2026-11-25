//! Root quotes, charges and retains the requesting canister’s acquisition.
use super::{Acquisition, Pools, STATE, api, standby};
use canic::sharding::{
    Placement,
    payment::{Demand, Payment, Quote, Refusal},
};
use canic_management_protocol::CREATION_CYCLES;
use ic_platform_protocol::management as mgmt;
use ic_platform_types::Principal;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy)]
pub(super) struct Settlement {
    pub source: Acquisition,
    pub charge: u128,
}

impl Settlement {
    pub(super) fn origin(self) -> super::super::Origin {
        super::super::Origin::Paid(self.source, self.charge)
    }
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct Reservation {
    demand: Demand,
    payer: Principal,
    maximum: u128,
    pub(super) deadline: u64,
    pub(super) index: Option<usize>,
    quote: Quote,
    observed: bool,
}

#[derive(Serialize, Deserialize)]
pub(super) struct Receipt {
    demand: Demand,
    payer: Principal,
    maximum: u128,
    quote: Quote,
}

#[derive(Clone, Debug, PartialEq, Eq, Ord, PartialOrd, Serialize, Deserialize)]
pub(super) enum Owner {
    Pool(String),
    Project(Principal),
}

fn grant(p: &Pools, d: &Demand, caller: Principal) -> Result<(Owner, u64), Refusal> {
    let (name, owner, deadline) = match d {
        Demand::ShardCapacity(name, _) => p
            .shards
            .get(name)
            .map(|l| (&l.approval.pool, &l.approval.hub, l.approval.expires_at_ns)),
        Demand::Singleton(role) => {
            let registry = p.singleton.as_ref().ok_or(Refusal::UnknownGrant)?;
            if registry.grant.role != *role || p.project(caller).is_none() {
                return Err(Refusal::Unauthorized);
            }
            return Ok((Owner::Project(caller), registry.grant.expires_at_ns));
        }
        Demand::Instance(_) => p
            .instances
            .as_ref()
            .map(|g| (&g.component, &g.hub, g.expires_at_ns)),
        Demand::Worker(name, _) => p
            .workers
            .get(name)
            .map(|w| (&w.grant.component, &w.grant.hub, w.grant.expires_at_ns)),
    }
    .ok_or(Refusal::UnknownGrant)?;
    if owner != &caller.to_text() {
        return Err(Refusal::Unauthorized);
    }
    Ok((Owner::Pool(name.clone()), deadline))
}

pub(super) fn validate(pools: &Pools) {
    let mut held = std::collections::BTreeSet::new();
    for (name, reservation) in &pools.quotes {
        let (owner, deadline) = grant(pools, &reservation.demand, reservation.payer)
            .expect("retained quote requires its original grant");
        assert_eq!((&owner, deadline), (name, reservation.deadline));
        assert!(reservation.maximum > 0);
        let quote = &reservation.quote;
        assert_eq!(reservation.index.is_some(), quote.spare.is_some());
        if reservation.observed {
            assert!(valid_quote(quote, reservation.maximum));
        } else {
            assert!(quote.spare.is_some() && quote.working_cycles == 0);
        }
        if let Some(index) = reservation.index
            && api::time() < deadline
        {
            assert!(held.insert(index));
            let (_, asset) = pools.standby.get(index).expect("reserved asset");
            assert_eq!(standby::available(asset).map(|(p, _)| p), quote.spare);
        }
    }
    for receipt in pools.payments.values() {
        grant(pools, &receipt.demand, receipt.payer).expect("original payment authority");
        assert!(valid_quote(&receipt.quote, receipt.maximum));
    }
    for demand in pools
        .shards
        .keys()
        .map(|pool| Demand::ShardCapacity(pool.clone(), 0))
        .chain(
            pools
                .workers
                .keys()
                .map(|pool| Demand::Worker(pool.clone(), 0)),
        )
        .chain(pools.instances.iter().map(|_| Demand::Instance([0; 32])))
        .chain(
            pools
                .singleton
                .iter()
                .map(|s| Demand::Singleton(s.grant.role.clone())),
        )
    {
        remaining_budget(pools, &demand);
    }
}

fn valid_quote(quote: &Quote, maximum: u128) -> bool {
    let Some(total) = quote.creation_fee.checked_add(quote.working_cycles) else {
        return false;
    };
    total <= maximum && (quote.spare.is_some() || total == u128::from(CREATION_CYCLES))
}

fn remaining_budget(pools: &Pools, demand: &Demand) -> u128 {
    let limit = match demand {
        Demand::ShardCapacity(name, _) => pools.shards[name].approval.creation_budget,
        Demand::Worker(name, _) => pools.workers[name].grant.creation_budget,
        Demand::Instance(_) => pools.instances.as_ref().unwrap().creation_budget,
        Demand::Singleton(_) => pools.singleton.as_ref().unwrap().grant.creation_budget,
    };
    let spent = pools
        .payments
        .values()
        .filter(|receipt| match (&receipt.demand, demand) {
            (Demand::ShardCapacity(a, _), Demand::ShardCapacity(b, _))
            | (Demand::Worker(a, _), Demand::Worker(b, _))
            | (Demand::Singleton(a), Demand::Singleton(b)) => a == b,
            (Demand::Instance(_), Demand::Instance(_)) => true,
            _ => false,
        })
        .map(|r| r.quote.creation_fee + r.quote.working_cycles)
        .sum::<u128>();
    u128::from(limit)
        .checked_sub(spent)
        .expect("retained acquisition charges exceed the pool grant")
}

fn place(d: &Demand, paid: Option<Settlement>) -> Placement {
    match d {
        Demand::ShardCapacity(pool, n) => super::allocate_shard(pool, *n, paid),
        Demand::Singleton(role) => super::singleton::request(role, paid),
        Demand::Instance(key) => super::instance(*key, paid),
        Demand::Worker(pool, index) => super::workers::resolve_worker(pool, *index, true, paid),
    }
}

fn generation(p: &Placement) -> Option<[u8; 32]> {
    match p {
        Placement::Pending(a) | Placement::Assigned(a, _) | Placement::Unavailable(a, _) => {
            Some(a.generation)
        }
        _ => None,
    }
}

fn fresh_quote(fee: u128, maximum: u128) -> Result<Quote, Refusal> {
    let total = u128::from(CREATION_CYCLES);
    let working_cycles = total
        .checked_sub(fee)
        .filter(|n| *n > 0 && total <= maximum)
        .ok_or(Refusal::MaximumTooLow)?;
    Ok(Quote {
        creation_fee: fee,
        working_cycles,
        spare: None,
    })
}

/// Quote, settle or recover one allocation, always as its approved hub.
/// # Errors
/// Conflicting requests, expired grants and insufficient cycles refuse before payment.
/// # Panics
/// Traps to roll back the allocation if its exact debit cannot be accepted and retained.
pub async fn acquire_placement(demand: Demand, maximum: u128) -> Result<Payment, Refusal> {
    let payer = api::caller();
    let (pool, deadline) =
        STATE.with_borrow(|s| grant(&s.as_ref().unwrap().pools, &demand, payer))?;
    if let Demand::ShardCapacity(name, ordinal) = &demand
        && STATE.with_borrow(|s| {
            usize::from(*ordinal) > s.as_ref().unwrap().pools.shards[name].ordinals.len()
        })
    {
        return Ok(Payment::Placed(Placement::GrowthRefused, None));
    }
    let unpaid = place(&demand, None);
    if unpaid != Placement::GrowthRefused {
        let receipt = STATE.with_borrow(|s| -> Result<_, Refusal> {
            let pools = &s.as_ref().unwrap().pools;
            let Some(r) = generation(&unpaid)
                .and_then(|g| pools.payments.get(&g))
                .filter(|r| r.demand == demand)
            else {
                return Ok(None);
            };
            if r.maximum != maximum || r.payer != payer {
                return Err(Refusal::PaymentMismatch);
            }
            Ok(Some(r.quote.clone()))
        })?;
        // Another key may have opened a shared shard while this request was being quoted.
        STATE.with_borrow_mut(|s| {
            let s = s.as_mut().unwrap();
            if generation(&unpaid).is_some()
                && s.pools
                    .quotes
                    .get(&pool)
                    .is_some_and(|q| q.demand == demand)
            {
                s.pools.quotes.remove(&pool);
                s.pool.set(s.pools.encode());
            }
        });
        return Ok(Payment::Placed(unpaid, receipt));
    }
    if api::time() >= deadline {
        return Err(Refusal::GrantExpired);
    }
    if maximum == 0 {
        return Err(Refusal::ZeroMaximum);
    }
    let reserved = STATE.with_borrow_mut(|s| -> Result<_, Refusal> {
        let s = s.as_mut().unwrap();
        if let Some(q) = s.pools.quotes.get(&pool) {
            if q.demand != demand || q.payer != payer || q.maximum != maximum {
                return Err(Refusal::QuoteConflict);
            }
            return Ok(q.clone());
        }
        let fee = ic_platform_runtime::raw::cost_create_canister();
        let (index, quote) = match s.pools.acquisition().map_err(|_| Refusal::SubnetFull)? {
            Acquisition::Reserve { index, principal } => (
                Some(index),
                Quote {
                    creation_fee: fee,
                    working_cycles: 0,
                    spare: Some(principal),
                },
            ),
            Acquisition::Create
                if remaining_budget(&s.pools, &demand) >= u128::from(CREATION_CYCLES) =>
            {
                (None, fresh_quote(fee, maximum)?)
            }
            Acquisition::Create => return Err(Refusal::BudgetExhausted),
        };
        let q = Reservation {
            demand: demand.clone(),
            payer,
            maximum,
            deadline,
            index,
            quote,
            observed: index.is_none(),
        };
        s.pools.quotes.insert(pool.clone(), q.clone());
        s.pool.set(s.pools.encode());
        Ok(q)
    })?;
    if !reserved.observed {
        let id = reserved.quote.spare.expect("reserved spare");
        let args = mgmt::CanisterArgs { canister_id: id }
            .encode(1024)
            .map_err(|_| Refusal::SpareUnverified)?;
        let (status, balance) = standby::observe_target(&args)
            .await
            .map_err(|_| Refusal::SpareUnverified)?;
        if status.module_hash.is_some()
            || status.status != ic_platform_protocol::management_snapshots::Status::Running
        {
            return Err(Refusal::SpareUnverified);
        }
        return STATE.with_borrow_mut(|s| {
            let s = s.as_mut().unwrap();
            grant(&s.pools, &demand, payer)?;
            if api::time() >= deadline {
                return Err(Refusal::QuoteExpired);
            }
            if let Some(current) = s.pools.quotes.get(&pool)
                && current.observed
                && current.payer == payer
                && current.maximum == maximum
                && current.demand == demand
            {
                return Ok(Payment::Quoted(current.quote.clone()));
            }
            if s.pools.quotes.get(&pool) != Some(&reserved) {
                return Err(Refusal::QuoteExpired);
            }
            let affordable = reserved
                .quote
                .creation_fee
                .checked_add(balance)
                .is_some_and(|cost| cost <= maximum.min(remaining_budget(&s.pools, &demand)));
            let mut q = reserved;
            if affordable {
                q.quote.working_cycles = balance;
            } else {
                s.pools.quotes.remove(&pool);
                s.pool.set(s.pools.encode());
                if s.pools.full() {
                    return Err(Refusal::SubnetFull);
                }
                if remaining_budget(&s.pools, &demand) < u128::from(CREATION_CYCLES) {
                    return Err(Refusal::BudgetExhausted);
                }
                q.quote = fresh_quote(q.quote.creation_fee, maximum)?;
                q.index = None;
            }
            q.observed = true;
            let quote = q.quote.clone();
            s.pools.quotes.insert(pool, q);
            s.pool.set(s.pools.encode());
            Ok(Payment::Quoted(quote))
        });
    }
    if ic_platform_runtime::raw::msg_cycles_available() == 0 {
        return Ok(Payment::Quoted(reserved.quote));
    }
    let charge = reserved
        .quote
        .creation_fee
        .checked_add(reserved.quote.working_cycles)
        .ok_or(Refusal::ChargeOverflow)?;
    if charge > maximum || ic_platform_runtime::raw::msg_cycles_available() < charge {
        return Err(Refusal::InsufficientPayment);
    }
    if STATE.with_borrow(|s| remaining_budget(&s.as_ref().unwrap().pools, &demand)) < charge {
        return Err(Refusal::BudgetExhausted);
    }
    let source = match reserved.index {
        Some(index) => Acquisition::Reserve {
            index,
            principal: reserved.quote.spare.expect("reserved principal"),
        },
        None if STATE.with_borrow(|s| s.as_ref().unwrap().pools.full()) => {
            return Err(Refusal::SubnetFull);
        }
        None => Acquisition::Create,
    };
    let placed = place(&demand, Some(Settlement { source, charge }));
    let Placement::Pending(allocation) = &placed else {
        return Ok(Payment::Placed(placed, None));
    };
    assert_eq!(ic_platform_runtime::raw::msg_cycles_accept(charge), charge);
    STATE.with_borrow_mut(|s| {
        let s = s.as_mut().unwrap();
        assert!(s.pools.quotes.remove(&pool).is_some());
        assert!(
            s.pools
                .payments
                .insert(
                    allocation.generation,
                    Receipt {
                        demand,
                        payer,
                        maximum,
                        quote: reserved.quote.clone()
                    }
                )
                .is_none()
        );
        s.pool.set(s.pools.encode());
    });
    Ok(Payment::Placed(placed, Some(reserved.quote)))
}
