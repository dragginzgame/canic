//! Module: blob_service::ops
//!
//! Responsibility: own service borrowing and observe platform-version continuity.
//! Does not own: tenant policy, provider workflows or framework lifecycle.
//! Boundary: one service installation borrows grants from Canic’s committed runtime.

pub(crate) mod account;
pub(crate) mod gateways;
pub mod memory;
pub(crate) mod metrics;

use crate::ops::memory::{Grants, Memory};

use std::cell::RefCell;

use ic_blob_storage::ops::service::{
    installation::{
        ServiceInstallation, ServiceInstallationMemories, ValidatedServiceInstallation,
    },
    stores::ServiceStores,
};
use ic_blob_storage_contracts::{
    configuration::ServiceInstallationCandidate, download::scope::CaffeineDownloadScope,
    dto::configuration::ServiceInstallationInput, upload::completion::CompletionAuthority,
};

/// Application owner of the shared service and its observed platform version.
struct Host {
    installation: ServiceInstallation<Memory>,
    last_platform_version: u64,
}
thread_local! {
    static HOST: RefCell<Option<Host>> = const { RefCell::new(None) };
}
pub(crate) fn install(input: &ServiceInstallationInput) {
    // The complete candidate is checked before this host opens the service grants.
    let candidate = ValidatedServiceInstallation::new(
        ic_cdk::api::canister_self(),
        ServiceInstallationCandidate {
            configuration: input.configuration,
            project: &input.project,
            completion_verifier: input.completion_verifier,
            release: ic_blob_storage::LIBRARY_VERSION,
            platform_installation_version: ic_cdk::api::canister_version(),
        },
    )
    .expect("invalid installation configuration");
    assert_uninitialized();
    let Grants {
        configuration,
        stores,
    } = memory::open();
    let installation = ServiceInstallation::install(
        ServiceInstallationMemories {
            configuration,
            stores,
        },
        candidate,
    )
    .expect("service installation");
    publish(Host {
        installation,
        last_platform_version: ic_cdk::api::canister_version(),
    });
}
pub(crate) fn restore() {
    assert_uninitialized();
    let Grants {
        configuration,
        stores,
    } = memory::open();
    let installation = ServiceInstallation::open(
        ServiceInstallationMemories {
            configuration,
            stores,
        },
        ic_cdk::api::canister_self(),
        ic_blob_storage::LIBRARY_VERSION,
    )
    .expect("service restoration");
    publish(Host {
        installation,
        last_platform_version: ic_cdk::api::canister_version(),
    });
}
fn assert_uninitialized() {
    HOST.with_borrow(|host| assert!(host.is_none(), "host initialization is not reset"));
}
fn publish(host: Host) {
    HOST.with_borrow_mut(|state| *state = Some(host));
}
// Management changes can restore an old heap without a lifecycle hook. A gap
// enters a one-way fence; only fresh IC history can later prove continuity.
pub(crate) fn observe_version() {
    let version = ic_cdk::api::canister_version();
    HOST.with_borrow_mut(|state| {
        let host = state.as_mut().expect("initialized host");
        if version < host.last_platform_version
            || version.saturating_sub(host.last_platform_version) > 1
        {
            host.installation.fence();
        }
        host.last_platform_version = version;
    });
}
// Replicated queries discard their heap checkpoint; failed asynchronous replies
// can also leave a gap. An active owner gets one independent continuity check
// before a later mutation. Already-fenced restoration never activates here.
/// Anchor for proving a running instance after an observed message-version gap.
pub(crate) struct ActiveContinuityRequest {
    pub(crate) installation_version: u64,
}
pub(crate) fn begin_update() -> Option<ActiveContinuityRequest> {
    let version = ic_cdk::api::canister_version();
    let (needs_check, was_active, anchor) = HOST.with_borrow(|state| {
        let host = state.as_ref().expect("initialized host");
        (
            version < host.last_platform_version
                || version.saturating_sub(host.last_platform_version) > 1,
            !host.installation.stores().uploads.is_fenced(),
            host.installation.platform_installation_version(),
        )
    });
    observe_version();
    (needs_check && was_active).then_some(ActiveContinuityRequest {
        installation_version: anchor,
    })
}
pub(crate) fn resume_continuous_active(
    proof: ic_blob_storage::ops::service::recovery::CurrentInstanceProof,
) -> Result<(), ic_blob_storage::ops::service::recovery::CurrentInstanceRecoveryError> {
    HOST.with_borrow_mut(|state| {
        state
            .as_mut()
            .expect("initialized host")
            .installation
            .resume_current_instance(proof)
    })
}
pub(crate) fn read<R>(f: impl FnOnce(&ServiceStores<Memory>) -> R) -> R {
    observe_version();
    HOST.with_borrow(|host| {
        f(host
            .as_ref()
            .expect("initialized host")
            .installation
            .stores())
    })
}
pub(crate) fn mutate<R>(f: impl FnOnce(&mut ServiceStores<Memory>) -> R) -> R {
    observe_version();
    HOST.with_borrow_mut(|host| {
        f(host
            .as_mut()
            .expect("initialized host")
            .installation
            .stores_mut())
    })
}
pub(crate) fn with_completion<R>(
    f: impl FnOnce(&mut ServiceStores<Memory>, CompletionAuthority) -> R,
) -> R {
    observe_version();
    HOST.with_borrow_mut(|state| {
        let installation = &mut state.as_mut().expect("initialized host").installation;
        let authority = installation.completion_authority();
        f(installation.stores_mut(), authority)
    })
}
pub(crate) fn with_download<R>(
    f: impl FnOnce(
        &ic_blob_storage::ops::service::uploads::StableUploads<Memory>,
        &CaffeineDownloadScope,
    ) -> R,
) -> R {
    observe_version();
    HOST.with_borrow(|host| {
        let installation = &host.as_ref().expect("initialized host").installation;
        f(
            &installation.stores().uploads,
            installation.download_scope(),
        )
    })
}
pub(crate) fn with_verification<R>(
    f: impl FnOnce(
        &ic_blob_storage::ops::service::uploads::StableUploads<Memory>,
        CompletionAuthority,
        &CaffeineDownloadScope,
    ) -> R,
) -> R {
    observe_version();
    HOST.with_borrow(|host| {
        let installation = &host.as_ref().expect("initialized host").installation;
        f(
            &installation.stores().uploads,
            installation.completion_authority(),
            installation.download_scope(),
        )
    })
}
pub(crate) fn with_installation<R>(f: impl FnOnce(&ServiceInstallation<Memory>) -> R) -> R {
    observe_version();
    HOST.with_borrow(|host| {
        let installation = &host.as_ref().expect("initialized host").installation;
        f(installation)
    })
}
pub(crate) fn with_certificate<R>(f: impl FnOnce(&mut ServiceInstallation<Memory>) -> R) -> R {
    observe_version();
    HOST.with_borrow_mut(|host| f(&mut host.as_mut().expect("initialized host").installation))
}
/// Mutable installation access used by the shared current-instance workflow.
pub(crate) struct RecoveryHost;
impl ic_blob_storage::ops::service::recovery::RecoveryInstallationAccess for RecoveryHost {
    type Memory = Memory;
    fn with_installation<R>(
        &self,
        operation: impl FnOnce(&mut ServiceInstallation<Memory>) -> R,
    ) -> R {
        with_certificate(operation)
    }
}
