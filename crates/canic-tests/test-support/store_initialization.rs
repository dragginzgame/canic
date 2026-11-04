//! Generated Store test entrypoint; the simulator controls the actual IC await.

use ic_platform::management_canister_types::{HttpMethod, HttpRequestArgs, HttpRequestResult};

canic::start_wasm_store!(
    init = {
        let args = HttpRequestArgs {
            url: "https://store-initialization.invalid/94".to_string(),
            max_response_bytes: Some(256),
            method: HttpMethod::GET,
            headers: Vec::new(),
            body: None,
            transform: None,
            is_replicated: None,
            pricing_version: Some(1),
        };
        let cycles = ic_platform::cdk::api::cost_http_request(args.url.len() as u64, 256);
        let response = ic_platform::cdk::call::Call::unbounded_wait(
            ic_platform::Principal::management_canister(),
            "http_request",
        )
        .take_raw_args(ic_platform::candid::contract::encode_one(args).unwrap())
        .with_cycles(cycles)
        .await
        .expect("controlled Store user initialization reply");
        let response: HttpRequestResult =
            ic_platform::candid::decoder::decode_one(&response.into_bytes())
                .expect("HTTP response");
        match response.body.as_slice() {
            b"pending" => std::future::pending::<()>().await,
            b"trap" => ic_platform::cdk::api::trap("controlled Store user initialization trap"),
            b"complete" => (),
            _ => panic!("unknown Store initialization fixture mode"),
        }
    }
);
canic::finish!();
canic::__canic_emit_protected_readiness_endpoint!(before = []);
