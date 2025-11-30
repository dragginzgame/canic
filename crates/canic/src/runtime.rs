use std::cell::RefCell;

//
// CANIC_EAGER_TLS
//
// Holds a list of closures that, when called, will "touch" each thread_local!
// static and force its initialization. This ensures that *any* TLS value
// registered via `canic_thread_local!` (or similar macros) will be eagerly
// initialized, instead of lazily on first use.
//

thread_local! {
    /// Registry of closures that force eager initialization of TLS statics.
    pub static CANIC_EAGER_TLS: RefCell<Vec<fn()>> = const {
        RefCell::new(Vec::new())
    };
}

/// Ensure all eager TLS statics are touched before memory init runs.
///
/// This drains (not clones) the list of registered TLS initializer closures,
/// and invokes each one exactly once. Using `std::mem::take` ensures:
///
/// - we avoid holding a mutable borrow while calling user-registered closures
/// - closures cannot be called twice (the vector is emptied atomically)
/// - no accidental re-entrancy can append while we iterate
/// - the registry remains empty after initialization, preventing repeats
///
/// This must run *before* memory initialization so that all thread_local!
/// statics which allocate or register stable memory segments are forced to
/// initialize deterministically and contribute their memory IDs in a stable
/// order. Without this eager pass, TLS statics would initialize lazily on
/// first use, which could cause non-deterministic memory registration during
/// canister execution.
pub fn init_eager_tls() {
    // Atomically take ownership of the initializer list and leave it empty.
    let funcs = CANIC_EAGER_TLS.with(|v| {
        let mut v = v.borrow_mut();
        std::mem::take(&mut *v)
    });

    // Invoke all registered TLS initializers.
    for f in funcs {
        f();
    }
}
