//! Direct greeting observation for retained management regressions; not the user CLI.
use ic_testkit::pocket_ic::{CanisterId, nonblocking::PocketIc};

pub async fn call(pic: &PocketIc, canister: CanisterId, name: &str) -> String {
    let request = ic_wire::encode(
        &name,
        ic_wire::Encoding::Frame {
            method: ex_greeting::GREET,
            direction: ic_wire::Direction::Request,
        },
        ex_greeting::LIMITS,
    )
    .unwrap();
    let bytes = pic
        .query_call(canister, CanisterId::anonymous(), "greet", request)
        .await
        .unwrap();
    let greeting: ex_greeting::Greeting = ic_wire::decode(
        &bytes,
        ic_wire::Encoding::Frame {
            method: ex_greeting::GREET,
            direction: ic_wire::Direction::Response,
        },
        ex_greeting::LIMITS,
    )
    .unwrap();
    greeting.message
}
