//! Recurring deposits from Root to its directly installed named services.
use super::super::{MANAGEMENT_LIMITS, STATE, api, directory, management, reserved_cycles};
use canic_management_protocol::{
    Phase,
    children::ChildId,
    stored::{ChildFunding, ChildFundingStatus},
};
use ic_platform_protocol::management as mgmt;
use ic_platform_types::Principal;
use ic_timers::{Lifetime, Observation, OnceDemand, OnceNext, OnceResult};
use serde::{Deserialize, Serialize};

const CHECK_NS: u64 = 60_000_000_000;

#[derive(Serialize, Deserialize)]
pub(super) struct Grant {
    rule: ChildFunding,
    status: ChildFundingStatus,
}

impl Grant {
    pub(super) fn valid(&self) -> bool {
        let r = &self.rule;
        r.parent == api::canister_self()
            && r.below_cycles > 0
            && r.amount_cycles > 0
            && r.budget_cycles >= r.amount_cycles
            && r.expires_at_ns > 0
            && self.status.charged_cycles <= r.budget_cycles
            && self.status.attempts.checked_mul(r.amount_cycles) == Some(self.status.charged_cycles)
    }

    fn active(&self, now: u64) -> bool {
        !self.status.closed
            && !self.status.issued
            && now < self.rule.expires_at_ns
            && self.rule.budget_cycles - self.status.charged_cycles >= self.rule.amount_cycles
    }
}

fn installed(slot: ChildId, r: &ChildFunding) -> bool {
    STATE.with_borrow(|s| {
        s.as_ref().unwrap().journal.0.get(&slot).is_some_and(|j| {
            j.0.phase == Phase::Ready && j.0.worker.as_deref() == Some(&r.child.to_text())
        })
    }) && directory::named_service(slot).is_some_and(|service| service.name == r.component)
}

/// Install one immutable grant, or inspect that exact grant after a retry.
/// # Errors
/// Requires controller approval, capacity custody and a matching ready permanent child.
/// # Panics
/// Requires an opened Root.
pub fn configure(rule: ChildFunding) -> Result<ChildFundingStatus, String> {
    if !api::is_controller(&api::caller()) {
        return Err("controller required".into());
    }
    let slot = ChildId::ALL
        .into_iter()
        .find(|&slot| installed(slot, &rule))
        .ok_or("funding requires the named direct child installed by this Root")?;
    if let Some(previous) = STATE.with_borrow(|s| {
        s.as_ref().unwrap().pools.funding.get(&slot).map(|g| {
            if g.rule == rule {
                Ok(g.status.clone())
            } else {
                Err("child funding approval differs".into())
            }
        })
    }) {
        return previous;
    }
    super::admit_pool_change(false).map_err(|e| format!("{e:?}"))?;
    let grant = Grant {
        rule,
        status: ChildFundingStatus::default(),
    };
    if !super::has_capacity() || !grant.valid() || !grant.active(api::time()) {
        return Err("funding requires subnet capacity, positive amounts, sufficient budget and a future deadline".into());
    }
    STATE.with_borrow_mut(|s| {
        let s = s.as_mut().unwrap();
        s.pools.funding.insert(slot, grant);
        s.pool.set(s.pools.encode());
    });
    schedule(slot);
    Ok(ChildFundingStatus::default())
}

/// Stop new deposits. Any issued debit remains charged.
/// # Errors
/// The caller must control Root and name an approved child.
/// # Panics
/// Requires an opened Root.
pub fn stop(child: Principal) -> Result<ChildFundingStatus, String> {
    if !api::is_controller(&api::caller()) {
        return Err("controller required".into());
    }
    STATE.with_borrow_mut(|s| {
        let s = s.as_mut().unwrap();
        let g = s
            .pools
            .funding
            .values_mut()
            .find(|g| g.rule.child == child)
            .ok_or("unknown funding child")?;
        g.status.closed = true;
        let status = g.status.clone();
        s.pool.set(s.pools.encode());
        Ok(status)
    })
}

fn active(slot: ChildId) -> Option<ChildFunding> {
    STATE.with_borrow(|s| {
        s.as_ref()?
            .pools
            .funding
            .get(&slot)
            .filter(|g| g.active(api::time()))
            .map(|g| g.rule.clone())
    })
}

pub(in crate::engine) fn resume() {
    for slot in ChildId::ALL {
        if active(slot).is_some() {
            schedule(slot);
        }
    }
}

fn schedule(slot: ChildId) {
    let index = slot.index().to_string();
    ic_timers::bind_once(
        ["canic", "child-funding", &index],
        Lifetime::RemoveWhenStopped,
        move || step(slot),
    )
    .expect("child funding timer")
    .set(OnceDemand::At(api::time()))
    .expect("schedule balance check");
}

async fn step(slot: ChildId) -> OnceResult {
    let observation = match active(slot) {
        Some(rule) => match deposit(slot, &rule).await {
            Ok(true) => Observation::Success,
            Ok(false) => Observation::NoWork,
            Err(()) => Observation::ExpectedFailure,
        },
        None => Observation::NoWork,
    };
    OnceResult {
        observation,
        next: active(slot).map_or(OnceNext::Stop, |r| {
            OnceNext::At(api::time().saturating_add(CHECK_NS).min(r.expires_at_ns))
        }),
    }
}

async fn deposit(slot: ChildId, rule: &ChildFunding) -> Result<bool, ()> {
    let args = mgmt::CanisterArgs {
        canister_id: rule.child,
    }
    .encode(1024)
    .map_err(|_| ())?;
    let raw = management("canister_status", args, 0)
        .await
        .map_err(|_| ())?;
    let status = mgmt::decode_canister_status(&raw, MANAGEMENT_LIMITS).map_err(|_| ())?;
    let resources = mgmt::decode_canister_resources(&raw, MANAGEMENT_LIMITS).map_err(|_| ())?;
    if status.settings.controllers != [rule.parent]
        || resources.cycles.0 >= rule.below_cycles.into()
        || active(slot).as_ref() != Some(rule)
        || !installed(slot, rule)
        || super::admit_pool_change(false).is_err()
        || ic_platform_runtime::raw::liquid_cycle_balance()
            < reserved_cycles() + u128::from(rule.amount_cycles)
    {
        return Ok(false);
    }

    STATE.with_borrow_mut(|s| {
        let s = s.as_mut().unwrap();
        let status = &mut s.pools.funding.get_mut(&slot).unwrap().status;
        status.attempts += 1;
        status.charged_cycles += rule.amount_cycles;
        status.issued = true;
        status.refund_cycles = None;
        s.pool.set(s.pools.encode());
    });
    let (success, refund) = super::super::deposit_cycles(rule.child, rule.amount_cycles).await;
    STATE.with_borrow_mut(|s| {
        let s = s.as_mut().unwrap();
        let status = &mut s.pools.funding.get_mut(&slot).unwrap().status;
        status.issued = !success && refund != Some(u128::from(rule.amount_cycles));
        status.closed |= !success;
        status.refund_cycles = refund;
        s.pool.set(s.pools.encode());
    });
    if success { Ok(true) } else { Err(()) }
}

/// Verify a managed payment owner before forwarding a controller's funding decision.
/// # Errors
/// Requires a ready retained child and Root controller authority.
/// # Panics
/// Requires an open Root.
pub fn check_parent(parent: Principal) -> Result<(), String> {
    if !api::is_controller(&api::caller()) {
        return Err("controller required".into());
    }
    if STATE.with_borrow(|s| {
        s.as_ref()
            .expect("open Root")
            .journal
            .0
            .values()
            .any(|j| j.0.phase == Phase::Ready && j.0.worker.as_deref() == Some(&parent.to_text()))
    }) {
        Ok(())
    } else {
        Err("funding parent must be a ready managed installation".into())
    }
}
