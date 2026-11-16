use canic::{GREET, Greeting, LIMITS, MAX_NAME_BYTES};
use ic_platform_runtime::{self as runtime, system as api};

fn bounded() -> Result<(), String> {
    if ic_platform_runtime::system::argument_size() > LIMITS.bytes {
        return Err("greeting frame too large".into());
    }
    Ok(())
}

fn decode(bytes: &[u8]) -> Result<String, std::convert::Infallible> {
    Ok({
        let name: String = ic_wire::decode(
            bytes,
            ic_wire::Encoding::Frame {
                method: GREET,
                direction: ic_wire::Direction::Request,
            },
            LIMITS,
        )
        .unwrap_or_else(|_| api::trap("invalid greeting frame"));
        if name.len() > MAX_NAME_BYTES {
            api::trap("name exceeds 128 UTF-8 bytes");
        }
        name
    })
}

#[expect(
    clippy::needless_pass_by_value,
    reason = "entrypoint encoder owns return value"
)]
fn encode(value: Greeting) -> Result<Vec<u8>, ic_wire::Error> {
    ic_wire::encode(
        &value,
        ic_wire::Encoding::Frame {
            method: GREET,
            direction: ic_wire::Direction::Response,
        },
        LIMITS,
    )
}

#[runtime::query(runtime = runtime, guard = bounded, decode = decode, max_bytes = 4096, on_error = endpoint_error, name = "greet", encode = encode)]
#[expect(
    clippy::needless_pass_by_value,
    reason = "the entrypoint owns its decoded request"
)]
fn greet(name: String) -> Greeting {
    crate::greeting(&name)
}

// Inter-canister readiness uses a replicated method; it shares greeting admission
// and application behavior rather than introducing a second readiness contract.
#[runtime::update(runtime = runtime, guard = bounded, decode = decode, max_bytes = 4096, on_error = endpoint_error, name = "probe", encode = encode)]
#[expect(
    clippy::needless_pass_by_value,
    reason = "the entrypoint owns its decoded request"
)]
fn probe(name: String) -> Greeting {
    crate::greeting(&name)
}

fn endpoint_error(error: impl std::fmt::Debug) -> ! {
    ic_platform_runtime::system::trap(&format!("endpoint refused: {error:?}"))
}
