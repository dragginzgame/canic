//!
//! Minimal client canister that calls the authority buy endpoint.
//!

#![allow(clippy::unused_async)]

use candid::Principal;
use ic_cdk::call::Call;
use ic_cdk::update;

#[update]
async fn call_buy(authority: Principal) -> Result<(), String> {
    ic_cdk::println!("intent_client: call_buy authority={authority}");
    let call_result = Call::unbounded_wait(authority, "buy").with_arg(1_u64).await;

    match call_result {
        Ok(response) => {
            let result: Result<(), String> = response
                .candid()
                .map_err(|err| format!("decode failed: {err}"))?;
            ic_cdk::println!("intent_client: call_buy result={:?}", result);
            result
        }
        Err(err) => {
            ic_cdk::println!("intent_client: call_buy failed err={err}");
            Err(format!("call failed: {err}"))
        }
    }
}
