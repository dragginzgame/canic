use canic_profile_protocol::{
    BIND, CHECK, CheckResult, INFO, Info, LIMITS, ROUTE, Route, RouteError, RouteResult,
    WorkerCheck,
};
use ic_platform_runtime::{
    Principal,
    call::{Call, WaitPolicy},
    cdk::{self, api},
};
use ic_storage::{
    AllocationGrants, DefaultMemoryImpl, GenericAllocationPolicy, MemoryAuthority, MemoryRequest,
    MemoryRuntime, SealedDeclarationSnapshot, StableKey, StorageConfig,
    adapter::{Bound, CodecError, StableCell, Storable},
};
use std::{borrow::Cow, cell::RefCell};

struct Binding(Option<String>);

impl Storable for Binding {
    const BOUND: Bound = Bound::Bounded {
        max_size: 128,
        is_fixed_size: false,
    };
    fn to_bytes(&self) -> Result<Cow<'_, [u8]>, CodecError> {
        ic_wire::native::encode(&self.0, LIMITS)
            .map(Cow::Owned)
            .map_err(|_| CodecError::Invalid("entry binding"))
    }
    fn from_bytes(bytes: Cow<'_, [u8]>) -> Result<Self, CodecError> {
        let value: Option<String> = ic_wire::native::decode(&bytes, LIMITS)
            .map_err(|_| CodecError::Invalid("entry binding"))?;
        if let Some(worker) = &value {
            valid_worker(worker).map_err(|_| CodecError::Invalid("worker identity"))?;
        }
        Ok(Self(value))
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
        },
    )
    .expect("admit entry backing");
    if !fresh {
        assert_eq!(
            runtime.diagnostics().expect("entry integrity").allocations,
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
        StableCell::load(memory, 128)
    }
    .expect("strict entry opening");
    STATE.with_borrow_mut(|state| {
        *state = Some(State {
            binding,
            _runtime: runtime,
            checking: false,
        })
    });
}

#[cdk::init]
fn init() {
    open(true);
}
#[cdk::post_upgrade]
fn post_upgrade() {
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
    if ic_platform_runtime::ic0::msg_arg_data_size() > LIMITS.bytes {
        return Err("entry frame too large".into());
    }
    Ok(())
}

#[expect(
    clippy::needless_pass_by_value,
    reason = "entrypoint owns decoder bytes"
)]
fn decode_route(bytes: Vec<u8>) {
    ic_wire::native::decode_request::<()>(ROUTE, &bytes, LIMITS)
        .unwrap_or_else(|_| api::trap("invalid route request"));
}
#[expect(
    clippy::needless_pass_by_value,
    reason = "entrypoint owns decoder bytes"
)]
fn decode_bind(bytes: Vec<u8>) -> Route {
    ic_wire::native::decode_request(BIND, &bytes, LIMITS)
        .unwrap_or_else(|_| api::trap("invalid bind request"))
}
#[expect(
    clippy::needless_pass_by_value,
    reason = "entrypoint owns decoder bytes"
)]
fn decode_check(bytes: Vec<u8>) {
    ic_wire::native::decode_request::<()>(CHECK, &bytes, LIMITS)
        .unwrap_or_else(|_| api::trap("invalid check request"));
}
#[expect(clippy::needless_pass_by_value, reason = "entrypoint owns result")]
fn encode_route(value: RouteResult) -> Vec<u8> {
    ic_wire::native::encode_response(ROUTE, &value, LIMITS).expect("bounded route")
}
#[expect(clippy::needless_pass_by_value, reason = "entrypoint owns result")]
fn encode_bind(value: RouteResult) -> Vec<u8> {
    ic_wire::native::encode_response(BIND, &value, LIMITS).expect("bounded route")
}
#[expect(clippy::needless_pass_by_value, reason = "entrypoint owns result")]
fn encode_check(value: CheckResult) -> Vec<u8> {
    ic_wire::native::encode_response(CHECK, &value, LIMITS).expect("bounded check")
}

#[cdk::query(
    guard = "bounded",
    decode_with = "decode_route",
    encode_with = "encode_route"
)]
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
#[cdk::update(
    guard = "bounded",
    decode_with = "decode_route",
    encode_with = "encode_route"
)]
fn resolve() -> RouteResult {
    route()
}

#[cdk::update(
    guard = "bounded",
    decode_with = "decode_bind",
    encode_with = "encode_bind"
)]
fn bind(input: Route) -> RouteResult {
    if !api::is_controller(&api::msg_caller()) {
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

#[cdk::update(
    guard = "bounded",
    decode_with = "decode_check",
    encode_with = "encode_check"
)]
async fn check() -> CheckResult {
    if !api::is_controller(&api::msg_caller()) {
        return Err(RouteError::Forbidden);
    }
    let route = route()?;
    let target = valid_worker(&route.worker)?;
    let request = ic_wire::native::encode_request(INFO, &(), LIMITS).expect("constant request");
    STATE.with_borrow_mut(|state| {
        let state = state.as_mut().expect("initialized entry");
        if state.checking {
            return Err(RouteError::Busy);
        }
        state.checking = true;
        Ok(())
    })?;
    let response = Call::new(target, "probe", WaitPolicy::Bounded { timeout_seconds: 5 })
        .take_raw_args(request)
        .execute()
        .await;
    STATE.with_borrow_mut(|state| state.as_mut().expect("initialized entry").checking = false);
    let response = response.map_err(|_| RouteError::Unavailable)?;
    let info: Info = ic_wire::native::decode_response(INFO, &response, LIMITS)
        .map_err(|_| RouteError::InvalidReply)?;
    // Bound the response encoder even when the selected worker is not our implementation.
    if info.executable.len() > 128 {
        return Err(RouteError::InvalidReply);
    }
    Ok(WorkerCheck { route, info })
}
