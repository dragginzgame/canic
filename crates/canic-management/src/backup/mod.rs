//! Stable original-result ledger, independent of application rollback and Host journals.

#[cfg(test)]
mod tests;

mod runtime;

use canic_management_protocol::backup::{
    Binding, Error, MAX_RETAINED, MAX_STEPS, Outcome, Record, Step,
};
#[cfg(feature = "fault-injection")]
#[doc(hidden)]
pub use runtime::inject;

pub(crate) use runtime::occupied;

pub use runtime::{execute, lookup};
use serde::Serialize;

pub(crate) const CAPACITY: usize = MAX_RETAINED + 128 * 1024;
const LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: CAPACITY,
    allocation: CAPACITY * 10,
    nodes: 131_072,
    depth: 24,
};
#[derive(Default, Serialize)]
pub(crate) struct Ledger(Vec<Record>);
impl Ledger {
    pub(crate) fn encode(&self) -> Vec<u8> {
        ic_wire::encode(self, ic_wire::Encoding::Payload, LIMITS).expect("bounded snapshot ledger")
    }

    pub(crate) fn open(bytes: &[u8]) -> Self {
        let records: ic_wire::List<Record, MAX_STEPS> =
            ic_wire::decode(bytes, ic_wire::Encoding::Payload, LIMITS)
                .expect("snapshot ledger reopen");
        let ledger = Self(records.into_vec());
        assert!(ledger.0.len() <= MAX_STEPS);
        assert!(
            ledger
                .0
                .iter()
                .map(|r| r.binding.reply_bytes as usize)
                .sum::<usize>()
                <= MAX_RETAINED
        );
        ledger
    }

    pub(crate) fn occupied(&self) -> bool {
        !self.0.is_empty()
    }

    pub(crate) fn lookup(&self, b: &Binding) -> Result<Record, Error> {
        for r in self.0.iter() {
            let a = &r.binding.operation;
            if a.id == b.operation.id && a.kind == b.operation.kind {
                if a != &b.operation {
                    return Err(Error::Conflict);
                }
                if r.binding.step == b.step {
                    return if &r.binding == b {
                        Ok(r.clone())
                    } else {
                        Err(Error::Conflict)
                    };
                }
            }
        }
        Err(Error::Missing)
    }

    pub(crate) fn reserve(&mut self, b: Binding) -> Result<(), Error> {
        match self.lookup(&b) {
            Err(Error::Missing) => (),
            Ok(_) => return Err(Error::Conflict),
            Err(e) => return Err(e),
        }
        if self
            .0
            .iter()
            .any(|r| !matches!(r.outcome, Outcome::Reply(_)))
        {
            return Err(Error::Busy);
        }
        let previous = self
            .0
            .iter()
            .find(|r| r.binding.operation.kind == b.operation.kind);
        if previous.is_some_and(|r| r.binding.operation != b.operation) {
            return Err(Error::Capacity);
        }
        // A second operation cannot adopt the first operation's incomplete target work.
        if previous.is_none()
            && !self.0.is_empty()
            && !self.0.iter().any(|r| {
                r.binding.step == Step::FinalStatus && matches!(r.outcome, Outcome::Reply(_))
            })
        {
            return Err(Error::Busy);
        }
        if self.0.len() >= MAX_STEPS
            || self
                .0
                .iter()
                .map(|r| r.binding.reply_bytes as usize)
                .sum::<usize>()
                + b.reply_bytes as usize
                > MAX_RETAINED
        {
            return Err(Error::Capacity);
        }
        self.0.push(Record {
            binding: b,
            outcome: Outcome::Pending,
        });
        Ok(())
    }

    pub(crate) fn complete(&mut self, b: &Binding, outcome: Outcome) {
        let r = self
            .0
            .iter_mut()
            .find(|r| &r.binding == b)
            .expect("original issued slot");
        assert_eq!(r.outcome, Outcome::Pending, "terminal outcome immutable");
        if outcome.retained_bytes() <= b.reply_bytes as usize {
            r.outcome = outcome;
        }
    }
}

impl Ledger {
    /// Existing raw executor operations retain their original profile. A capture fence
    /// selects the recovery profile for the lifetime of this two-operation ledger.
    pub(crate) fn admit_recovery(
        &self,
        request: &canic_management_protocol::backup::Request,
    ) -> Result<(), Error> {
        use canic_management_protocol::backup::{Effect, Kind};
        let op = &request.operation;
        let Some(capture) = self
            .0
            .iter()
            .find(|r| r.binding.operation.kind == Kind::Capture && r.binding.step == Step::Fence)
        else {
            return match (&request.effect, op.kind, request.step) {
                (Effect::Recovery { .. }, Kind::Capture, Step::Fence) if self.0.is_empty() => {
                    Ok(())
                }
                (Effect::Recovery { .. }, _, _) => Err(Error::Busy),
                _ => Ok(()),
            };
        };
        let capture = &capture.binding.operation;
        if capture.manager != op.manager
            || capture.caller != op.caller
            || capture.network != op.network
            || capture.target != op.target
            || capture.generation != op.generation
        {
            return Err(Error::Conflict);
        }
        if let Effect::Recovery { source, .. } = &request.effect
            && *source != capture.approval
        {
            return Err(Error::Conflict);
        }
        let done = |kind, step| {
            self.0.iter().any(|r| {
                r.binding.operation.kind == kind
                    && r.binding.step == step
                    && matches!(r.outcome, Outcome::Reply(_))
            })
        };
        let here = |step| done(op.kind, step);
        let allowed = match request.step {
            Step::Fence => op.kind == Kind::Restore && done(Kind::Capture, Step::Open),
            Step::Stop => here(Step::Fence),
            Step::Start => here(Step::FinalStatus),
            Step::Accept => op.kind == Kind::Restore && here(Step::Start),
            Step::Open => {
                if op.kind == Kind::Capture {
                    here(Step::Start)
                } else {
                    here(Step::Accept)
                }
            }
            _ if !here(Step::Stop) || here(Step::Start) => false,
            Step::AdmitAllocation => here(Step::List),
            Step::Allocate => here(Step::AdmitAllocation),
            Step::Upload(_) => here(Step::Allocate) && !here(Step::BeforeLoad),
            Step::BeforeLoad => here(Step::Allocate) && here(Step::Metadata),
            Step::AdmitLoad => here(Step::BeforeLoad),
            Step::Load => here(Step::AdmitLoad),
            Step::FinalStatus => here(if op.kind == Kind::Capture {
                Step::Take
            } else {
                Step::Load
            }),
            Step::Read(_) | Step::Metadata => {
                if op.kind == Kind::Capture {
                    here(Step::Take)
                } else {
                    here(Step::Allocate) && !here(Step::Load)
                }
            }
            Step::InitialStatus | Step::List | Step::Take => true,
        };
        if allowed { Ok(()) } else { Err(Error::Busy) }
    }
}
