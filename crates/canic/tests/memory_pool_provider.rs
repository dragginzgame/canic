//! Qualify one authored pool as both an artifact registration and a pure provider.

use canic::__internal::core::__reexports::ic_memory;

canic::memory::memory_allocation_pool!(
    provider = application_memory_pool,
    authorities = [("provider-test", "provider_test.")],
    exclusions = [240, 241, 242, 243, 244],
);

fn grow_application_memory() -> u64 {
    canic::memory::open("provider_test.rows.v1")
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
    let mut input = canic::memory::framework();
    input.pool = application_memory_pool().unwrap();
    input
        .requests
        .push(ic_memory::MemoryRequest::new("provider-test", "provider_test.rows.v1").unwrap());
    canic::memory::bootstrap(input).unwrap();
    let shared = canic::memory::runtime::shared_runtime().unwrap();
    assert_eq!(grow_application_memory(), 0);
    assert_eq!(application_memory_pool().unwrap(), before);
    let mut input = canic::memory::framework();
    input.pool = application_memory_pool().unwrap();
    input
        .requests
        .push(ic_memory::MemoryRequest::new("provider-test", "provider_test.rows.v1").unwrap());
    canic::memory::bootstrap(input).unwrap();
    assert_eq!(grow_application_memory(), 1);
    let repeated = canic::memory::runtime::shared_runtime().unwrap();
    assert!(std::rc::Rc::ptr_eq(&shared, &repeated));
    let lease = shared.open_memory("provider_test.rows.v1").unwrap();
    assert!(matches!(
        repeated.open_memory("provider_test.rows.v1"),
        Err(ic_memory::RuntimeOpenError::AlreadyOpen { .. })
    ));
    drop(lease);
    assert_eq!(application_memory_pool().unwrap(), before);
}

#[test]
fn explicit_composition_refuses_duplicates_and_admission_before_publication() {
    std::thread::spawn(|| {
        use canic::memory::{
            admission::{MemoryBootstrapAdmission, PolicyIdentity},
            registry::MemoryRegistryError,
        };
        let mut duplicated = canic::memory::framework();
        duplicated.requests.push(duplicated.requests[0].clone());
        assert!(matches!(
            canic::memory::bootstrap(duplicated),
            Err(canic::memory::MemoryBootstrapError::Bootstrap(
                ic_memory::RuntimeBootstrapError::Resolution(
                    ic_memory::MemoryResolutionError::Declarations(
                        ic_memory::MemoryDeclarationError::DuplicateRequest { .. }
                    )
                )
            ))
        ));
        assert!(!canic::memory::runtime::is_memory_bootstrap_ready().unwrap());

        let mut rejected = canic::memory::framework();
        rejected.admission = Some(MemoryBootstrapAdmission::new(
            PolicyIdentity::new([3; 32]),
            |_| {
                Err(MemoryRegistryError::InvalidDeclaration {
                    stable_key: "test.refused.v1".to_string(),
                    reason: "host admission refusal",
                })
            },
        ));
        assert!(matches!(
            canic::memory::bootstrap(rejected),
            Err(canic::memory::MemoryBootstrapError::Bootstrap(
                ic_memory::RuntimeBootstrapError::AdmissionPolicy(
                    MemoryRegistryError::InvalidDeclaration { .. }
                )
            ))
        ));
        assert!(!canic::memory::runtime::is_memory_bootstrap_ready().unwrap());
        canic::memory::bootstrap(canic::memory::framework()).unwrap();
        assert!(canic::memory::runtime::is_memory_bootstrap_ready().unwrap());
    })
    .join()
    .unwrap();
}
