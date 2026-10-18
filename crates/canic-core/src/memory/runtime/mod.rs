//! Module: memory::runtime
//!
//! Responsibility: retain the Canic-owned runtime and enforce readiness before store access.
//! Does not own: stable schema definitions, allocation policy, or lifecycle hooks.
//! Boundary: macros and lifecycle call this before stable-memory-backed statics are used.

use ic_memory::{MemoryRuntime, RuntimeDiagnosticError, ic_stable_structures::DefaultMemoryImpl};
use std::cell::RefCell;

#[cfg(any(test, debug_assertions))]
use std::sync::Mutex;

#[cfg(any(test, debug_assertions))]
static TEST_BOOTSTRAP_HOOK: Mutex<Option<fn()>> = Mutex::new(None);

thread_local! {
    pub(super) static RUNTIME: RefCell<Option<MemoryRuntime<DefaultMemoryImpl>>> = const { RefCell::new(None) };
}

/// Inspect the Canic-owned runtime without constructing memory on a read path.
pub(crate) fn with_runtime<T>(
    inspect: impl FnOnce(&MemoryRuntime<DefaultMemoryImpl>) -> Result<T, RuntimeDiagnosticError>,
) -> Result<T, RuntimeDiagnosticError> {
    RUNTIME.with(|cell| {
        let runtime = cell
            .try_borrow()
            .map_err(|_| RuntimeDiagnosticError::ReentrantAccess)?;
        inspect(
            runtime
                .as_ref()
                .ok_or(RuntimeDiagnosticError::NotBootstrapped)?,
        )
    })
}

/// Return whether the lifecycle owner has committed the host composition.
pub fn is_memory_bootstrap_ready() -> Result<bool, RuntimeDiagnosticError> {
    RUNTIME.with(|cell| {
        let runtime = cell
            .try_borrow()
            .map_err(|_| RuntimeDiagnosticError::ReentrantAccess)?;
        Ok(runtime.as_ref().is_some_and(MemoryRuntime::is_bootstrapped))
    })
}

/// Panic if a stable-memory slot is touched before memory bootstrap is ready.
///
/// # Panics
///
/// Panics when the Canic-owned memory runtime has not been bootstrapped before the
/// stable-memory slot identified by `key` is accessed. In tests and
/// debug builds, an installed bootstrap hook is run first and the function only
/// panics if memory remains unbootstrapped after that hook.
pub fn assert_memory_bootstrap_ready(key: &str) {
    match is_memory_bootstrap_ready() {
        Ok(true) => return,
        Ok(false) => {}
        Err(error) => {
            panic!("stable memory slot '{key}' could not inspect memory bootstrap: {error}");
        }
    }

    #[cfg(any(test, debug_assertions))]
    {
        run_test_bootstrap_hook();
        match is_memory_bootstrap_ready() {
            Ok(true) => return,
            Ok(false) => {}
            Err(error) => {
                panic!(
                    "stable memory slot '{key}' could not inspect memory bootstrap after the test hook: {error}"
                );
            }
        }
    }

    panic!(
        "stable memory slot '{key}' accessed before memory bootstrap; run Canic memory bootstrap with its host pool first"
    );
}

/// Install a test-only hook that can run the crate's normal memory bootstrap
/// before host unit tests first touch macro-backed stable memory.
///
/// # Panics
///
/// Panics if the process-local test bootstrap hook mutex is poisoned.
#[cfg(any(test, debug_assertions))]
pub fn install_test_bootstrap_hook(hook: fn()) {
    *TEST_BOOTSTRAP_HOOK
        .lock()
        .expect("test bootstrap hook poisoned") = Some(hook);
}

/// Return whether a test bootstrap hook has been installed.
///
/// # Panics
///
/// Panics if the process-local test bootstrap hook mutex is poisoned.
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
