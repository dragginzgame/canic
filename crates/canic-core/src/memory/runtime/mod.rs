use std::sync::Mutex;

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
#[cfg(any(test, debug_assertions))]
static TEST_BOOTSTRAP_HOOK: Mutex<Option<fn()>> = Mutex::new(None);

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

/// Return whether memory access is currently allowed during bootstrap.
#[must_use]
pub fn is_memory_bootstrap_ready() -> bool {
    ic_memory::runtime::is_default_memory_manager_bootstrapped()
}

/// Panic if a stable-memory slot is touched before memory bootstrap is ready.
pub fn assert_memory_bootstrap_ready(label: &str, id: u8) {
    if is_memory_bootstrap_ready() {
        return;
    }

    #[cfg(any(test, debug_assertions))]
    {
        run_test_bootstrap_hook();
        if is_memory_bootstrap_ready() {
            return;
        }
    }

    panic!(
        "stable memory slot '{label}' (id {id}) accessed before memory bootstrap; call ic_memory::bootstrap_default_memory_manager_with_policy(...) first"
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

/// Install a test-only hook that can run the crate's normal memory bootstrap
/// before host unit tests first touch macro-backed stable memory.
#[cfg(any(test, debug_assertions))]
pub fn install_test_bootstrap_hook(hook: fn()) {
    *TEST_BOOTSTRAP_HOOK
        .lock()
        .expect("test bootstrap hook poisoned") = Some(hook);
}

/// Return whether a test bootstrap hook has been installed.
#[cfg(any(test, debug_assertions))]
#[must_use]
pub fn has_test_bootstrap_hook() -> bool {
    TEST_BOOTSTRAP_HOOK
        .lock()
        .expect("test bootstrap hook poisoned")
        .is_some()
}

#[cfg(any(test, debug_assertions))]
fn run_test_bootstrap_hook() {
    let hook = *TEST_BOOTSTRAP_HOOK
        .lock()
        .expect("test bootstrap hook poisoned");
    if let Some(hook) = hook {
        hook();
    }
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{
        Mutex,
        atomic::{AtomicU32, Ordering},
    };

    static COUNT: AtomicU32 = AtomicU32::new(0);
    static TEST_LOCK: Mutex<()> = Mutex::new(());

    fn clear_test_queues() {
        CANIC_EAGER_TLS
            .lock()
            .expect("eager tls queue poisoned")
            .clear();
    }

    fn bump() {
        COUNT.fetch_add(1, Ordering::SeqCst);
    }

    #[test]
    fn init_eager_tls_runs_and_clears_queue() {
        let _guard = TEST_LOCK.lock().expect("test lock poisoned");
        clear_test_queues();
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
}
