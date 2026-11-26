//! Call the installed demo through its application-owned Wire contract.
use canic::method::Contract;
use example_counter::{CounterMethod, Error, Request, Snapshot};
use ic_testkit::pocket_ic::{CanisterId, nonblocking::PocketIc};

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
            CounterMethod::NAME,
            CounterMethod::encode_request(&request).unwrap(),
        )
        .await
        .unwrap();
    CounterMethod::decode_reply(&reply).unwrap()
}
