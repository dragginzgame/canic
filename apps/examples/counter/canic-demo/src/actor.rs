//! Explicit lifecycle and local application handlers; Storage owns the backing.
use crate::{Error, LIMITS, METHOD, Request, Snapshot};
use canic::readiness::State as Readiness;
use ic_platform_runtime::{self as runtime, system as api};
use ic_platform_types::Principal;
use ic_storage::{DefaultMemoryImpl, adapter::StableCell};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;

const KEY: &str = if cfg!(feature = "incompatible-schema") {
    "counter.state.v2"
} else {
    "counter.state.v1"
};

thread_local! {
    static COUNTER: RefCell<Option<StableCell<Vec<u8>>>> = const { RefCell::new(None) };
}

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
        ic_wire::decode(bytes, ic_wire::Encoding::Payload, LIMITS).expect("counter record");
    assert!(record.owner != Principal::anonymous(), "counter authority");
    assert!(
        record
            .fence
            .is_none_or(|attempt| attempt != 0 && attempt == record.last_attempt),
        "counter attempt"
    );
    record
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
        ic_wire::encode(&initial, ic_wire::Encoding::Payload, LIMITS).expect("initial counter")
    });
    let cell = canic::storage::cell(&runtime, KEY, LIMITS.bytes, initial);
    decode_record(cell.get());
    COUNTER.with_borrow_mut(|state| *state = Some(cell));
}

fn read() -> Record {
    COUNTER.with_borrow(|state| decode_record(state.as_ref().expect("counter open").get()))
}

fn write(record: &Record) {
    let bytes =
        ic_wire::encode(record, ic_wire::Encoding::Payload, LIMITS).expect("encode counter");
    COUNTER.with_borrow_mut(|state| {
        state.as_mut().expect("counter open").set(bytes);
    });
}

canic::mount_readiness_endpoint!(crate::actor::readiness);

fn readiness() -> Readiness {
    if COUNTER.with_borrow(Option::is_none) {
        Readiness::Unavailable
    } else if read().fence.is_some() {
        Readiness::Fenced
    } else {
        Readiness::Ready
    }
}

/// Create the counter and bind its fence authority to the installer.
#[runtime::init(
    runtime = runtime,
    guard = lifecycle_guard,
    decode = lifecycle_decode,
    max_bytes = 0,
    on_error = fail,
)]
fn init((): ()) {
    open(true);
}

/// Reopen the demo state from the preceding executable.
#[runtime::post_upgrade(
    runtime = runtime,
    guard = lifecycle_guard,
    decode = lifecycle_decode,
    max_bytes = 0,
    on_error = fail,
)]
fn post_upgrade((): ()) {
    open(false);
    #[cfg(feature = "reject-upgrade")]
    {
        let mut record = read();
        record.value = 999;
        write(&record);
        api::trap("qualification: reject after persistent write");
    }
}

/// Read or increment the public counter, or change its fence as the recorded owner.
#[runtime::update(
    runtime = runtime,
    guard = bounded,
    decode = decode,
    max_bytes = 256,
    on_error = fail,
    name = "counter",
    encode = encode,
)]
fn counter(request: Request) -> Result<Snapshot, Error> {
    let mut record = read();
    record.apply(request, api::caller())?;
    let snapshot = record.snapshot();
    if request != Request::Read {
        write(&record);
    }
    Ok(snapshot)
}

/// Reject oversized frames before copying the request. The handler owns operation-specific checks.
fn bounded() -> Result<(), &'static str> {
    if api::argument_size() > LIMITS.bytes {
        return Err("counter frame too large");
    }
    Ok(())
}

fn decode(bytes: &[u8]) -> Result<Request, ic_wire::Error> {
    ic_wire::decode(
        bytes,
        ic_wire::Encoding::Frame {
            method: METHOD,
            direction: ic_wire::Direction::Request,
        },
        LIMITS,
    )
}

fn encode(value: Result<Snapshot, Error>) -> Result<Vec<u8>, ic_wire::Error> {
    ic_wire::encode(
        &value,
        ic_wire::Encoding::Frame {
            method: METHOD,
            direction: ic_wire::Direction::Response,
        },
        LIMITS,
    )
}

/// The IC admits lifecycle calls. The declared argument budget still applies.
fn lifecycle_guard() -> Result<(), std::convert::Infallible> {
    Ok(())
}

fn lifecycle_decode(_bytes: &[u8]) -> Result<(), std::convert::Infallible> {
    Ok(())
}

fn fail(error: impl std::fmt::Debug) -> ! {
    api::trap(&format!("counter refused: {error:?}"))
}

/// Apply the recovery command under the counter's retained ownership and fence.
#[runtime::update(
    runtime = runtime,
    name = "recovery",
    guard = bounded,
    decode = decode_recovery,
    max_bytes = 128,
    encode = encode_recovery,
    on_error = fail,
)]
fn recovery(command: canic::recovery::Command) -> canic::recovery::Command {
    use canic::recovery::Command;
    let mut record = read();
    let caller = api::caller();
    assert_eq!(record.owner, caller, "counter recovery owner");
    match command {
        Command::Fence(attempt) => record
            .apply(Request::Fence { attempt }, caller)
            .expect("counter fence"),
        Command::Open(attempt) => {
            assert_eq!(record.fence, Some(attempt), "counter opening generation");
            record
                .apply(Request::Activate { attempt }, caller)
                .expect("counter opening");
        }
        Command::Accept {
            checkpoint,
            generation,
        } => {
            assert_eq!(record.fence, Some(checkpoint), "counter checkpoint");
            assert!(generation > record.last_attempt, "fresh counter generation");
            record.last_attempt = generation;
            record.fence = Some(generation);
        }
    }
    write(&record);
    command
}

fn decode_recovery(bytes: &[u8]) -> Result<canic::recovery::Command, ic_wire::Error> {
    ic_wire::decode(
        bytes,
        ic_wire::Encoding::Frame {
            method: canic::recovery::METHOD,
            direction: ic_wire::Direction::Request,
        },
        canic::recovery::LIMITS,
    )
}

fn encode_recovery(command: canic::recovery::Command) -> Result<Vec<u8>, ic_wire::Error> {
    canic::recovery::encode(command, ic_wire::Direction::Response)
}
