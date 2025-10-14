// Example: create-canister request flow.
// Compile with `--features ic` to include the canister module.

#[cfg(feature = "ic")]
mod canister_demo {
    use canic::Error;
    use canic::canister;
    use canic::ops::request::{
        CreateCanisterParent, CreateCanisterResponse, create_canister_request,
    };

    // Not a full canister; demonstrates the call site.
    async fn create_blank_canister() -> Result<CreateCanisterResponse, Error> {
        create_canister_request::<()>(&canister::BLANK, CreateCanisterParent::Caller, None::<()>)
            .await
    }
}

fn main() {
    println!("ops_create_canister example (use --features ic)");
}
