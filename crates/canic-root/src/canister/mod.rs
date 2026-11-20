#[cfg(feature = "backup")]
mod backup;
#[cfg(feature = "sharding")]
mod sharding;

use canic_management::{self as management, Readiness};
use canic_management_protocol::Failure;
use ic_platform_runtime as runtime;

canic::mount_metrics_endpoint!(canic_management::metrics);

/// Create the Root management journal and initialize selected enrollment state.
#[runtime::init(
    runtime = runtime,
    guard = lifecycle_guard,
    decode = lifecycle_decode,
    max_bytes = 0,
    on_error = endpoint_error,
)]
fn init((): ()) {
    management::open(true, readiness());
    #[cfg(feature = "auth")]
    crate::auth::fresh();
}

/// Reopen the root state from the preceding executable.
#[runtime::post_upgrade(
    runtime = runtime,
    guard = lifecycle_guard,
    decode = lifecycle_decode,
    max_bytes = 0,
    on_error = endpoint_error,
)]
fn post_upgrade((): ()) {
    management::open(false, readiness());
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

#[cfg(feature = "blob")]
management::mount_endpoints! {
    stage_application_upgrade, reconcile_application_upgrade, activate_application_upgrade,
}

/// Deliver one exact revision to a ready named service under the operator's approval.
#[canic::update(guard = directory_controller)]
async fn publish_directory(request: (String, u64)) -> Result<u64, String> {
    let (name, revision) = request;
    management::application::publish_directory(&name, revision)
        .await
        .map_err(String::from)
}

fn directory_controller() -> Result<(), &'static str> {
    runtime::system::is_controller(&runtime::system::caller())
        .then_some(())
        .ok_or("controller required")
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
