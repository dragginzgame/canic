//! Module: memory::runtime
//!
//! Responsibility: retain the Canic-owned runtime and enforce readiness before store access.
//! Does not own: stable schema definitions, allocation policy, or lifecycle hooks.
//! Boundary: macros and lifecycle call this before stable-memory-backed statics are used.

use crate::memory::{MemoryBootstrap, MemoryBootstrapError, configured_bucket_pages, policy};
use ic_memory::{DefaultMemoryImpl, MemoryRuntime, RuntimeDiagnosticError};
use std::{cell::RefCell, rc::Rc};

#[cfg(any(test, debug_assertions))]
use std::sync::Mutex;

#[cfg(any(test, debug_assertions))]
static TEST_BOOTSTRAP_HOOK: Mutex<Option<fn()>> = Mutex::new(None);

thread_local! {
    pub(super) static RUNTIME: RefCell<Option<Rc<MemoryRuntime<DefaultMemoryImpl>>>> = const { RefCell::new(None) };
}

/// Bootstrap exactly the supplied host composition before any store opens.
pub fn bootstrap(input: MemoryBootstrap) -> Result<(), MemoryBootstrapError> {
    let declarations = ic_memory::SealedDeclarationSnapshot::new(input.requests)
        .map_err(ic_memory::MemoryResolutionError::from)
        .map_err(ic_memory::RuntimeBootstrapError::from)?;
    let config = ic_memory::MemoryManagerConfig::new(configured_bucket_pages())?;
    let mut policy = policy::CanicMemoryManagerPolicy::new();
    if let Some(participant) = input.admission {
        policy = policy.with_admission(participant);
    }
    RUNTIME.with(|cell| -> Result<(), MemoryBootstrapError> {
        let mut owner = cell
            .try_borrow_mut()
            .map_err(|_| RuntimeDiagnosticError::ReentrantAccess)?;
        if let Some(runtime) = owner.as_ref() {
            runtime.verify_bootstrap(&declarations, &input.pool, &policy)?;
        } else {
            let mut runtime = MemoryRuntime::new_with_config(DefaultMemoryImpl::default(), config)?;
            runtime.bootstrap(&declarations, &input.pool, &policy)?;
            // Share only committed authority. Failed admission publishes no
            // runtime, so a consumer cannot mistake partial setup for readiness.
            *owner = Some(Rc::new(runtime));
        }
        Ok(())
    })?;
    super::pool::remember(input.pool);
    Ok(())
}

/// Share the exact committed host runtime with a synchronous storage participant.
///
/// This clones its `Rc`, never its backing memory or allocation leases. Bind the
/// returned instance before opening consumer stores. Absence and reentrant
/// access fail without constructing or bootstrapping another manager.
pub fn shared_runtime() -> Result<Rc<MemoryRuntime<DefaultMemoryImpl>>, RuntimeDiagnosticError> {
    RUNTIME.with(|cell| {
        let owner = cell
            .try_borrow()
            .map_err(|_| RuntimeDiagnosticError::ReentrantAccess)?;
        let runtime = owner
            .as_ref()
            .ok_or(RuntimeDiagnosticError::NotBootstrapped)?;
        Ok(Rc::clone(runtime))
    })
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
        Ok(runtime
            .as_ref()
            .is_some_and(|runtime| runtime.is_bootstrapped()))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handoff_requires_bootstrap_and_shares_one_allocation_authority() {
        std::thread::spawn(|| {
            assert!(matches!(
                shared_runtime(),
                Err(RuntimeDiagnosticError::NotBootstrapped)
            ));
            crate::memory::bootstrap_framework().unwrap();
            let runtime = shared_runtime().unwrap();
            assert!(Rc::ptr_eq(&runtime, &shared_runtime().unwrap()));
            let key = crate::memory::keys::RUNTIME_BINDINGS_V1;
            let memory = crate::memory::open(key);
            assert_eq!(
                runtime.memory_id(key).unwrap(),
                with_runtime(|runtime| { Ok(runtime.memory_id(key).unwrap()) }).unwrap()
            );
            drop(memory);
            crate::memory::bootstrap(crate::memory::MemoryBootstrap {
                requests: crate::memory::requests().unwrap(),
                pool: crate::memory::pool::framework_pool(Vec::new(), Vec::new()).unwrap(),
                admission: None,
            })
            .unwrap();
            assert!(Rc::ptr_eq(&runtime, &shared_runtime().unwrap()));
        })
        .join()
        .unwrap();
    }
}
