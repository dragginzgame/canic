//! Recurring deposits from Root to its directly installed named services.
use super::super::{MANAGEMENT_LIMITS, STATE, api, directory, management, reserved_cycles};
use canic_management_protocol::stored::FundingError;
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
    pub(super) rule: ChildFunding,
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

    pub(super) fn settled(&self) -> u64 {
        let status = &self.status;
        // A failed attempt closes the grant. Only its final attempt can be unsettled.
        status.attempts
            - u64::from(status.attempts > 0 && (status.issued || status.refund_cycles != Some(0)))
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
        s.as_ref()
            .unwrap()
            .journal
            .rows()
            .get(&slot)
            .is_some_and(|j| {
                j.0.phase == Phase::Ready && j.0.worker.as_deref() == Some(&r.child.to_text())
            })
    }) && directory::named_service(slot).is_some_and(|service| service.name == r.component)
}

/// Install one immutable grant, or inspect that exact grant after a retry.
/// # Errors
/// Requires controller approval, capacity custody and a matching ready permanent child.
/// # Panics
/// Requires an opened Root.
pub fn configure(rule: ChildFunding) -> Result<ChildFundingStatus, FundingError> {
    if !api::is_controller(&api::caller()) {
        return Err(FundingError::Unauthorized);
    }
    let slot = super::super::service_slots()
        .into_iter()
        .find(|&slot| installed(slot, &rule))
        .ok_or(FundingError::UnknownChild)?;
    if let Some(previous) = STATE.with_borrow(|s| {
        s.as_ref().unwrap().pools.funding.get(&slot).map(|g| {
            if g.rule == rule {
                Ok(g.status.clone())
            } else {
                Err(FundingError::ApprovalConflict)
            }
        })
    }) {
        return previous;
    }
    super::admit_pool_change(false).map_err(|_| FundingError::CustodyOccupied)?;
    let grant = Grant {
        rule,
        status: ChildFundingStatus::default(),
    };
    if !super::has_capacity() || !grant.valid() || !grant.active(api::time()) {
        return Err(FundingError::InvalidApproval);
    }
    STATE.with_borrow_mut(|s| {
        let s = s.as_mut().unwrap();
        s.pools.funding.insert(slot, grant);
        s.pool
            .put(&super::Key::Funding(slot), &s.pools.funding[&slot]);
    });
    schedule(slot);
    Ok(ChildFundingStatus::default())
}

/// Stop new deposits. Any issued debit remains charged.
/// # Errors
/// The caller must control Root and name an approved child.
/// # Panics
/// Requires an opened Root.
pub fn stop(child: Principal) -> Result<ChildFundingStatus, FundingError> {
    if !api::is_controller(&api::caller()) {
        return Err(FundingError::Unauthorized);
    }
    STATE.with_borrow_mut(|s| {
        let s = s.as_mut().unwrap();
        let (slot, g) = s
            .pools
            .funding
            .iter_mut()
            .find(|(_, g)| g.rule.child == child)
            .ok_or(FundingError::UnknownChild)?;
        g.status.closed = true;
        let status = g.status.clone();
        s.pool.put(&super::Key::Funding(*slot), g);
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
    let now = api::time();
    STATE.with_borrow(|s| {
        for (&slot, grant) in &s.as_ref().unwrap().pools.funding {
            if grant.active(now) {
                schedule(slot);
            }
        }
    });
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
    if crate::backup::target_occupied(&rule.child.to_text()) {
        return Ok(false);
    }
    let Some(_guard) = super::super::observing(slot) else {
        return Ok(false);
    };
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
        s.pool
            .put(&super::Key::Funding(slot), &s.pools.funding[&slot]);
    });
    let (success, refund) = super::super::deposit_cycles(rule.child, rule.amount_cycles).await;
    STATE.with_borrow_mut(|s| {
        let s = s.as_mut().unwrap();
        let status = &mut s.pools.funding.get_mut(&slot).unwrap().status;
        status.issued = !success && refund != Some(u128::from(rule.amount_cycles));
        if success {
            s.pools
                .receipts
                .record(rule.child, status.attempts, api::time());
        }
        status.closed |= !success;
        status.refund_cycles = refund;
        s.pool
            .put(&super::Key::Funding(slot), &s.pools.funding[&slot]);
        if success {
            s.pool.put(&super::Key::FundingReceipts, &s.pools.receipts);
        }
    });
    if success { Ok(true) } else { Err(()) }
}

/// Report only the calling child's operating grant receipts.
/// # Panics
/// Requires the Root placement ledger to be open.
#[must_use]
pub fn receipts() -> Option<canic::funding::FundingReceipts> {
    STATE.with_borrow(|state| {
        let pools = &state.as_ref().expect("open Root").pools;
        let child = api::caller();
        let grant = pools
            .funding
            .values()
            .find(|grant| grant.rule.child == child)?;
        Some(
            pools
                .receipts
                .report(&grant.rule, &grant.status, grant.settled()),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pre_receipt_settlement_count_excludes_the_last_unconfirmed_or_refunded_attempt() {
        let mut grant = Grant {
            rule: ChildFunding {
                parent: Principal::from_slice(&[1]),
                child: Principal::from_slice(&[2]),
                component: "service".into(),
                below_cycles: 1,
                amount_cycles: 10,
                budget_cycles: 30,
                expires_at_ns: u64::MAX,
            },
            status: ChildFundingStatus::default(),
        };
        assert_eq!(grant.settled(), 0);
        grant.status.attempts = 3;
        for (issued, refund, paid) in [
            (true, None, 2),
            (true, Some(0), 2),
            (false, Some(10), 2),
            (false, Some(0), 3),
        ] {
            grant.status.issued = issued;
            grant.status.refund_cycles = refund;
            assert_eq!(grant.settled(), paid);
        }
    }
}
