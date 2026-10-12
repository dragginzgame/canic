//! Qualify one authored pool as both an artifact registration and a pure provider.

use canic::api::runtime::MemoryRuntimeApi;

canic::memory::memory_allocation_pool!(
    provider = application_memory_pool,
    authorities = [("provider-test", "provider_test.")],
    exclusions = [(240, 244)],
);

fn grow_application_memory() -> u64 {
    canic::memory::ic_memory_key!(authority = "provider-test", key = "provider_test.rows.v1",)
        .grow(1)
        .unwrap()
}

#[test]
fn checked_provider_is_available_before_bootstrap_and_after_warm_adoption() {
    let before = application_memory_pool().unwrap();
    assert!(before.authorities().iter().any(|grant| {
        grant.authority() == "provider-test" && grant.key_prefix() == "provider_test."
    }));
    assert!(before.authorities().iter().any(|grant| {
        grant.authority() == canic::memory::CANIC_CORE_MEMORY_AUTHORITY
            && grant.key_prefix() == "canic.core."
    }));
    assert!(!before.contains(0));
    assert!(!before.contains(242));
    assert!(before.contains(239));
    assert!(before.contains(245));

    // Provider reads neither a sealed host registry nor the committed runtime.
    assert_eq!(application_memory_pool().unwrap(), before);
    MemoryRuntimeApi::bootstrap_registry().unwrap();
    assert_eq!(grow_application_memory(), 0);
    assert_eq!(application_memory_pool().unwrap(), before);
    MemoryRuntimeApi::bootstrap_registry().unwrap();
    assert_eq!(grow_application_memory(), 1);
    assert_eq!(application_memory_pool().unwrap(), before);
}
