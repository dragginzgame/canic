use canic_management::{self as management, Readiness, endpoint};
use canic_management_protocol::{Failure, Refusal, Stage, Status};
use ic_platform_runtime::cdk;

#[cdk::init]
fn init() {
    management::open(true, readiness());
}
#[cdk::post_upgrade]
fn post_upgrade() {
    management::open(false, readiness());
}
#[cdk::query(
    guard = "endpoint::public",
    decode_with = "endpoint::decode_status",
    encode_with = "endpoint::encode_status"
)]
fn status() -> Status {
    management::status()
}
// Replicated observation provides certified ingress evidence without dispatching
// management work, changing approval, or requiring an unexpired effect allowance.
#[cdk::update(
    guard = "endpoint::controller",
    decode_with = "endpoint::decode_status",
    encode_with = "endpoint::encode_status"
)]
fn observe() -> Status {
    management::status()
}
#[cdk::update(
    guard = "endpoint::uploader",
    decode_with = "endpoint::decode_stage",
    encode_with = "endpoint::encode_stage"
)]
fn stage(input: Stage) -> Result<Status, Refusal> {
    management::stage(input)
}
#[cdk::update(
    guard = "endpoint::controller",
    decode_with = "endpoint::decode_reconcile",
    encode_with = "endpoint::encode_reconcile"
)]
async fn reconcile() -> Status {
    management::reconcile().await
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
