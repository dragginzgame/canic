use canic_management::{self as management, Readiness};
use canic_management_protocol::Failure;
use ic_platform_runtime::cdk;

#[cdk::init]
fn init() {
    management::open(true, readiness());
}
#[cdk::post_upgrade]
fn post_upgrade() {
    management::open(false, readiness());
}
management::mount_endpoints! {
    store_publication, observe_store, observe_store_custody, store_admission,
    stage_store_maintenance, advance_store_maintenance, observe_store_maintenance,
    install_store_maintenance, activate_store_maintenance, store_maintenance_status, recover_store_maintenance,
    status, observe, observe_custody, observe_root_custody, observe_root_history, observe_store_attempt, stage, reconcile,
    stage_upgrade, upgrade_status, observe_upgrade, reconcile_upgrade,
    observe_stored_children, stage_stored_child, reconcile_stored_child,
    stage_stored_upgrade, reconcile_stored_upgrade,
    observe_children, child_status, observe_child, stage_child, reconcile_child,
    stage_child_upgrade, observe_child_upgrade, child_upgrade_status, reconcile_child_upgrade,
}

fn readiness() -> Readiness {
    Readiness {
        method: "probe",
        request: ic_wire::encode(
            &"Root",
            ic_wire::Encoding::Frame {
                method: canic::GREET,
                direction: ic_wire::Direction::Request,
            },
            canic::LIMITS,
        )
        .expect("probe args"),
        accept: |reply| {
            let greeting: canic::Greeting = ic_wire::decode(
                reply,
                ic_wire::Encoding::Frame {
                    method: canic::GREET,
                    direction: ic_wire::Direction::Response,
                },
                canic::LIMITS,
            )
            .map_err(|_| Failure::InvalidReply)?;
            if greeting.message.is_empty() {
                Err(Failure::Mismatch)
            } else {
                Ok(())
            }
        },
    }
}

#[cfg(feature = "fault-injection")]
management::mount_endpoints! { inject_child_fault }
