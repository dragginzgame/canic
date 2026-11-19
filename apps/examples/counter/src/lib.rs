//! Application-owned counter contract. The IC supplies caller identity at execution.

mod endpoints;

use canic::{method::Contract, readiness::State as Readiness};
use ic_platform_runtime::system as api;
use ic_platform_types::Principal;
use ic_storage::{DefaultMemoryImpl, adapter::StableCell};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;

/// Wire identity of the replicated `counter` endpoint.
pub const METHOD: u64 = 1;

/// Persisted record budget shared by install and reopen.
const RECORD_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 256,
    allocation: 4096,
    nodes: 64,
    depth: 8,
};

///
/// CounterMethod
/// Wire contract shared by the canister and its native command codec.
///

pub struct CounterMethod;

impl Contract for CounterMethod {
    const NAME: &'static str = "counter";
    const ID: u64 = METHOD;
    type Input = Request;
    type Output = Result<Snapshot, Error>;
}

/// Explicit application operations. Caller identity always comes from ingress.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Request {
    /// Observe the current value and fence without changing them.
    Read,
    /// Increment once; no automatic retry or application deduplication is promised.
    Increment,
    /// Close writes under a new monotonic nonzero maintenance attempt.
    Fence {
        /// Application maintenance identity, selected by the installation authority.
        attempt: u64,
    },
    /// Explicitly reopen the matching fence. A stale attempt cannot activate a newer one.
    Activate {
        /// Exact previously admitted attempt, never a caller assertion of authority.
        attempt: u64,
    },
}

///
/// Snapshot
/// Reports the counter value, active fence and executable version.
///

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Snapshot {
    /// Number of committed increments.
    pub value: u64,
    /// Current closed-admission attempt, if any.
    pub fence: Option<u64>,
    /// Executable marker for the qualified A/B upgrade, not a storage schema.
    pub executable: u8,
}

/// Refusals happen before mutation. Corrupt storage or post-write failures trap.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Error {
    /// Maintenance is restricted to the actual original installation caller.
    NotOwner,
    /// Increment is closed by the current fence.
    Fenced,
    /// The counter cannot grow beyond u64::MAX.
    Overflow,
    /// Attempt is zero, already spent, stale or conflicts with the current fence.
    Attempt,
}

const KEY: &str = if cfg!(feature = "incompatible-schema") {
    "counter.state.v2"
} else {
    "counter.state.v1"
};

thread_local! {
    static COUNTER: RefCell<Option<StableCell<Vec<u8>>>> = const { RefCell::new(None) };
}

///
/// Record
/// Retains the counter value, installation owner and maintenance fence.
///

#[derive(Clone, Deserialize, Serialize)]
struct Record {
    owner: Principal,
    value: u64,
    last_attempt: u64,
    fence: Option<u64>,
}

impl Record {
    fn snapshot(&self) -> Snapshot {
        Snapshot {
            value: self.value,
            fence: self.fence,
            executable: if cfg!(feature = "counter-v2") { 2 } else { 1 },
        }
    }

    fn apply(&mut self, request: Request, caller: Principal) -> Result<(), Error> {
        match request {
            Request::Read => {}
            Request::Increment => {
                if self.fence.is_some() {
                    return Err(Error::Fenced);
                }
                self.value = self.value.checked_add(1).ok_or(Error::Overflow)?;
            }
            Request::Fence { attempt } | Request::Activate { attempt } => {
                if caller != self.owner {
                    return Err(Error::NotOwner);
                }
                match request {
                    Request::Fence { .. } if self.fence == Some(attempt) => {}
                    Request::Fence { .. }
                        if self.fence.is_none() && attempt > self.last_attempt =>
                    {
                        self.last_attempt = attempt;
                        self.fence = Some(attempt);
                    }
                    Request::Activate { .. } if attempt != 0 && attempt == self.last_attempt => {
                        self.fence = None;
                    }
                    _ => return Err(Error::Attempt),
                }
            }
        }

        Ok(())
    }
}

fn decode_record(bytes: &[u8]) -> Record {
    let record: Record =
        ic_wire::decode(bytes, ic_wire::Encoding::Payload, RECORD_LIMITS).expect("counter record");
    assert!(record.owner != Principal::anonymous(), "counter authority");
    assert!(
        record
            .fence
            .is_none_or(|attempt| attempt != 0 && attempt == record.last_attempt),
        "counter attempt"
    );

    record
}

fn init() {
    open(true);
}

fn post_upgrade() {
    open(false);
    #[cfg(feature = "reject-upgrade")]
    {
        let mut record = read();
        record.value = 999;
        write(&record);
        api::trap("qualification: reject after persistent write");
    }
}

fn open(fresh: bool) {
    let runtime =
        canic::storage::bootstrap(DefaultMemoryImpl::default(), fresh, "counter", &[KEY], 8);
    let initial = fresh.then(|| {
        let owner = api::caller();
        assert!(
            owner != Principal::anonymous(),
            "named installation authority required"
        );
        let initial = Record {
            owner,
            value: 0,
            last_attempt: u64::from(cfg!(feature = "initially-fenced")),
            fence: cfg!(feature = "initially-fenced").then_some(1),
        };

        ic_wire::encode(&initial, ic_wire::Encoding::Payload, RECORD_LIMITS)
            .expect("initial counter")
    });
    let cell = canic::storage::cell(&runtime, KEY, RECORD_LIMITS.bytes, initial);
    decode_record(cell.get());
    COUNTER.with_borrow_mut(|state| *state = Some(cell));
}

fn read() -> Record {
    COUNTER.with_borrow(|state| decode_record(state.as_ref().expect("counter open").get()))
}

fn write(record: &Record) {
    let bytes =
        ic_wire::encode(record, ic_wire::Encoding::Payload, RECORD_LIMITS).expect("encode counter");
    COUNTER.with_borrow_mut(|state| {
        state.as_mut().expect("counter open").set(bytes);
    });
}

fn readiness() -> Readiness {
    if COUNTER.with_borrow(Option::is_none) {
        Readiness::Unavailable
    } else if read().fence.is_some() {
        Readiness::Fenced
    } else {
        Readiness::Ready
    }
}
