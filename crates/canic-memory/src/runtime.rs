use std::cell::RefCell;

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

thread_local! {
    static CANIC_EAGER_TLS: RefCell<Vec<fn()>> = const {
        RefCell::new(Vec::new())
    };
}

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
    let funcs = CANIC_EAGER_TLS.with(|v| {
        let mut v = v.borrow_mut();
        std::mem::take(&mut *v)
    });

    debug_assert!(
        CANIC_EAGER_TLS.with(|v| v.borrow().is_empty()),
        "CANIC_EAGER_TLS was modified during init_eager_tls() execution"
    );

    for f in funcs {
        f();
    }
}

/// Register a TLS initializer function for eager execution.
///
/// This is called by the `eager_static!` macro. The function pointer `f`
/// must be a zero-argument function (`fn()`) that performs a `.with(|_| {})`
/// on the thread-local static it is meant to initialize.
pub fn defer_tls_initializer(f: fn()) {
    CANIC_EAGER_TLS.with_borrow_mut(|v| v.push(f));
}

///
/// TESTS
///

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    thread_local! {
        static COUNT: Cell<u32> = const { Cell::new(0) };
    }

    fn bump() {
        COUNT.with(|c| c.set(c.get() + 1));
    }

    #[test]
    fn init_eager_tls_runs_and_clears_queue() {
        COUNT.with(|c| c.set(0));
        CANIC_EAGER_TLS.with(|v| v.borrow_mut().push(bump));
        init_eager_tls();
        let first = COUNT.with(Cell::get);
        assert_eq!(first, 1);

        // second call sees empty queue
        init_eager_tls();
        let second = COUNT.with(Cell::get);
        assert_eq!(second, 1);
    }
}
