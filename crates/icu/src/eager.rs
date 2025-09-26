use std::cell::RefCell;

//
// ICU_EAGER_TLS
//
// Holds a list of closures that, when called, will "touch" each thread_local!
// static and force its initialization. This ensures that *any* TLS value
// registered via `icu_thread_local!` (or similar macros) will be eagerly
// initialized, instead of lazily on first use.
//

thread_local! {
    /// Registry of closures that force eager initialization of TLS statics.
    pub static ICU_EAGER_TLS: RefCell<Vec<fn()>> = const {
        RefCell::new(Vec::new())
    };
}

/// Ensure all eager TLS statics are touched before memory init runs.
pub fn init_eager_tls() {
    // Drain into a temporary Vec so we don't hold the borrow
    // while invoking the closures.
    let funcs: Vec<_> = ICU_EAGER_TLS.with(|v| v.borrow().clone());
    for f in funcs {
        f();
    }
}
