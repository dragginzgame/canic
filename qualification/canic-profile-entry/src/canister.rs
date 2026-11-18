use canic_profile_protocol::{
    BIND, CHECK, CheckResult, LIMITS, ROUTE, Route, RouteError, RouteResult, WorkerCheck,
};
use ic_platform_runtime::{self as runtime, system as api};
use ic_platform_types::Principal;
use ic_storage::{
    DefaultMemoryImpl, MemoryRuntime, StableKey, StorageConfig,
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
    let runtime = canic::storage::bootstrap_with_prefix(
        DefaultMemoryImpl::default(),
        fresh,
        "entry",
        "entry.",
        &["entry.worker.v1"],
        StorageConfig {
            allocations: 1,
            extents: 2,
            pages: 16,
            ..Default::default()
        },
    );
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

#[canic::query(method = canic_profile_protocol::ROUTE, limits = canic_profile_protocol::LIMITS, reply_limits = canic_profile_protocol::LIMITS)]
fn route() -> RouteResult {
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
#[canic::update(name = "resolve", method = ROUTE, limits = LIMITS)]
fn replicated_route() -> RouteResult {
    route()
}

#[canic::update(method = BIND, limits = LIMITS)]
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

#[canic::update(method = CHECK, limits = LIMITS)]
async fn check() -> CheckResult {
    if !api::is_controller(&api::caller()) {
        return Err(RouteError::Forbidden);
    }
    let route = route()?;
    let target = valid_worker(&route.worker)?;
    STATE.with_borrow_mut(|state| {
        let state = state.as_mut().expect("initialized entry");
        if state.checking {
            return Err(RouteError::Busy);
        }
        state.checking = true;
        Ok(())
    })?;
    let response = canic::method::call::<canic_profile_protocol::Probe>(target, 5, &()).await;
    STATE.with_borrow_mut(|state| state.as_mut().expect("initialized entry").checking = false);
    let info = response.map_err(|error| match error {
        canic::method::CallError::Reply(_) => RouteError::InvalidReply,
        _ => RouteError::Unavailable,
    })?;
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
