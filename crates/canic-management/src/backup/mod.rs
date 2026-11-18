//! Stable original-result ledger, independent of application rollback and Host journals.
use canic_management_protocol::backup::{
    Binding, Error, MAX_RETAINED, MAX_STEPS, Outcome, Record, Step,
};
use serde::Serialize;

#[cfg(target_arch = "wasm32")]
mod runtime;
#[cfg(test)]
mod tests;
#[cfg(all(target_arch = "wasm32", feature = "fault-injection"))]
#[doc(hidden)]
pub use runtime::inject;
#[cfg(target_arch = "wasm32")]
pub(crate) use runtime::occupied;
#[cfg(target_arch = "wasm32")]
pub use runtime::{execute, lookup};

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
