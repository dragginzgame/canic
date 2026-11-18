use canic_profile_protocol::{
    BIND, CHECK, CheckResult, INFO, Info, LIMITS, ROUTE, Route, RouteError, RouteResult,
    WorkerCheck,
};
use ic_platform_runtime::{
    self as runtime,
    call::{Call, Wait},
    system as api,
};
use ic_platform_types::Principal;
use ic_storage::{
    AllocationGrants, DefaultMemoryImpl, GenericAllocationPolicy, MemoryAuthority, MemoryRequest,
    MemoryRuntime, SealedDeclarationSnapshot, StableKey, StorageConfig,
    adapter::{Bound, StableCell, Storable},
};
use std::{borrow::Cow, cell::RefCell};

struct Binding(Option<String>);

impl Storable for Binding {
    const BOUND: Bound = Bound::Bounded {
        max_size: 128,
        is_fixed_size: false,
    };
    fn to_bytes(&self) -> Cow<'_, [u8]> {
        ic_wire::encode(&self.0, ic_wire::Encoding::Payload, LIMITS)
            .map(Cow::Owned)
            .expect("entry binding")
    }
    fn into_bytes(self) -> Vec<u8> {
        self.to_bytes().into_owned()
    }
    fn from_bytes(bytes: Cow<'_, [u8]>) -> Self {
        let value: Option<String> =
            ic_wire::decode(&bytes, ic_wire::Encoding::Payload, LIMITS).expect("entry binding");
        if let Some(worker) = &value {
            valid_worker(worker).expect("worker identity");
        }
        Self(value)
    }
}

struct State {
    binding: StableCell<Binding>,
    // Keep the sole admitted backing owner alive with its issued lease.
    _runtime: MemoryRuntime<DefaultMemoryImpl>,
    checking: bool,
}

thread_local! { static STATE: RefCell<Option<State>> = const { RefCell::new(None) }; }

fn open(fresh: bool) {
    let mut runtime = MemoryRuntime::new_with_config(
        DefaultMemoryImpl::default(),
        StorageConfig {
            allocations: 1,
            extents: 2,
            pages: 16,
            ..Default::default()
        },
    )
    .expect("admit entry backing");
    if !fresh {
        assert_eq!(
            runtime.memory_allocations().expect("entry integrity").len(),
            1,
            "missing retained entry binding"
        );
    }
    runtime
        .bootstrap(
            &SealedDeclarationSnapshot::new(vec![
                MemoryRequest::new("entry", "entry.worker.v1").expect("constant request"),
            ])
            .expect("constant declarations"),
            &AllocationGrants::new(vec![
                MemoryAuthority::new("entry", "entry.").expect("constant authority"),
            ])
            .expect("constant grants"),
            &GenericAllocationPolicy,
        )
        .expect("commit entry admission");
    let memory = runtime
        .open_memory(&StableKey::parse("entry.worker.v1").expect("constant key"))
        .expect("entry lease");
    let binding = if fresh {
        StableCell::create(memory, Binding(None))
    } else {
        StableCell::load(memory).expect("strict entry opening")
    };
    STATE.with_borrow_mut(|state| {
        *state = Some(State {
            binding,
            _runtime: runtime,
            checking: false,
        })
    });
}

#[runtime::init(runtime = runtime, guard = lifecycle_guard, decode = lifecycle_decode, max_bytes = 0, on_error = endpoint_error)]
fn init((): ()) {
    open(true);
}
#[runtime::post_upgrade(runtime = runtime, guard = lifecycle_guard, decode = lifecycle_decode, max_bytes = 0, on_error = endpoint_error)]
fn post_upgrade((): ()) {
    open(false);
}

fn valid_worker(text: &str) -> Result<Principal, RouteError> {
    let principal = Principal::from_text(text).map_err(|_| RouteError::InvalidWorker)?;
    if principal == Principal::anonymous()
        || principal == Principal::management_canister()
        || principal == api::canister_self()
        || principal.to_text() != text
    {
        return Err(RouteError::InvalidWorker);
    }
    Ok(principal)
}

fn bounded() -> Result<(), String> {
    if ic_platform_runtime::system::argument_size() > LIMITS.bytes {
        return Err("entry frame too large".into());
    }
    Ok(())
}

fn decode_route(bytes: &[u8]) -> Result<(), ic_wire::Error> {
    ic_wire::decode::<()>(
        bytes,
        ic_wire::Encoding::Frame {
            method: ROUTE,
            direction: ic_wire::Direction::Request,
        },
        LIMITS,
    )
}
fn decode_bind(bytes: &[u8]) -> Result<Route, ic_wire::Error> {
    ic_wire::decode(
        bytes,
        ic_wire::Encoding::Frame {
            method: BIND,
            direction: ic_wire::Direction::Request,
        },
        LIMITS,
    )
}
fn decode_check(bytes: &[u8]) -> Result<(), ic_wire::Error> {
    ic_wire::decode::<()>(
        bytes,
        ic_wire::Encoding::Frame {
            method: CHECK,
            direction: ic_wire::Direction::Request,
        },
        LIMITS,
    )
}
#[expect(clippy::needless_pass_by_value, reason = "entrypoint owns result")]
fn encode_route(value: RouteResult) -> Result<Vec<u8>, ic_wire::Error> {
    ic_wire::encode(
        &value,
        ic_wire::Encoding::Frame {
            method: ROUTE,
            direction: ic_wire::Direction::Response,
        },
        LIMITS,
    )
}
#[expect(clippy::needless_pass_by_value, reason = "entrypoint owns result")]
fn encode_bind(value: RouteResult) -> Result<Vec<u8>, ic_wire::Error> {
    ic_wire::encode(
        &value,
        ic_wire::Encoding::Frame {
            method: BIND,
            direction: ic_wire::Direction::Response,
        },
        LIMITS,
    )
}
#[expect(clippy::needless_pass_by_value, reason = "entrypoint owns result")]
fn encode_check(value: CheckResult) -> Result<Vec<u8>, ic_wire::Error> {
    ic_wire::encode(
        &value,
        ic_wire::Encoding::Frame {
            method: CHECK,
            direction: ic_wire::Direction::Response,
        },
        LIMITS,
    )
}

#[runtime::query(runtime = runtime, guard = bounded, decode = decode_route, max_bytes = 4096, on_error = endpoint_error, name = "route", encode = encode_route)]
fn route((): ()) -> RouteResult {
    STATE
        .with_borrow(|state| {
            state
                .as_ref()
                .expect("initialized entry")
                .binding
                .get()
                .0
                .clone()
        })
        .map(|worker| Route { worker })
        .ok_or(RouteError::Unbound)
}

// Clients use replicated resolution before sending private data to a discovered worker.
#[runtime::update(runtime = runtime, guard = bounded, decode = decode_route, max_bytes = 4096, on_error = endpoint_error, name = "resolve", encode = encode_route)]
fn resolve((): ()) -> RouteResult {
    route(())
}

#[runtime::update(runtime = runtime, guard = bounded, decode = decode_bind, max_bytes = 4096, on_error = endpoint_error, name = "bind", encode = encode_bind)]
fn bind(input: Route) -> RouteResult {
    if !api::is_controller(&api::caller()) {
        return Err(RouteError::Forbidden);
    }
    valid_worker(&input.worker)?;
    STATE.with_borrow_mut(|state| {
        let state = state.as_mut().expect("initialized entry");
        if let Some(worker) = &state.binding.get().0 {
            if worker != &input.worker {
                return Err(RouteError::Conflict);
            }
        } else {
            state.binding.set(Binding(Some(input.worker.clone())));
        }
        Ok(input)
    })
}

#[runtime::update(runtime = runtime, guard = bounded, decode = decode_check, max_bytes = 4096, on_error = endpoint_error, name = "check", encode = encode_check)]
async fn check((): ()) -> CheckResult {
    if !api::is_controller(&api::caller()) {
        return Err(RouteError::Forbidden);
    }
    let route = route(())?;
    let target = valid_worker(&route.worker)?;
    let request = ic_wire::encode(
        &(),
        ic_wire::Encoding::Frame {
            method: INFO,
            direction: ic_wire::Direction::Request,
        },
        LIMITS,
    )
    .expect("constant request");
    STATE.with_borrow_mut(|state| {
        let state = state.as_mut().expect("initialized entry");
        if state.checking {
            return Err(RouteError::Busy);
        }
        state.checking = true;
        Ok(())
    })?;
    let response = Call::new(target, "probe", Wait::Bounded { seconds: 5 })
        .owned_args(request)
        .execute()
        .await;
    STATE.with_borrow_mut(|state| state.as_mut().expect("initialized entry").checking = false);
    let response = response.map_err(|_| RouteError::Unavailable)?;
    let response = match response.into_parts().0 {
        ic_platform_runtime::call::Outcome::Reply(bytes) => bytes,
        _ => return Err(RouteError::Unavailable),
    };
    let info: Info = ic_wire::decode(
        &response,
        ic_wire::Encoding::Frame {
            method: INFO,
            direction: ic_wire::Direction::Response,
        },
        LIMITS,
    )
    .map_err(|_| RouteError::InvalidReply)?;
    // Bound the response encoder even when the selected worker is not our implementation.
    if info.executable.len() > 128 {
        return Err(RouteError::InvalidReply);
    }
    Ok(WorkerCheck { route, info })
}

fn endpoint_error(error: impl std::fmt::Debug) -> ! {
    ic_platform_runtime::system::trap(&format!("endpoint refused: {error:?}"))
}

fn lifecycle_guard() -> Result<(), std::convert::Infallible> {
    Ok(())
}

fn lifecycle_decode(_bytes: &[u8]) -> Result<(), std::convert::Infallible> {
    Ok(())
}
