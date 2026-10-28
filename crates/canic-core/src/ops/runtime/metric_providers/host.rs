//! Explicit host lifecycle over the single committed Canic Memory runtime.
use super::*;
use ic_memory::{DefaultMemoryImpl, MemoryRequest, RuntimeMemory};
use ic_storage::adapter::StableBTreeMap;
use std::cell::RefCell;

thread_local! {
    static PROVIDERS: RefCell<ProviderMetrics<DefaultMemoryImpl>> = RefCell::default();
}
/// Register each component's static definitions in its synchronous lifecycle participant.
pub fn register(
    provider: &'static str,
    definitions: &'static [MetricDefinition],
) -> Result<(), ProviderMetricsError> {
    PROVIDERS.with_borrow_mut(|state| state.register(provider, definitions))
}
/// Called once after all synchronous component participants and before deferred work.
/// The immutable compiled configuration is the sole storage-mode authority.
pub fn activate(fresh: bool) -> Result<(), ProviderMetricsError> {
    let authority =
        crate::config::RoleRuntimeConfig::try_get().ok_or(ProviderMetricsError::Lifecycle)?;
    let role = crate::ops::runtime::env::EnvOps::canister_role()
        .map_err(|_| ProviderMetricsError::Lifecycle)?;
    let component = crate::ops::runtime::env::EnvOps::component_spec().ok();
    let selected = authority
        .canister(component.as_ref(), &role)
        .ok_or(ProviderMetricsError::Lifecycle)?;
    PROVIDERS.with_borrow_mut(|state| {
        state.activate(
            &selected.metrics,
            fresh,
            crate::ops::ic::IcOps::canister_version(),
            || {
                let memory: RuntimeMemory<DefaultMemoryImpl> = crate::memory::open(MEMORY_KEY);
                if (memory.size() == 0) != fresh {
                    return Err(ProviderMetricsError::Backing);
                }
                let map = if fresh {
                    StableBTreeMap::new(memory)
                } else {
                    StableBTreeMap::load(memory)
                };
                Ok(MetricsStore::new(map))
            },
        )
    })
}
/// Record one declared diagnostic scalar, with identical heap/stable call syntax.
pub fn add(provider: &str, name: &str, amount: u128) -> Result<(), ProviderMetricsError> {
    PROVIDERS.with_borrow_mut(|state| state.add(provider, name, amount))
}
/// Read a provider's admitted counters without sampling, remote calls or writes.
pub fn snapshot(provider: &str) -> Result<Vec<MetricObservation>, ProviderMetricsError> {
    let mut rows = PROVIDERS.with_borrow(|state| state.snapshot(provider))?;
    let current = crate::ops::ic::IcOps::canister_version();
    for row in &mut rows {
        row.window = current;
    }
    Ok(rows)
}

/// Union only the current role's compiled variants before bootstrap. A heap-only
/// artifact declares no Metrics allocation; unrelated child-role policy is ignored.
pub fn role_memory_requests(
    authority: &crate::config::RoleRuntimeAuthority,
) -> Result<Vec<MemoryRequest>, ic_memory::MemoryDeclarationError> {
    if authority
        .canisters
        .iter()
        .filter(|entry| entry.role == authority.role)
        .any(|entry| entry.config.metrics.needs_stable())
    {
        Ok(vec![MemoryRequest::new(
            crate::memory::CANIC_CORE_MEMORY_AUTHORITY,
            MEMORY_KEY,
        )?])
    } else {
        Ok(Vec::new())
    }
}

/// Record one observation using the same declared provider on either backend.
pub fn record(provider: &str, name: &str, value: u64) -> Result<(), ProviderMetricsError> {
    PROVIDERS.with_borrow_mut(|state| state.record(provider, name, value))
}
