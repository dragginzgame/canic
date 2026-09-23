//! Module: memory::runtime
//!
//! Responsibility: enforce memory bootstrap readiness before lazy store access.
//! Does not own: stable schema definitions, allocation policy, or lifecycle hooks.
//! Boundary: macros and lifecycle call this before stable-memory-backed statics are used.

#[cfg(any(test, debug_assertions))]
use std::sync::Mutex;

#[cfg(any(test, debug_assertions))]
static TEST_BOOTSTRAP_HOOK: Mutex<Option<fn()>> = Mutex::new(None);

/// Return whether memory access is currently allowed during bootstrap.
pub fn is_memory_bootstrap_ready() -> Result<bool, ic_memory::RuntimeStateError> {
    ic_memory::is_default_memory_manager_bootstrapped()
}

/// Panic if a stable-memory slot is touched before memory bootstrap is ready.
///
/// # Panics
///
/// Panics when the default memory manager has not been bootstrapped before the
/// stable-memory slot identified by `label` and `id` is accessed. In tests and
/// debug builds, an installed bootstrap hook is run first and the function only
/// panics if memory remains unbootstrapped after that hook.
pub fn assert_memory_bootstrap_ready(label: &str, id: u8) {
    match is_memory_bootstrap_ready() {
        Ok(true) => return,
        Ok(false) => {}
        Err(error) => {
            panic!(
                "stable memory slot '{label}' (id {id}) could not inspect memory bootstrap: {error}"
            );
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
                    "stable memory slot '{label}' (id {id}) could not inspect memory bootstrap after the test hook: {error}"
                );
            }
        }
    }

    panic!(
        "stable memory slot '{label}' (id {id}) accessed before memory bootstrap; call ic_memory::bootstrap_default_memory_manager_with_policy(...) first"
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
