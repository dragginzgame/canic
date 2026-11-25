#[cfg(feature = "backup")]
mod backup;
#[cfg(feature = "child-management")]
mod sharding;

use canic_management::{self as management, Readiness};
use canic_management_protocol::Failure;
use ic_platform_runtime as runtime;

// init
// Create the Root journal with the approved fleet ceiling and initialize enrollment.
// guard  : lifecycle_guard (IC lifecycle)
// limits : up to 64 install bytes and no reply
#[runtime::init(
    runtime = runtime,
    guard = lifecycle_guard,
    decode = bootstrap_decode,
    max_bytes = 64,
    on_error = endpoint_error,
)]
fn init(policy: Option<canic_management_protocol::stored::RootBootstrap>) {
    management::open(true, readiness());
    if let Some(policy) = policy {
        management::bootstrap(policy);
    }
    canic::logging::record(canic::logging::Level::Info, "root", "installed");
    #[cfg(feature = "auth")]
    crate::auth::fresh();
}

// post_upgrade
// Reopen the root state from the preceding executable.
// guard  : lifecycle_guard (IC lifecycle)
// limits : empty request and no reply
#[runtime::post_upgrade(
    runtime = runtime,
    guard = lifecycle_guard,
    decode = lifecycle_decode,
    max_bytes = 0,
    on_error = endpoint_error,
)]
fn post_upgrade((): ()) {
    management::open(false, readiness());
    #[cfg(feature = "child-management")]
    for (family, retained) in management::sharding::retained_mounts() {
        const MOUNTS: &[&str] = &[
            #[cfg(feature = "child-creation")]
            "child-creation",
            #[cfg(feature = "reserve-pool")]
            "reserve-pool",
            #[cfg(feature = "child-funding")]
            "child-funding",
        ];
        if retained && !MOUNTS.contains(&family) {
            runtime::system::trap(&format!("retained Root approval requires {family}"));
        }
    }
    canic::logging::record(
        canic::logging::Level::Info,
        "root",
        "reopened after upgrade",
    );
}

management::mount_endpoints! {
    publish_store_artifact, observe_store, store_admission, store_removal,
    stage_store_maintenance, advance_store_maintenance, observe_store_maintenance,
    install_store_maintenance, activate_store_maintenance, store_maintenance_status, recover_store_maintenance,
    observe_root_custody,
    stage_child_creation, reconcile_child_creation,
    stage_child_upgrade, reconcile_child_upgrade,
    child_status, observe_child, stage_inline_child_creation, reconcile_inline_child_creation,
    stage_inline_child_upgrade, observe_child_upgrade, child_upgrade_status, reconcile_inline_child_upgrade,
}

#[cfg(feature = "child-management")]
management::mount_endpoints! { observe_child_deployments, }

#[cfg(feature = "blob")]
management::mount_endpoints! {
    activate_application_upgrade,
}

// logs
// Read one page of Root diagnostics using the operator’s signed call.
// guard  : Root controller
// limits : default request, Logs::REPLY (4,096 bytes)
#[canic::update(contract = canic::logging::Logs, guard = canic::guard::require_controller)]
fn logs(before: Option<u64>) -> canic::logging::Page {
    canic::logging::page(before)
}

// redeliver_directory_revision
// Deliver one exact revision to a ready named service under the operator's approval.
// guard  : Root controller
// limits : default
#[canic::update(contract = canic::topology::directory::RedeliverDirectoryRevision, guard = canic::guard::require_controller)]
async fn redeliver_directory_revision(
    request: (String, u64),
) -> Result<u64, canic::topology::directory::RedeliveryError> {
    let (name, revision) = request;
    management::application::redeliver_directory_revision(&name, revision).await
}

// child_metrics
// Read bounded performance observations from an installed, ready child of this Root.
// guard  : Root controller
// limits : ChildMetrics::REQUEST and ChildMetrics::REPLY (4,096-byte reply)
#[cfg(feature = "child-metrics")]
#[canic::update(contract = canic::metrics::ChildMetrics, guard = canic::guard::require_controller)]
async fn child_metrics(request: (u8, canic::metrics::Request)) -> canic::metrics::Report {
    use canic::metrics::ReadMetrics;
    use canic_management_protocol::{Phase, children::ChildId};
    let (slot, source) = request;
    let slot = ChildId::new(usize::from(slot))
        .unwrap_or_else(|| runtime::system::trap("unknown child slot"));
    let status = management::children::status(slot);
    if status.phase != Phase::Ready {
        runtime::system::trap("child is not ready");
    }
    let target = status
        .worker
        .expect("ready child")
        .parse()
        .expect("retained child principal");
    let caller = runtime::system::caller();
    let report = canic::method::call::<ReadMetrics>(target, 5, &source)
        .await
        .unwrap_or_else(|error| canic::method::fail(error));
    assert!(
        runtime::system::is_controller(&caller),
        "controller required"
    );
    report
}

fn readiness() -> Readiness {
    Readiness {
        method: "readiness",
        request: ic_wire::encode(
            &(),
            ic_wire::Encoding::Frame {
                method: canic::readiness::METHOD,
                direction: ic_wire::Direction::Request,
            },
            canic::readiness::REQUEST_LIMITS,
        )
        .expect("readiness args"),
        accept: |reply| {
            let state: canic::readiness::State = ic_wire::decode(
                reply,
                ic_wire::Encoding::Frame {
                    method: canic::readiness::METHOD,
                    direction: ic_wire::Direction::Response,
                },
                canic::readiness::RESPONSE_LIMITS,
            )
            .map_err(|_| Failure::InvalidReply)?;
            match state {
                canic::readiness::State::Ready => Ok(()),
                canic::readiness::State::Unavailable | canic::readiness::State::Fenced => {
                    Err(Failure::Mismatch)
                }
            }
        },
    }
}

#[cfg(feature = "fault-injection")]
management::mount_endpoints! { inject_child_fault }

fn endpoint_error(error: impl std::fmt::Debug) -> ! {
    ic_platform_runtime::system::trap(&format!("endpoint refused: {error:?}"))
}

/// The IC admits lifecycle calls. The declared argument budget still applies.
fn lifecycle_guard() -> Result<(), std::convert::Infallible> {
    Ok(())
}

fn bootstrap_decode(
    bytes: &[u8],
) -> Result<Option<canic_management_protocol::stored::RootBootstrap>, ic_wire::Error> {
    if bytes.is_empty() {
        return Ok(None);
    }
    ic_wire::decode(
        bytes,
        ic_wire::Encoding::Payload,
        canic_management_protocol::LIMITS,
    )
    .map(Some)
}

fn lifecycle_decode(_bytes: &[u8]) -> Result<(), std::convert::Infallible> {
    Ok(())
}
