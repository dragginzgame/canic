//! Module: memory
//!
//! Responsibility: adapt Canic stable-memory declarations to `ic-memory` bootstrap.
//! Also owns physical stable-memory extent observation for runtime metrics.
//! Does not own: stable data schemas, ops storage APIs, or lifecycle orchestration.
//! Boundary: lifecycle initializes this before stable structures are accessed.

pub mod admission;
pub(crate) mod keys;
pub(crate) mod ledger;
mod policy;
pub mod pool;
pub mod registry;
pub mod runtime;

pub use crate::memory_allocation_pool;
pub use policy::CanicMemoryManagerPolicy;

/// Stable allocation-policy authority for Canic core memory declarations.
pub const CANIC_CORE_MEMORY_AUTHORITY: &str = "canic-core";
/// Stable allocation-policy authority for Canic control-plane memory declarations.
pub const CANIC_CONTROL_PLANE_MEMORY_AUTHORITY: &str = "canic-control-plane";

const MEMORY_AUTHORITY: &str = CANIC_CORE_MEMORY_AUTHORITY;

/// Bucket size compiled into this artifact, in 64 KiB Wasm pages.
///
/// # Panics
/// Panics if the build script supplies an invalid value; normal builds validate it first.
#[must_use]
pub fn configured_bucket_pages() -> u16 {
    env!("CANIC_MEMORY_BUCKET_PAGES")
        .parse()
        .expect("build-validated memory bucket size")
}

/// Observe the current physical stable-memory extent without allocating or reading data.
#[cfg(target_arch = "wasm32")]
pub(crate) fn stable_extent_bytes() -> u64 {
    ic_cdk::api::stable_size().saturating_mul(65_536)
}

/// Complete framework requests, including stores whose lazy initializers have not run.
pub fn requests() -> Result<Vec<ic_memory::MemoryRequest>, ic_memory::MemoryDeclarationError> {
    keys::requests()
}

/// Open a key already committed by the host's explicit bootstrap.
///
/// # Panics
/// Panics if the host has not bootstrapped or omitted this key.
#[must_use]
pub fn open(
    key: &str,
) -> ic_memory::RuntimeMemory<ic_memory::ic_stable_structures::DefaultMemoryImpl> {
    runtime::assert_memory_bootstrap_ready(key);
    runtime::with_runtime(|runtime| Ok(runtime.open_memory(key)))
        .expect("initialized memory runtime")
        .expect("committed stable key")
}

/// Host-owned component requests, grants and optional pre-commit admission.
pub struct MemoryBootstrap {
    /// Complete component requests, including the selected framework.
    pub requests: Vec<ic_memory::MemoryRequest>,
    /// Exact namespace grants and unmanaged physical exclusions.
    pub pool: ic_memory::MemoryAllocationPool,
    /// One composed pre-commit callback with a stable semantic identity.
    pub admission: Option<admission::MemoryBootstrapAdmission>,
}

/// Failure to construct, borrow or bootstrap the Canic-owned memory runtime.
#[derive(Debug, thiserror::Error)]
pub enum MemoryBootstrapError {
    /// Backing memory or configured geometry could not be admitted.
    #[error(transparent)]
    Construction(#[from] ic_memory::RuntimeConstructionError),
    /// The runtime is already borrowed by a synchronous participant.
    #[error(transparent)]
    Access(#[from] ic_memory::RuntimeDiagnosticError),
    /// A component already retains the committed runtime; bootstrap must precede handoff.
    #[error("memory bootstrap cannot run after sharing the host runtime")]
    SharedRuntime,
    /// The complete host composition could not be committed.
    #[error(transparent)]
    Bootstrap(#[from] ic_memory::RuntimeBootstrapError<registry::MemoryRegistryError>),
}

/// Bootstrap exactly the supplied host composition before any store opens.
///
/// Finish bootstrap before sharing the runtime with component participants.
pub fn bootstrap(input: MemoryBootstrap) -> Result<(), MemoryBootstrapError> {
    let declarations = ic_memory::SealedDeclarationSnapshot::new(input.requests)
        .map_err(ic_memory::MemoryResolutionError::from)
        .map_err(ic_memory::RuntimeBootstrapError::from)?;
    let config = ic_memory::MemoryManagerConfig::new(configured_bucket_pages())?;
    let mut policy = policy::CanicMemoryManagerPolicy::new();
    if let Some(participant) = input.admission {
        policy = policy.with_admission(participant);
    }
    runtime::RUNTIME.with(|cell| -> Result<(), MemoryBootstrapError> {
        let mut owner = cell
            .try_borrow_mut()
            .map_err(|_| ic_memory::RuntimeDiagnosticError::ReentrantAccess)?;
        if owner.is_none() {
            *owner = Some(std::rc::Rc::new(ic_memory::MemoryRuntime::new_with_config(
                ic_memory::ic_stable_structures::DefaultMemoryImpl::default(),
                config,
            )?));
        }
        let runtime = owner
            .as_mut()
            .ok_or(ic_memory::RuntimeDiagnosticError::NotBootstrapped)?;
        let runtime = std::rc::Rc::get_mut(runtime).ok_or(MemoryBootstrapError::SharedRuntime)?;
        runtime.bootstrap(&declarations, &input.pool, &policy)?;
        Ok(())
    })?;
    pool::remember(input.pool);
    Ok(())
}

pub(crate) fn bootstrap_framework() -> Result<(), MemoryBootstrapError> {
    if runtime::is_memory_bootstrap_ready()? {
        return Ok(());
    }
    bootstrap(MemoryBootstrap {
        requests: requests()
            .map_err(ic_memory::MemoryResolutionError::from)
            .map_err(ic_memory::RuntimeBootstrapError::from)?,
        pool: pool::framework_pool(Vec::new(), Vec::new())
            .map_err(registry::MemoryRegistryError::from)
            .map_err(ic_memory::RuntimeBootstrapError::AdmissionPolicy)?,
        admission: None,
    })
}
