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
    status, observe, stage, reconcile,
    stage_upgrade, upgrade_status, observe_upgrade, reconcile_upgrade,
}

fn readiness() -> Readiness {
    Readiness {
        method: "probe",
        request: ic_wire::native::encode_request(canic::GREET, &"Root", canic::LIMITS)
            .expect("probe args"),
        accept: |reply| {
            let greeting: canic::Greeting =
                ic_wire::native::decode_response(canic::GREET, reply, canic::LIMITS)
                    .map_err(|_| Failure::InvalidReply)?;
            if greeting.message.is_empty() {
                Err(Failure::Mismatch)
            } else {
                Ok(())
            }
        },
    }
}
