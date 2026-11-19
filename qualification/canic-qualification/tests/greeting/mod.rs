//! Direct greeting observation for retained management regressions; not the user CLI.
use ic_testkit::pocket_ic::{CanisterId, nonblocking::PocketIc};

pub async fn call(pic: &PocketIc, canister: CanisterId, name: &str) -> String {
    let request = ic_wire::encode(
        &name,
        ic_wire::Encoding::Frame {
            method: canic_hello::GREET,
            direction: ic_wire::Direction::Request,
        },
        canic_hello::LIMITS,
    )
    .unwrap();
    let bytes = pic
        .query_call(canister, CanisterId::anonymous(), "greet", request)
        .await
        .unwrap();
    let greeting: canic_hello::Greeting = ic_wire::decode(
        &bytes,
        ic_wire::Encoding::Frame {
            method: canic_hello::GREET,
            direction: ic_wire::Direction::Response,
        },
        canic_hello::LIMITS,
    )
    .unwrap();
    greeting.message
}
