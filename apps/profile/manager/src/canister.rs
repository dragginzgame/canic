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
    status, observe, observe_custody, stage, reconcile,
    stage_upgrade, upgrade_status, observe_upgrade, reconcile_upgrade,
}

#[cfg(feature = "fault-injection")]
management::mount_endpoints! { inject_fault }

fn readiness() -> Readiness {
    Readiness {
        method: "probe",
        request: ic_wire::native::encode_request(
            canic_profile_protocol::INFO,
            &(),
            canic_management_protocol::LIMITS,
        )
        .expect("probe args"),
        accept: |reply| {
            let info: canic_profile_protocol::Info = ic_wire::native::decode_response(
                canic_profile_protocol::INFO,
                reply,
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
