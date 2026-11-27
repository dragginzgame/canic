use super::Ledger;
use crate::engine::{self, STATE};
use canic_management_protocol::{
    Phase,
    backup::{Binding, Effect, Error, Outcome, Record, Request},
    upgrade,
};
use ic_platform_runtime::{
    call::{Call, Outcome as CallOutcome, Wait},
    system as api,
};
use ic_platform_types::Principal;

pub(crate) fn occupied() -> bool {
    STATE.with_borrow(|s| s.as_ref().is_some_and(|s| s.backup.occupied()))
}

pub(crate) fn target_occupied(target: &str) -> bool {
    read(|ledger| ledger.active(target).is_some())
}

fn read<T>(f: impl FnOnce(&Ledger) -> T) -> T {
    STATE.with_borrow(|s| f(&s.as_ref().expect("open manager").backup))
}

fn mutate<T>(f: impl FnOnce(&mut Ledger) -> T) -> T {
    STATE.with_borrow_mut(|s| {
        let s = s.as_mut().expect("open manager");
        f(&mut s.backup)
    })
}

/// Return only the exact retained original record; never issue management work.
/// # Errors
/// Refuses a missing or conflicting original binding.
/// # Panics
/// Traps without current controller authority or an opened management owner.
pub fn lookup(binding: &Binding) -> Result<Record, Error> {
    assert!(api::is_controller(&api::caller()), "controller required");
    read(|l| l.lookup(binding))
}

/// Issue one admitted snapshot effect and durably retain its bounded original outcome.
/// Duplicate execution is lookup-only, including after expiry and compatible upgrade.
/// # Errors
/// Refuses conflicts, exhausted allowances, unowned targets or active management work.
/// # Panics
/// Traps on authority failure, failed custody writes or qualification fault injection.
pub async fn execute(request: Request) -> Result<Record, Error> {
    assert!(api::is_controller(&api::caller()), "controller required");
    let (binding, bytes) = request.prepare()?;
    match read(|l| l.lookup(&binding)) {
        Ok(r) => return Ok(r),
        Err(Error::Missing) => (),
        Err(e) => return Err(e),
    }
    let op = &binding.operation;
    if op.manager != api::canister_self().to_text() || op.caller != api::caller().to_text() {
        return Err(Error::Invalid);
    }
    if api::time() > binding.expires_at_ns
        || binding.expires_at_ns.saturating_sub(api::time()) > 300_000_000_000
    {
        return Err(Error::Expired);
    }
    let version = match &request.effect {
        Effect::Take(a) => a.sender_canister_version,
        Effect::Load(a) => a.sender_canister_version,
        _ => None,
    };
    if version.is_some_and(|v| v != api::canister_version()) {
        return Err(Error::Invalid);
    }
    let slot = engine::service_for_worker(&op.target).ok_or(Error::Busy)?;
    #[cfg(feature = "sharding")]
    if !engine::sharding::snapshot_eligible(slot) {
        return Err(Error::Busy);
    }
    let creation = engine::status(slot);
    let upgrade = engine::upgrade::status(slot);
    if creation.phase != Phase::Ready
        || creation.failure.is_some()
        || !matches!(upgrade.phase, upgrade::Phase::Empty | upgrade::Phase::Ready)
        || upgrade.failure.is_some()
        || !engine::snapshot_idle(slot)
        || engine::store_maintenance::active()
    {
        return Err(Error::Busy);
    }
    read(|ledger| ledger.admit_recovery(&request))?;
    if matches!(
        binding.step,
        canic_management_protocol::backup::Step::AdmitAllocation
            | canic_management_protocol::backup::Step::AdmitLoad
    ) {
        return mutate(|ledger| {
            ledger.reserve(binding.clone())?;
            ledger.complete(&binding, Outcome::Reply(Vec::new()));
            ledger.lookup(&binding)
        });
    }
    let command = binding.step.command(op.kind);
    let recipient = if command.is_some() {
        Principal::from_text(&op.target).map_err(|_| Error::Invalid)?
    } else {
        Principal::management_canister()
    };
    let call = Call::new(recipient, &binding.method, Wait::Unbounded)
        .owned_args(bytes)
        .cycles(u128::from(binding.cycles));
    let required = call.required_cycles().map_err(|_| Error::Capacity)?;
    if required > u128::from(binding.cycle_limit)
        || ic_platform_runtime::raw::liquid_cycle_balance() < engine::reserved_cycles() + required
    {
        return Err(Error::Capacity);
    }
    mutate(|l| {
        l.reserve(binding.clone())?;
        if op.kind == canic_management_protocol::backup::Kind::Restore
            && binding.step == canic_management_protocol::backup::Step::Fence
            && let Effect::Recovery { source, .. } = &request.effect
        {
            l.bind_restore(op, *source);
        }
        Ok(())
    })?;
    // This reservation and enqueue commit together when this execution returns.
    // Callback trapping rolls back only callback evidence, leaving Pending permanent.
    let completion = call.execute().await;
    #[cfg(feature = "fault-injection")]
    fault(1);
    let outcome = match completion {
        Ok(c) => match c.into_parts().0 {
            CallOutcome::Reply(bytes) => {
                if command.is_some_and(|c| {
                    canic::recovery::encode(c, ic_wire::Direction::Response)
                        .ok()
                        .as_ref()
                        != Some(&bytes)
                }) {
                    // The child may have applied the command despite a bad acknowledgement.
                    Outcome::Pending
                } else {
                    Outcome::Reply(bytes)
                }
            }
            CallOutcome::Rejected(r) => Outcome::Rejected(r.code(), r.message().into()),
            CallOutcome::OutcomeUnknown(_) => Outcome::Pending,
        },
        Err(_) => Outcome::Refused,
    };
    mutate(|l| l.complete(&binding, outcome));
    #[cfg(feature = "fault-injection")]
    fault(2);
    read(|l| l.lookup(&binding))
}

#[cfg(feature = "fault-injection")]
thread_local! { static FAULT: std::cell::Cell<u8> = const { std::cell::Cell::new(0) }; }

#[cfg(feature = "fault-injection")]
fn fault(point: u8) {
    if FAULT.with(|f| f.get()) == point {
        api::trap("snapshot callback fault");
    }
}

/// Select a callback-only qualification fault; never mounted by production Root.
#[cfg(feature = "fault-injection")]
pub fn inject(point: u8) {
    FAULT.with(|f| f.set(point));
}
