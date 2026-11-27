//! Stable original-result ledger, independent of application rollback and Host journals.

#[cfg(test)]
mod tests;

mod runtime;

use canic_management_protocol::backup::{
    Binding, Error, Kind, LIMITS, Operation, Outcome, Record, Step,
};
#[cfg(feature = "fault-injection")]
#[doc(hidden)]
pub use runtime::inject;

pub(crate) use runtime::{occupied, target_occupied};

use ic_storage::{DefaultMemoryImpl, MemoryRuntime, adapter::StableBTreeMap};
pub use runtime::{execute, lookup};
pub(crate) const KEY: &str = "manager.target_snapshots.v1";

#[derive(Clone, Copy, serde::Serialize)]
enum Key<'a> {
    Owner(Kind, [u8; 16]),
    Active(&'a str),
    Count,
    Capture(&'a str),
    Source(&'a str, [u8; 32]),
    RestoreSource([u8; 16]),
    Step(Kind, [u8; 16], Step),
    Unsettled(&'a str),
}

fn encode(value: &impl serde::Serialize) -> Vec<u8> {
    ic_wire::encode(value, ic_wire::Encoding::Payload, LIMITS).expect("snapshot row")
}

pub(crate) struct Ledger {
    stable: StableBTreeMap<Vec<u8>, Vec<u8>>,
}

impl Ledger {
    pub(crate) fn open(runtime: &MemoryRuntime<DefaultMemoryImpl>, fresh: bool) -> Self {
        Self {
            stable: canic::storage::map(runtime, KEY, fresh),
        }
    }

    fn get<T: serde::de::DeserializeOwned>(&self, key: Key<'_>) -> Option<T> {
        self.stable.get(&encode(&key)).map(|bytes| {
            ic_wire::decode(&bytes, ic_wire::Encoding::Payload, LIMITS)
                .expect("retained snapshot row")
        })
    }

    fn active(&self, target: &str) -> Option<Operation> {
        self.get(Key::Active(target))
    }

    fn count(&self) -> u64 {
        self.get(Key::Count).unwrap_or(0)
    }

    fn record(&self, operation: &Operation, step: Step) -> Option<Record> {
        self.get(Key::Step(operation.kind, operation.id, step))
    }

    fn done(&self, op: &Operation, step: Step) -> bool {
        self.record(op, step)
            .is_some_and(|r| matches!(r.outcome, Outcome::Reply(_)))
    }

    pub(crate) fn occupied(&self) -> bool {
        self.count() != 0
    }

    pub(crate) fn lookup(&self, b: &Binding) -> Result<Record, Error> {
        if self
            .get::<Operation>(Key::Owner(b.operation.kind, b.operation.id))
            .is_some_and(|op| op != b.operation)
        {
            return Err(Error::Conflict);
        }
        let r = self.record(&b.operation, b.step).ok_or(Error::Missing)?;
        if r.binding != *b {
            return Err(Error::Conflict);
        }
        Ok(r)
    }

    pub(crate) fn reserve(&mut self, b: Binding) -> Result<(), Error> {
        match self.lookup(&b) {
            Err(Error::Missing) => (),
            Ok(_) => return Err(Error::Conflict),
            Err(e) => return Err(e),
        }
        let op = &b.operation;
        let pending = encode(&Key::Unsettled(&op.target));
        if self.stable.contains_key(&pending) {
            return Err(Error::Busy);
        }
        if let Some(active) = self.active(&op.target) {
            if active != *op {
                return Err(Error::Busy);
            }
        } else {
            let owner = encode(&Key::Owner(op.kind, op.id));
            if self.stable.contains_key(&owner) {
                return Err(Error::Conflict);
            }
            if op.kind == Kind::Capture
                && b.step == Step::Fence
                && self
                    .stable
                    .contains_key(&encode(&Key::Source(&op.target, op.approval)))
            {
                return Err(Error::Conflict);
            }
            self.stable.insert(owner, encode(op));
            self.stable
                .insert(encode(&Key::Active(&op.target)), encode(op));
            self.stable
                .insert(encode(&Key::Count), encode(&(self.count() + 1)));
            if op.kind == Kind::Capture {
                self.stable
                    .insert(encode(&Key::Capture(&op.target)), encode(op));
                if b.step == Step::Fence {
                    self.stable
                        .insert(encode(&Key::Source(&op.target, op.approval)), encode(op));
                }
            }
        }
        let key = encode(&Key::Step(b.operation.kind, b.operation.id, b.step));
        self.stable.insert(pending, key.clone());
        self.stable.insert(
            key,
            encode(&Record {
                binding: b,
                outcome: Outcome::Pending,
            }),
        );
        Ok(())
    }

    pub(crate) fn complete(&mut self, b: &Binding, outcome: Outcome) {
        let mut r = self.lookup(b).expect("original issued slot");
        assert_eq!(r.outcome, Outcome::Pending, "terminal outcome immutable");
        if outcome.retained_bytes() <= b.reply_bytes as usize {
            let key = encode(&Key::Step(b.operation.kind, b.operation.id, b.step));
            let pending = encode(&Key::Unsettled(&b.operation.target));
            assert_eq!(
                self.stable.get(&pending).as_ref(),
                Some(&key),
                "original unsettled snapshot step"
            );
            let replied = matches!(outcome, Outcome::Reply(_));
            let declined_snapshot = matches!(outcome, Outcome::Rejected(..) | Outcome::Refused)
                && matches!(b.step, Step::Take | Step::Allocate);
            if replied || declined_snapshot {
                self.stable.remove(&pending);
                let fenced = self.record(&b.operation, Step::Fence).is_some();
                if b.step
                    == if fenced {
                        Step::Open
                    } else {
                        Step::FinalStatus
                    }
                {
                    self.stable
                        .remove(&encode(&Key::Active(&b.operation.target)))
                        .expect("active snapshot target");
                    self.stable
                        .insert(encode(&Key::Count), encode(&(self.count() - 1)));
                }
            }
            r.outcome = outcome;
            self.stable.insert(key, encode(&r));
        }
    }
}

impl Ledger {
    fn bind_restore(&mut self, op: &Operation, source: [u8; 32]) {
        self.stable
            .insert(encode(&Key::RestoreSource(op.id)), encode(&source));
    }

    /// Enforce the selected capture and the exact operation's recovery procedure.
    pub(crate) fn admit_recovery(
        &self,
        request: &canic_management_protocol::backup::Request,
    ) -> Result<(), Error> {
        use canic_management_protocol::backup::Effect;
        let op = &request.operation;
        let capture = match (op.kind, &request.effect) {
            (Kind::Capture, _) => {
                if request.step == Step::Fence {
                    return if self.active(&op.target).is_none() {
                        Ok(())
                    } else {
                        Err(Error::Busy)
                    };
                }
                if self.active(&op.target).is_none()
                    && self
                        .get::<Operation>(Key::Capture(&op.target))
                        .is_some_and(|old| self.record(&old, Step::Fence).is_some())
                {
                    return Err(Error::Busy);
                }
                self.record(op, Step::Fence).map(|r| r.binding.operation)
            }
            (Kind::Restore, Effect::Recovery { source, .. }) => {
                let capture: Operation = self
                    .get(Key::Source(&op.target, *source))
                    .ok_or(Error::Conflict)?;
                if self
                    .get::<[u8; 32]>(Key::RestoreSource(op.id))
                    .is_some_and(|bound| bound != *source)
                {
                    return Err(Error::Conflict);
                }
                Some(capture)
            }
            (Kind::Restore, _) => {
                let capture = if let Some(source) = self.get(Key::RestoreSource(op.id)) {
                    self.get(Key::Source(&op.target, source))
                } else {
                    self.get(Key::Capture(&op.target))
                };
                capture.filter(|c| self.record(c, Step::Fence).is_some())
            }
        };
        let Some(capture) = capture else {
            return if matches!(request.effect, Effect::Recovery { .. }) {
                Err(Error::Busy)
            } else {
                Ok(())
            };
        };
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
        let here = |step| self.done(op, step);
        let allowed = match request.step {
            Step::Fence => {
                op.kind == Kind::Restore
                    && self.done(&capture, Step::Open)
                    && self.done(&capture, Step::Take)
            }
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
            Step::FinalStatus if op.kind == Kind::Capture => self
                .record(op, Step::Take)
                .is_some_and(|r| !matches!(r.outcome, Outcome::Pending)),
            Step::FinalStatus => here(Step::Load),
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
