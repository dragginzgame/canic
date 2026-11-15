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
    observe_root_custody, observe_stored_children,
    stage_stored_child, reconcile_stored_child, stage_stored_upgrade, reconcile_stored_upgrade,
    child_status, child_upgrade_status,
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
