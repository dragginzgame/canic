//! Qualify explicit host namespace grants before and after bootstrap.
use canic::__internal::core::__reexports::ic_storage;

canic::memory::memory_allocation_grants!(
    provider = application_memory_grants,
    authorities = [("provider-test", "provider_test.")],
);

fn input() -> canic::memory::MemoryBootstrap {
    let mut input = canic::memory::framework();
    input.grants = application_memory_grants().unwrap();
    input
        .requests
        .push(ic_storage::MemoryRequest::new("provider-test", "provider_test.rows.v1").unwrap());
    input
}

#[test]
fn checked_provider_is_pure_and_warm_bootstrap_keeps_the_same_allocation() {
    let grants = application_memory_grants().unwrap();
    assert_eq!(application_memory_grants().unwrap(), grants);
    assert!(!canic::memory::runtime::is_memory_bootstrap_ready().unwrap());
    canic::memory::bootstrap(input()).unwrap();
    assert_eq!(
        canic::memory::open("provider_test.rows.v1")
            .grow(1)
            .unwrap(),
        0
    );
    let runtime = canic::memory::runtime::shared_runtime().unwrap();
    let key = ic_storage::StableKey::parse("provider_test.rows.v1").unwrap();
    let id = runtime.allocation_id(&key);
    canic::memory::bootstrap(input()).unwrap();
    assert_eq!(runtime.allocation_id(&key), id);
    assert_eq!(
        canic::memory::open("provider_test.rows.v1")
            .grow(1)
            .unwrap(),
        1
    );
    assert_eq!(application_memory_grants().unwrap(), grants);
}

#[test]
fn explicit_composition_rejects_duplicate_unauthorized_and_refused_admission() {
    use canic::memory::{admission::MemoryBootstrapAdmission, registry::MemoryRegistryError};
    let mut duplicate = input();
    duplicate.requests.push(duplicate.requests[0].clone());
    assert!(matches!(
        canic::memory::bootstrap(duplicate),
        Err(canic::memory::MemoryBootstrapError::Bootstrap(
            ic_storage::RuntimeBootstrapError::Admission(ic_storage::AdmissionError::Duplicate)
        ))
    ));
    assert!(!canic::memory::runtime::is_memory_bootstrap_ready().unwrap());
    let mut unauthorized = input();
    unauthorized.requests.push(
        ic_storage::MemoryRequest::new("provider-test", "canic.core.unauthorized.v1").unwrap(),
    );
    assert!(matches!(
        canic::memory::bootstrap(unauthorized),
        Err(canic::memory::MemoryBootstrapError::Bootstrap(
            ic_storage::RuntimeBootstrapError::Admission(ic_storage::AdmissionError::Unauthorized)
        ))
    ));
    assert!(!canic::memory::runtime::is_memory_bootstrap_ready().unwrap());
    let mut rejected = input();
    rejected.admission = Some(MemoryBootstrapAdmission::new([3; 32], |_| {
        Err(MemoryRegistryError::InvalidDeclaration {
            stable_key: "test.refused.v1".into(),
            reason: "host admission refusal",
        })
    }));
    assert!(matches!(
        canic::memory::bootstrap(rejected),
        Err(canic::memory::MemoryBootstrapError::Bootstrap(
            ic_storage::RuntimeBootstrapError::Policy(
                MemoryRegistryError::InvalidDeclaration { .. }
            )
        ))
    ));
    assert!(!canic::memory::runtime::is_memory_bootstrap_ready().unwrap());
    canic::memory::bootstrap(input()).unwrap();
    assert!(canic::memory::runtime::is_memory_bootstrap_ready().unwrap());
}

#[test]
fn application_grants_cannot_overlap_framework_namespaces() {
    assert!(matches!(
        canic::memory::grants::framework_grants(vec![
            ic_storage::MemoryAuthority::new("application", "canic.core.").unwrap()
        ]),
        Err(ic_storage::AdmissionError::Duplicate)
    ));
}
