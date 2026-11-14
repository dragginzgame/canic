use canic::{GREET, Greeting, LIMITS, MAX_NAME_BYTES};
use ic_platform_runtime::cdk::{self, api};

fn bounded() -> Result<(), String> {
    if ic_platform_runtime::ic0::msg_arg_data_size() > LIMITS.bytes {
        return Err("greeting frame too large".into());
    }
    Ok(())
}

#[expect(
    clippy::needless_pass_by_value,
    reason = "entrypoint decoder owns argument buffer"
)]
fn decode(bytes: Vec<u8>) -> String {
    let name: String = ic_wire::native::decode_request(GREET, &bytes, LIMITS)
        .unwrap_or_else(|_| api::trap("invalid greeting frame"));
    if name.len() > MAX_NAME_BYTES {
        api::trap("name exceeds 128 UTF-8 bytes");
    }
    name
}

#[expect(
    clippy::needless_pass_by_value,
    reason = "entrypoint encoder owns return value"
)]
fn encode(value: Greeting) -> Vec<u8> {
    ic_wire::native::encode_response(GREET, &value, LIMITS).expect("bounded greeting")
}

#[cdk::query(guard = "bounded", decode_with = "decode", encode_with = "encode")]
#[expect(
    clippy::needless_pass_by_value,
    reason = "the entrypoint owns its decoded request"
)]
fn greet(name: String) -> Greeting {
    crate::greeting(&name)
}

// Inter-canister readiness uses a replicated method; it shares greeting admission
// and application behavior rather than introducing a second readiness contract.
#[cdk::update(guard = "bounded", decode_with = "decode", encode_with = "encode")]
#[expect(
    clippy::needless_pass_by_value,
    reason = "the entrypoint owns its decoded request"
)]
fn probe(name: String) -> Greeting {
    crate::greeting(&name)
}
