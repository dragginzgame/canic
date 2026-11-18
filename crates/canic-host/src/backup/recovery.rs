//! Explicit operator calls and local admission for the single managed counter.
use super::root::operation;
use canic_management_protocol::{
    backup::{self as snapshot, Step},
    digest,
};
use ic_backup::store::{BackupApplicationGuard, OperationAction, OperationPlan};

/// Build an original Root lifecycle request. Retain this request before submitting it.
/// Lookup uses its original binding, including its unchanged deadline.
/// # Errors
/// Refuses a direct plan or a step outside this procedure.
pub fn recovery_request(
    plan: OperationPlan<'_>,
    step: Step,
    expires_at_ns: u64,
) -> Result<snapshot::Request, snapshot::Error> {
    let operation = operation(plan).ok_or(snapshot::Error::Invalid)?;
    let child = snapshot::management::CanisterArgs {
        canister_id: operation
            .target
            .parse()
            .map_err(|_| snapshot::Error::Invalid)?,
    };
    let (effect, bytes) = if step == Step::Stop {
        let bytes = child
            .encode(snapshot::MAX_INNER)
            .map_err(|_| snapshot::Error::Invalid)?;
        (snapshot::Effect::Canister(child), bytes)
    } else {
        let (_, bytes) = step.recovery_payload(operation.kind, &child)?;
        let source = match plan {
            OperationPlan::Capture(p) => p.digest(),
            OperationPlan::Restore(p) => p.source_plan_sha256(),
        };
        (
            snapshot::Effect::Recovery {
                child,
                source: *source.as_bytes(),
            },
            bytes,
        )
    };
    let manager = plan.root_manager().ok_or(snapshot::Error::Invalid)?;
    Ok(snapshot::Request {
        operation,
        step,
        effect,
        arguments_hash: digest(&bytes),
        arguments_bytes: bytes
            .len()
            .try_into()
            .map_err(|_| snapshot::Error::Invalid)?,
        expires_at_ns,
        cycles: 0,
        cycle_limit: manager.cycle_limit,
        reply_bytes: 1024,
    })
}

/// A bounded local projection of authenticated Root receipts. Root independently checks
/// its surviving ledger at dispatch. Reading this projection never performs a remote call.
#[derive(Default, Clone)]
pub struct CounterRecoveryGuard {
    operation: Option<snapshot::Operation>,
    stopped: bool,
    allocation: bool,
    load: bool,
    started: bool,
}
impl CounterRecoveryGuard {
    /// Original application profile commitment includes the counter schema and executable.
    #[must_use]
    pub fn application(module: &[u8; 32]) -> [u8; 32] {
        digest(
            &[
                b"canic.counter.state.v1.root-recovery.v1".as_slice(),
                module,
            ]
            .concat(),
        )
    }

    /// Consume a reply from the authenticated Root execute or lookup for this original request.
    /// A restarted host reconstructs custody from the same original requests and receipts.
    /// # Errors
    /// Refuses changed bindings, nonterminal effects and missing or conflicting fences.
    pub fn observe(
        &mut self,
        plan: OperationPlan<'_>,
        original: &snapshot::Request,
        receipt: &snapshot::Record,
    ) -> Result<(), snapshot::Error> {
        let (binding, _) =
            recovery_request(plan, original.step, original.expires_at_ns)?.prepare()?;
        if original.prepare()?.0 != binding
            || receipt.binding != binding
            || !matches!(receipt.outcome, snapshot::Outcome::Reply(_))
        {
            return Err(snapshot::Error::Conflict);
        }
        if original.step == Step::Fence && self.operation.is_none() {
            self.operation = Some(binding.operation);
            return Ok(());
        }
        if self.operation.as_ref() != Some(&binding.operation) {
            return Err(snapshot::Error::Conflict);
        }
        match original.step {
            Step::Fence => (),
            Step::Stop => self.stopped = true,
            Step::AdmitAllocation => self.allocation = true,
            Step::AdmitLoad => self.load = true,
            Step::Start | Step::Accept | Step::Open => self.started = true,
            _ => return Err(snapshot::Error::Invalid),
        }
        Ok(())
    }
}
impl BackupApplicationGuard for CounterRecoveryGuard {
    type Error = snapshot::Error;

    fn protocol(&self) -> [u8; 32] {
        digest(b"canic.counter.recovery.guard.v1")
    }

    fn admit(
        &self,
        plan: OperationPlan<'_>,
        action: OperationAction,
        application: &[u8; 32],
    ) -> Result<(), Self::Error> {
        let module = match plan {
            OperationPlan::Capture(p) => p.request().module_sha256,
            OperationPlan::Restore(p) => p.module_sha256(),
        };
        if !self.stopped
            || self.started
            || self.operation != operation(plan)
            || *application != Self::application(module.as_bytes())
        {
            return Err(snapshot::Error::Conflict);
        }
        let allowed = match (plan, action) {
            (_, OperationAction::Inspect)
            | (OperationPlan::Capture(_), OperationAction::ReadSnapshot)
            | (OperationPlan::Capture(_), OperationAction::TakeSnapshot) => true,
            (
                OperationPlan::Restore(_),
                OperationAction::AllocateSnapshot
                | OperationAction::UploadSnapshot
                | OperationAction::ReadSnapshot,
            ) => self.allocation,
            (OperationPlan::Restore(_), OperationAction::LoadSnapshot) => {
                self.load && self.allocation
            }
            _ => false,
        };
        if allowed {
            Ok(())
        } else {
            Err(snapshot::Error::Busy)
        }
    }
}
