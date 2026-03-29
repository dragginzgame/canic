pub mod registry;

use std::sync::{
    Mutex,
    atomic::{AtomicBool, Ordering},
};

// -----------------------------------------------------------------------------
// CANIC_EAGER_TLS
// -----------------------------------------------------------------------------
// Internal registry of "TLS touch" functions.
//
// Each function must be a plain `fn()` pointer (not a closure). When invoked,
// the function must perform a `.with(|_| {})` on a thread_local! static.
// This guarantees that the TLS slot is *initialized eagerly*, not lazily, so
// stable memory pages or other backing buffers are allocated in a deterministic
// order before any canister entry points are executed.
//
// These functions are registered by the `eager_static!` macro via
// `defer_tls_initializer()`, and run once during process startup by
// `init_eager_tls()`.
// -----------------------------------------------------------------------------

static CANIC_EAGER_TLS: Mutex<Vec<fn()>> = Mutex::new(Vec::new());
static CANIC_EAGER_TLS_RUNNING: AtomicBool = AtomicBool::new(false);
static CANIC_EAGER_INIT: Mutex<Vec<fn()>> = Mutex::new(Vec::new());

/// Run all deferred TLS initializers and clear the registry.
///
/// This drains the internal queue of initializer functions and invokes
/// each *exactly once*. The use of `std::mem::take` ensures:
///
/// - the vector is fully emptied before we run any initializers
/// - we drop the borrow before calling user code (prevents borrow panics)
/// - functions cannot be re-run accidentally
/// - reentrant modifications of the queue become visible *after* this call
///
/// This should be invoked before any IC canister lifecycle hooks (init, update,
/// heartbeat, etc.) so that thread-local caches are in a fully-initialized state
/// before the canister performs memory-dependent work.
pub fn init_eager_tls() {
    ///
    /// RunningGuard
    ///

    struct RunningGuard;

    impl Drop for RunningGuard {
        fn drop(&mut self) {
            CANIC_EAGER_TLS_RUNNING.store(false, Ordering::SeqCst);
        }
    }

    CANIC_EAGER_TLS_RUNNING.store(true, Ordering::SeqCst);
    let _running_guard = RunningGuard;

    let funcs = {
        let mut funcs = CANIC_EAGER_TLS.lock().expect("eager tls queue poisoned");
        std::mem::take(&mut *funcs)
    };

    debug_assert!(
        CANIC_EAGER_TLS
            .lock()
            .expect("eager tls queue poisoned")
            .is_empty(),
        "CANIC_EAGER_TLS was modified during init_eager_tls() execution"
    );

    for f in funcs {
        f();
    }
}

/// Run all registered eager-init hooks and clear the registry.
///
/// This drains the internal queue of eager-init functions and invokes each
/// exactly once. Canic uses this during synchronous lifecycle bootstrap after
/// eager TLS initialization and before the memory registry is committed.
pub fn run_registered_eager_init() {
    let funcs = {
        let mut funcs = CANIC_EAGER_INIT.lock().expect("eager init queue poisoned");
        std::mem::take(&mut *funcs)
    };

    debug_assert!(
        CANIC_EAGER_INIT
            .lock()
            .expect("eager init queue poisoned")
            .is_empty(),
        "CANIC_EAGER_INIT was modified during eager init execution"
    );

    for f in funcs {
        f();
    }
}

#[must_use]
pub fn is_eager_tls_initializing() -> bool {
    CANIC_EAGER_TLS_RUNNING.load(Ordering::SeqCst)
}

#[must_use]
pub fn is_memory_bootstrap_ready() -> bool {
    is_eager_tls_initializing() || registry::MemoryRegistryRuntime::is_initialized()
}

pub fn assert_memory_bootstrap_ready(label: &str, id: u8) {
    if is_memory_bootstrap_ready() {
        return;
    }

    panic!(
        "stable memory slot '{label}' (id {id}) accessed before memory bootstrap; call init_eager_tls() and MemoryRegistryRuntime::init(...) first"
    );
}

/// Register a TLS initializer function for eager execution.
///
/// This is called by the `eager_static!` macro. The function pointer `f`
/// must be a zero-argument function (`fn()`) that performs a `.with(|_| {})`
/// on the thread-local static it is meant to initialize.
pub fn defer_tls_initializer(f: fn()) {
    CANIC_EAGER_TLS
        .lock()
        .expect("eager tls queue poisoned")
        .push(f);
}

/// Register an eager-init function for lifecycle bootstrap execution.
pub fn defer_eager_init(f: fn()) {
    CANIC_EAGER_INIT
        .lock()
        .expect("eager init queue poisoned")
        .push(f);
}

///
/// TESTS
///

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    static COUNT: AtomicU32 = AtomicU32::new(0);

    fn bump() {
        COUNT.fetch_add(1, Ordering::SeqCst);
    }

    #[test]
    fn init_eager_tls_runs_and_clears_queue() {
        COUNT.store(0, Ordering::SeqCst);
        CANIC_EAGER_TLS
            .lock()
            .expect("eager tls queue poisoned")
            .push(bump);
        init_eager_tls();
        let first = COUNT.load(Ordering::SeqCst);
        assert_eq!(first, 1);

        // second call sees empty queue
        init_eager_tls();
        let second = COUNT.load(Ordering::SeqCst);
        assert_eq!(second, 1);
    }

    #[test]
    fn run_registered_eager_init_runs_and_clears_queue() {
        COUNT.store(0, Ordering::SeqCst);
        CANIC_EAGER_INIT
            .lock()
            .expect("eager init queue poisoned")
            .push(bump);
        run_registered_eager_init();
        let first = COUNT.load(Ordering::SeqCst);
        assert_eq!(first, 1);

        // second call sees empty queue
        run_registered_eager_init();
        let second = COUNT.load(Ordering::SeqCst);
        assert_eq!(second, 1);
    }
}
