use canic_management::{self as management, Readiness};
use canic_management_protocol::Failure;
use ic_platform_runtime as runtime;

/// Initialize the profile manager state for this installation.
#[runtime::init(
    runtime = runtime,
    guard = lifecycle_guard,
    decode = lifecycle_decode,
    max_bytes = 0,
    on_error = endpoint_error,
)]
fn init((): ()) {
    management::open(true, readiness());
}

/// Reopen the profile manager state from the preceding executable.
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
    publish_store_artifact, observe_store, store_admission,
    stage_store_maintenance, advance_store_maintenance, observe_store_maintenance,
    install_store_maintenance, activate_store_maintenance, store_maintenance_status, recover_store_maintenance,
    historical_root_custody, observe_root_history, observe_stored_children,
    stage_stored_child, reconcile_stored_child, stage_stored_upgrade, reconcile_stored_upgrade,
    child_status, child_upgrade_status, stage_inline_child_creation, reconcile_inline_child_creation, observe_child,
    stage_inline_child_upgrade, reconcile_inline_child_upgrade, observe_child_upgrade,
    status, observe, observe_custody, stage, reconcile,
    stage_upgrade, upgrade_status, observe_upgrade, reconcile_upgrade,
}

#[cfg(feature = "fault-injection")]
management::mount_endpoints! { inject_fault }

fn readiness() -> Readiness {
    Readiness {
        method: "probe",
        request: ic_wire::encode(
            &(),
            ic_wire::Encoding::Frame {
                method: canic_profile_protocol::INFO,
                direction: ic_wire::Direction::Request,
            },
            canic_management_protocol::LIMITS,
        )
        .expect("probe args"),
        accept: |reply| {
            let info: canic_profile_protocol::Info = ic_wire::decode(
                reply,
                ic_wire::Encoding::Frame {
                    method: canic_profile_protocol::INFO,
                    direction: ic_wire::Direction::Response,
                },
                canic_management_protocol::LIMITS,
            )
            .map_err(|_| Failure::InvalidReply)?;
            if info.ready {
                Ok(())
            } else {
                Err(Failure::Mismatch)
            }
        },
    }
}

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
