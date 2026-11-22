#[cfg(feature = "backup")]
mod backup;
#[cfg(feature = "sharding")]
mod sharding;

use canic_management::{self as management, Readiness};
use canic_management_protocol::Failure;
use ic_platform_runtime as runtime;

// init
// Create the Root management journal and initialize selected enrollment state.
// guard  : lifecycle_guard (IC lifecycle)
// limits : empty request and no reply
#[runtime::init(
    runtime = runtime,
    guard = lifecycle_guard,
    decode = lifecycle_decode,
    max_bytes = 0,
    on_error = endpoint_error,
)]
fn init((): ()) {
    management::open(true, readiness());
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
    canic::logging::record(
        canic::logging::Level::Info,
        "root",
        "reopened after upgrade",
    );
}

management::mount_endpoints! {
    store_publication, observe_store, observe_store_custody, store_admission, store_removal,
    stage_store_maintenance, advance_store_maintenance, observe_store_maintenance,
    install_store_maintenance, activate_store_maintenance, store_maintenance_status, recover_store_maintenance,
    status, observe, observe_custody, observe_root_custody, observe_root_history, observe_store_attempt, stage, reconcile,
    stage_upgrade, upgrade_status, observe_upgrade, reconcile_upgrade,
    observe_stored_children, stage_stored_child, reconcile_stored_child,
    observe_application_children, stage_application_child, reconcile_application_child,
    stage_stored_upgrade, reconcile_stored_upgrade,
    observe_children, child_status, observe_child, stage_child, reconcile_child,
    stage_child_upgrade, observe_child_upgrade, child_upgrade_status, reconcile_child_upgrade,
}

#[cfg(feature = "role-issuer")]
management::mount_endpoints! { observe_role_children, }

#[cfg(feature = "blob")]
management::mount_endpoints! {
    stage_application_upgrade, reconcile_application_upgrade, activate_application_upgrade,
}

// logs
// Read one page of Root diagnostics using the operator’s signed call.
// guard  : Root controller
// limits : default request, Logs::REPLY (4,096 bytes)
#[canic::update(contract = canic::logging::Logs, guard = canic::guard::require_controller)]
fn logs(before: Option<u64>) -> canic::logging::Page {
    canic::logging::page(before)
}

// publish_directory
// Deliver one exact revision to a ready named service under the operator's approval.
// guard  : Root controller
// limits : default
#[canic::update(guard = canic::guard::require_controller)]
async fn publish_directory(request: (String, u64)) -> Result<u64, String> {
    let (name, revision) = request;
    management::application::publish_directory(&name, revision)
        .await
        .map_err(String::from)
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

fn lifecycle_decode(_bytes: &[u8]) -> Result<(), std::convert::Infallible> {
    Ok(())
}
