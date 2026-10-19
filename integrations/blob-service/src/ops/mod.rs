//! Module: blob_service::ops
//!
//! Responsibility: borrow the host installation and supply platform observations.
//! Does not own: tenant policy, provider workflows or framework lifecycle.
//! Boundary: one service installation borrows grants from Canic’s committed runtime.

pub mod memory;
pub mod metrics;

use crate::ops::memory::Memory;

use std::cell::RefCell;

use ic_blob_storage::ops::service::installation::{
    ServiceInstallation, ValidatedServiceInstallation,
};
use ic_blob_storage_contracts::{
    configuration::ServiceInstallationCandidate, dto::configuration::ServiceInstallationInput,
};

/// Application owner of the shared service installation.
pub struct Host;
thread_local! {
    static HOST: RefCell<Option<ServiceInstallation<Memory>>> = const { RefCell::new(None) };
}
pub fn install(input: &ServiceInstallationInput) {
    // The complete candidate is checked before this host opens the service grants.
    let candidate = ValidatedServiceInstallation::new(
        ic_platform::cdk::api::canister_self(),
        ServiceInstallationCandidate {
            configuration: input.configuration,
            project: &input.project,
            completion_verifier: input.completion_verifier,
            release: ic_blob_storage::LIBRARY_VERSION,
            platform_installation_version: ic_platform::cdk::api::canister_version(),
        },
    )
    .expect("invalid installation configuration");
    assert_uninitialized();
    let installation =
        ServiceInstallation::install(memory::open(), candidate).expect("service installation");
    publish(installation);
}
pub fn restore() {
    assert_uninitialized();
    let installation = ServiceInstallation::open(
        memory::open(),
        ic_platform::cdk::api::canister_self(),
        ic_blob_storage::LIBRARY_VERSION,
    )
    .expect("service restoration");
    publish(installation);
}
fn assert_uninitialized() {
    HOST.with_borrow(|host| assert!(host.is_none(), "host initialization is not reset"));
}
fn publish(mut installation: ServiceInstallation<Memory>) {
    installation.observe_platform_version(ic_platform::cdk::api::canister_version());
    HOST.with_borrow_mut(|state| *state = Some(installation));
}
/// Explicit endpoint catalog binding; this alias registers nothing.
pub type Workflow = ic_blob_storage::workflow::service::ServiceWorkflow<Host>;
impl ic_blob_storage::workflow::service::ServiceHost for Host {
    type Memory = Memory;
    fn with_installation<R>(operation: impl FnOnce(&mut ServiceInstallation<Memory>) -> R) -> R {
        HOST.with_borrow_mut(|state| operation(state.as_mut().expect("initialized host")))
    }
    fn platform_version() -> u64 {
        ic_platform::cdk::api::canister_version()
    }
    fn time() -> u64 {
        ic_platform::cdk::api::time()
    }
    fn limits() -> ic_blob_storage::workflow::service::ServiceExecutionLimits {
        use ic_blob_storage::ops::caffeine::decoding::CaffeineReplyLimits;
        ic_blob_storage::workflow::service::ServiceExecutionLimits {
            timeout: 30.try_into().unwrap(),
            gateways: CaffeineReplyLimits {
                max_bytes: 65_536.try_into().unwrap(),
                decoding_quota: 500_000.try_into().unwrap(),
                skipping_quota: 1000.try_into().unwrap(),
                max_type_entries: 32.try_into().unwrap(),
            },
            account: CaffeineReplyLimits {
                max_bytes: 4096.try_into().unwrap(),
                decoding_quota: 500_000.try_into().unwrap(),
                skipping_quota: 1000.try_into().unwrap(),
                max_type_entries: 64.try_into().unwrap(),
            },
            history_scan: 64.try_into().unwrap(),
            history_results: 32.try_into().unwrap(),
            funding_results: 32.try_into().unwrap(),
        }
    }
}
