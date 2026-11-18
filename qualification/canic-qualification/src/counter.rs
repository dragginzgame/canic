//! Call the installed demo through its application-owned Wire contract.
use canic_demo::{Error, Request, Snapshot};
use ic_testkit::pocket_ic::{CanisterId, nonblocking::PocketIc};
use ic_wire::{Direction, Encoding};

/// Send the application-owned counter request as the selected caller.
/// # Panics
/// Panics if the fixture transport or Wire frame fails.
pub async fn counter(
    pic: &PocketIc,
    id: CanisterId,
    caller: CanisterId,
    request: Request,
) -> Result<Snapshot, Error> {
    let reply = pic
        .update_call(
            id,
            caller,
            "counter",
            ic_wire::encode(
                &request,
                Encoding::Frame {
                    method: canic_demo::METHOD,
                    direction: Direction::Request,
                },
                canic_demo::LIMITS,
            )
            .unwrap(),
        )
        .await
        .unwrap();
    ic_wire::decode(
        &reply,
        Encoding::Frame {
            method: canic_demo::METHOD,
            direction: Direction::Response,
        },
        canic_demo::LIMITS,
    )
    .unwrap()
}
