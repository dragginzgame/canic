//! Requester-paid growth uses the same Root allocation and installation path as sponsored growth.
use super::{Acquisition, Pools, STATE, api, standby};
use canic::sharding::{
    Placement,
    payment::{Demand, Payment, Quote},
};
use canic_management_protocol::{CREATION_CYCLES, stored::CreationPayer};
use ic_platform_protocol::management as mgmt;
use ic_platform_types::Principal;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy)]
pub(super) struct Settlement {
    pub source: Acquisition,
    pub charge: u128,
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

fn grant(p: &Pools, d: &Demand, caller: Principal) -> Result<(String, u64), String> {
    if p.capacity.is_none() {
        return Err("requester-funded growth requires an approved subnet limit".into());
    }
    let (name, owner, payer, deadline) = match d {
        Demand::Shard(name, _) => p.shards.get(name).map(|l| {
            (
                &l.approval.pool,
                &l.approval.hub,
                l.approval.creation_payer,
                l.approval.expires_at_ns,
            )
        }),
        Demand::Instance(_) => p
            .instances
            .as_ref()
            .map(|g| (&g.component, &g.hub, g.creation_payer, g.expires_at_ns)),
        Demand::Worker(name, _) => p.workers.get(name).map(|w| {
            (
                &w.grant.component,
                &w.grant.hub,
                w.grant.creation_payer,
                w.grant.expires_at_ns,
            )
        }),
    }
    .ok_or("unknown placement pool")?;
    if owner != &caller.to_text() || payer != CreationPayer::Requester {
        return Err("requester-funded grant for the calling hub required".into());
    }
    Ok((name.clone(), deadline))
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
}

fn valid_quote(quote: &Quote, maximum: u128) -> bool {
    let Some(total) = quote.creation_fee.checked_add(quote.working_cycles) else {
        return false;
    };
    total <= maximum && (quote.spare.is_some() || total == u128::from(CREATION_CYCLES))
}

fn fresh_budget(pools: &Pools, demand: &Demand) -> bool {
    let (budget, spent) = match demand {
        Demand::Shard(pool, _) => {
            let ledger = &pools.shards[pool];
            let slots = pools
                .shard_slots(&ledger.approval)
                .into_iter()
                .filter(|slot| ledger.generations[slot.index()].is_some());
            (ledger.approval.creation_budget, pools.creation_debit(slots))
        }
        Demand::Instance(_) => {
            let grant = pools.instances.as_ref().expect("approved registry");
            let slots = &pools.instance_slots(grant)[..pools.projects.len()];
            (
                grant.creation_budget,
                pools.creation_debit(slots.iter().copied()),
            )
        }
        Demand::Worker(pool, _) => {
            let workers = &pools.workers[pool];
            (workers.grant.creation_budget, workers.debit())
        }
    };
    budget.saturating_sub(spent) >= CREATION_CYCLES
}

fn place(d: &Demand, paid: Option<Settlement>) -> Placement {
    match d {
        Demand::Shard(pool, key) => super::assign_shard(pool, *key, paid),
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

fn fresh_quote(fee: u128, maximum: u128) -> Result<Quote, String> {
    let total = u128::from(CREATION_CYCLES);
    let working_cycles = total
        .checked_sub(fee)
        .filter(|n| *n > 0 && total <= maximum)
        .ok_or("approved maximum cannot fund the fresh create and its current fee")?;
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
pub async fn fund(demand: Demand, maximum: u128) -> Result<Payment, String> {
    let payer = api::caller();
    let (pool, deadline) =
        STATE.with_borrow(|s| grant(&s.as_ref().unwrap().pools, &demand, payer))?;
    let unpaid = place(&demand, None);
    if unpaid != Placement::GrowthRefused {
        let receipt = STATE.with_borrow(|s| -> Result<_, String> {
            let pools = &s.as_ref().unwrap().pools;
            let Some(r) = generation(&unpaid)
                .and_then(|g| pools.payments.get(&g))
                .filter(|r| r.demand == demand)
            else {
                return Ok(None);
            };
            if r.maximum != maximum || r.payer != payer {
                return Err("allocation payment binding differs".into());
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
    if api::time() >= deadline || maximum == 0 {
        return Err("a live grant and positive maximum debit are required".into());
    }
    let (reserved, first) = STATE.with_borrow_mut(|s| -> Result<_, String> {
        let s = s.as_mut().unwrap();
        if let Some(q) = s.pools.quotes.get(&pool) {
            if q.demand != demand || q.payer != payer || q.maximum != maximum {
                return Err("pool has an outstanding quote for another request or maximum".into());
            }
            return Ok((q.clone(), false));
        }
        let fee = ic_platform_runtime::raw::cost_create_canister();
        let (index, quote) = match s
            .pools
            .acquisition()
            .map_err(|p| format!("allocation: {p:?}"))?
        {
            Acquisition::Reserve { index, principal } => (
                Some(index),
                Quote {
                    creation_fee: fee,
                    working_cycles: 0,
                    spare: Some(principal),
                },
            ),
            Acquisition::Create if fresh_budget(&s.pools, &demand) => {
                (None, fresh_quote(fee, maximum)?)
            }
            Acquisition::Create => return Err("pool creation budget exhausted".into()),
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
        Ok((q, true))
    })?;
    if !reserved.observed {
        let id = reserved.quote.spare.expect("reserved spare");
        let args = mgmt::CanisterArgs { canister_id: id }
            .encode(1024)
            .map_err(|_| "invalid spare")?;
        let (status, balance) = standby::observe_target(&args).await?;
        if status.module_hash.is_some()
            || status.status != ic_platform_protocol::management_snapshots::Status::Running
        {
            return Err("spare must be empty and running".into());
        }
        return STATE.with_borrow_mut(|s| {
            let s = s.as_mut().unwrap();
            grant(&s.pools, &demand, payer)?;
            if api::time() >= deadline || s.pools.quotes.get(&pool) != Some(&reserved) {
                return Err("original balance observation superseded or expired".into());
            }
            let affordable = reserved
                .quote
                .creation_fee
                .checked_add(balance)
                .is_some_and(|cost| cost <= maximum);
            let mut q = reserved;
            if affordable {
                q.quote.working_cycles = balance;
            } else {
                s.pools.quotes.remove(&pool);
                s.pool.set(s.pools.encode());
                if s.pools.full() {
                    return Err("spare exceeds maximum and subnet is full".into());
                }
                if !fresh_budget(&s.pools, &demand) {
                    return Err(
                        "spare exceeds maximum and pool creation budget is exhausted".into(),
                    );
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
    if first || ic_platform_runtime::raw::msg_cycles_available() == 0 {
        return Ok(Payment::Quoted(reserved.quote));
    }
    let charge = reserved
        .quote
        .creation_fee
        .checked_add(reserved.quote.working_cycles)
        .ok_or("charge overflow")?;
    if charge > maximum || ic_platform_runtime::raw::msg_cycles_available() < charge {
        return Err("insufficient attached cycles for original quote".into());
    }
    let source = match reserved.index {
        Some(index) => Acquisition::Reserve {
            index,
            principal: reserved.quote.spare.expect("reserved principal"),
        },
        None if STATE.with_borrow(|s| s.as_ref().unwrap().pools.full()) => {
            return Err("subnet capacity exhausted".into());
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
