//! Module: memory
//!
//! Responsibility: adapt Canic stable-memory declarations to `ic-storage` bootstrap.
//! Also owns physical stable-memory extent observation for runtime metrics.
//! Does not own: stable data schemas, ops storage APIs, or lifecycle orchestration.
//! Boundary: lifecycle initializes this before stable structures are accessed.

#[cfg(test)]
pub(crate) mod fixtures;

pub mod admission;
pub mod grants;
pub(crate) mod keys;
mod policy;
pub mod registry;
pub mod runtime;

pub use crate::memory_allocation_grants;
pub use policy::CanicStoragePolicy;

/// Stable allocation-policy authority for Canic core memory declarations.
pub const CANIC_CORE_MEMORY_AUTHORITY: &str = "canic-core";
/// Stable allocation-policy authority for Canic control-plane memory declarations.
pub const CANIC_CONTROL_PLANE_MEMORY_AUTHORITY: &str = "canic-control-plane";

const MEMORY_AUTHORITY: &str = CANIC_CORE_MEMORY_AUTHORITY;

/// Observe the current physical stable-memory extent without allocating or reading data.
#[cfg(target_arch = "wasm32")]
pub(crate) fn stable_extent_bytes() -> u64 {
    ic_platform::cdk::api::stable_size().saturating_mul(65_536)
}

/// Complete framework requests, including stores whose lazy initializers have not run.
pub fn requests() -> Result<Vec<ic_storage::MemoryRequest>, ic_storage::AdmissionError> {
    keys::requests()
}

/// Open a key already committed by the host's explicit bootstrap.
///
/// # Panics
/// Panics if the key is invalid, the host has not bootstrapped or omitted this key.
#[must_use]
pub fn open(key: &str) -> ic_storage::RuntimeMemory<ic_storage::DefaultMemoryImpl> {
    let stable_key = ic_storage::StableKey::parse(key).expect("declared stable key");
    runtime::assert_memory_bootstrap_ready(stable_key.as_str());
    runtime::with_runtime(|runtime| Ok(runtime.open_memory(&stable_key)))
        .expect("initialized memory runtime")
        .expect("committed stable key")
}

/// Host-owned component requests, grants and optional pre-commit admission.
pub struct MemoryBootstrap {
    /// Complete component requests, including the selected framework.
    pub requests: Vec<ic_storage::MemoryRequest>,
    /// Complete non-overlapping host namespace grants.
    pub grants: ic_storage::AllocationGrants,
    /// One composed pre-commit callback with a stable semantic identity.
    pub admission: Option<admission::MemoryBootstrapAdmission>,
}

/// Failure to construct, borrow or bootstrap the Canic-owned memory runtime.
#[derive(Debug, thiserror::Error)]
pub enum MemoryBootstrapError {
    /// Backing memory or resource limits could not be admitted.
    #[error(transparent)]
    Construction(#[from] ic_storage::RuntimeConstructionError),
    /// The runtime is already borrowed by a synchronous participant.
    #[error(transparent)]
    Access(#[from] runtime::MemoryRuntimeAccessError),
    /// The complete host composition could not be committed.
    #[error(transparent)]
    Bootstrap(#[from] ic_storage::RuntimeBootstrapError<registry::MemoryRegistryError>),
}

/// Bootstrap exactly the supplied host composition before any store opens.
pub fn bootstrap(input: MemoryBootstrap) -> Result<(), MemoryBootstrapError> {
    let declarations = ic_storage::SealedDeclarationSnapshot::new(input.requests)
        .map_err(ic_storage::RuntimeBootstrapError::from)?;
    let config = ic_storage::StorageConfig::default();
    let mut policy = policy::CanicStoragePolicy::new();
    if let Some(participant) = input.admission {
        policy = policy.with_admission(participant);
    }
    runtime::RUNTIME.with(|cell| -> Result<(), MemoryBootstrapError> {
        let mut owner = cell
            .try_borrow_mut()
            .map_err(|_| runtime::MemoryRuntimeAccessError::ReentrantAccess)?;
        if owner.is_none() {
            *owner = Some(std::rc::Rc::new(
                ic_storage::MemoryRuntime::new_with_config(
                    ic_storage::DefaultMemoryImpl::default(),
                    config,
                )?,
            ));
        }
        let runtime = owner
            .as_mut()
            .ok_or(runtime::MemoryRuntimeAccessError::NotBootstrapped)?;
        if runtime.is_bootstrapped() {
            runtime.verify_bootstrap(&declarations, &input.grants, &policy)?;
        } else {
            // Only committed runtimes escape through shared_runtime().
            std::rc::Rc::get_mut(runtime)
                .ok_or(runtime::MemoryRuntimeAccessError::ReentrantAccess)?
                .bootstrap(&declarations, &input.grants, &policy)?;
        }
        Ok(())
    })?;
    Ok(())
}

pub(crate) fn bootstrap_framework() -> Result<(), MemoryBootstrapError> {
    if runtime::is_memory_bootstrap_ready()? {
        return Ok(());
    }
    bootstrap(MemoryBootstrap {
        requests: requests().map_err(ic_storage::RuntimeBootstrapError::from)?,
        grants: grants::framework_grants(Vec::new())
            .map_err(ic_storage::RuntimeBootstrapError::from)?,
        admission: None,
    })
}
