// Example: create canister request flow.
// Compile with `--features ic` to include the canister module.

#[cfg(feature = "ic")]
mod canister_demo {
    use icu::ops::request::create_canister_request;
    use icu::ops::response::CreateCanisterResponse;
    use icu::{EXAMPLE, IcuError};

    // Not a full canister; demonstrates the call site.
    #[allow(dead_code)]
    async fn create_example_canister() -> Result<CreateCanisterResponse, IcuError> {
        create_canister_request::<()>(&EXAMPLE, None).await
    }
}

fn main() {
    println!("ops_create_canister example");
}
