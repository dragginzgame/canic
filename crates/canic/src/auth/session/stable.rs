//! Explicit checkpoint lifecycle for the standard Ed25519 receiver.
use super::{Engine, STATE, actor::configuration, initialize_with};
use crate::auth::install::{Install, Storage};
use ic_auth::receiver_memory::StableReceiver;
use ic_platform_runtime::system as api;
use ic_storage::{DefaultMemoryImpl, MemoryRuntime, StableKey, adapter::StableCell};
use std::cell::RefCell;

/// Exclusive IC Auth engine checkpoint.
pub const ENGINE_KEY: &str = "auth.receiver.engine.v1";
/// Protected install configuration and Canic challenge sequence.
pub const HOST_KEY: &str = "auth.receiver.host.v1";
const LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 16 * 1024,
    allocation: 256 * 1024,
    nodes: 16384,
    depth: 24,
};

thread_local! {
    static HOST: RefCell<Option<StableCell<Vec<u8>>>> = const { RefCell::new(None) };
}

pub(crate) fn layout(runtime: &MemoryRuntime<DefaultMemoryImpl>) -> Storage {
    let exists = |name| {
        runtime
            .allocation_id(&StableKey::parse(name).expect("Auth key"))
            .is_some()
    };
    match (exists(ENGINE_KEY), exists(HOST_KEY)) {
        (true, true) => Storage::Stable,
        (false, false) => Storage::Volatile,
        _ => panic!("incomplete Auth receiver storage"),
    }
}

/// Open the receiver using leases from the application's one bootstrap.
/// Supply its approved envelope only on fresh install. Upgrade consumes the
/// checkpoint with the original configuration and requires a new Root refresh.
///
/// # Panics
/// Traps on missing grants, incompatible storage or a refused checkpoint.
pub fn open(runtime: &MemoryRuntime<DefaultMemoryImpl>, install: Option<&Install>) {
    open_with_login_limit(runtime, install, None);
}

/// Open or reopen using the receiver mount’s absolute login ceiling.
///
/// # Panics
/// Traps if the configured policy or retained checkpoint is refused.
#[doc(hidden)]
pub fn open_with_login_limit(
    runtime: &MemoryRuntime<DefaultMemoryImpl>,
    install: Option<&Install>,
    max_login_secs: Option<u64>,
) {
    let mode = layout(runtime);
    if let Some(input) = install {
        assert_eq!(mode, input.storage, "approved Auth receiver storage");
    }
    if mode == Storage::Volatile {
        if let Some(input) = install {
            super::install_with_login_limit(input, max_login_secs);
        }
        return;
    }
    let initial = install.map(|input| {
        ic_wire::encode(
            &(input, None::<&crate::auth::enrollment::Receiver>),
            ic_wire::Encoding::Payload,
            LIMITS,
        )
        .expect("initial Auth host record")
    });
    let host = crate::storage::cell(runtime, HOST_KEY, LIMITS.bytes, initial);
    let (input, saved): (Install, Option<crate::auth::enrollment::Receiver>) =
        ic_wire::decode(host.get(), ic_wire::Encoding::Payload, LIMITS)
            .expect("retained Auth installation");
    assert_eq!(
        saved.is_none(),
        install.is_some(),
        "Auth host checkpoint lifecycle"
    );
    assert_eq!(
        input.storage,
        Storage::Stable,
        "stable Auth installation required"
    );
    let lease = runtime
        .open_memory(&StableKey::parse(ENGINE_KEY).expect("engine key"))
        .expect("Auth receiver lease");
    let (config, policy) = configuration(&input, max_login_secs);
    let now = api::time();
    initialize_with(config, policy, api::canister_self(), now, |config| {
        let engine = if install.is_some() {
            StableReceiver::new_fresh(lease, config, now)
        } else {
            StableReceiver::reopen(lease, config, now)
        };
        Engine::Stable(engine.unwrap_or_else(|error| {
            api::trap(&format!("open Auth receiver checkpoint: {error:?}"))
        }))
    });
    if let Some(saved) = saved {
        STATE.with_borrow_mut(|s| {
            s.as_mut()
                .expect("open Auth receiver")
                .enrollment
                .reopen(saved, now)
        });
    }
    HOST.with_borrow_mut(|slot| *slot = Some(host));
}

/// Seal both owners in the pre-upgrade transaction.
///
/// # Panics
/// A failed seal must trap so the IC rolls back the upgrade.
pub fn seal() {
    if HOST.with_borrow(Option::is_none) {
        return;
    }
    STATE.with_borrow_mut(|s| {
        let state = s.as_mut().expect("open Auth receiver");
        let Engine::Stable(engine) = &mut state.receiver else {
            panic!("expected stable receiver")
        };
        engine.seal().expect("seal Auth receiver");
        HOST.with_borrow_mut(|slot| {
            let host = slot.as_mut().expect("Auth host checkpoint");
            let (input, _): (Install, Option<crate::auth::enrollment::Receiver>) =
                ic_wire::decode(host.get(), ic_wire::Encoding::Payload, LIMITS)
                    .expect("original Auth installation");
            let bytes = ic_wire::encode(
                &(input, Some(&state.enrollment)),
                ic_wire::Encoding::Payload,
                LIMITS,
            )
            .expect("seal Auth host record");
            host.set(bytes);
        });
    });
}
