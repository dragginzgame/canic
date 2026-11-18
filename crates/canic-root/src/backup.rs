//! Explicit private snapshot mounts. No storage, timer or service boot on mounting.
use canic_management::backup;
use canic_management_protocol::backup::{Binding, Error, LIMITS, Record, Request};
use ic_platform_runtime as runtime;

#[runtime::update(runtime = runtime, name = "snapshot_execute", guard = guard,
    decode = decode::<Request, 80>, encode = encode::<Result<Record, Error>, 80>,
    max_bytes = 1_081_344, on_error = fail)]
async fn execute(request: Request) -> Result<Record, Error> {
    backup::execute(request).await
}
#[runtime::update(runtime = runtime, name = "snapshot_lookup", guard = guard,
    decode = decode::<Binding, 81>, encode = encode::<Result<Record, Error>, 81>,
    max_bytes = 4096, on_error = fail)]
#[expect(
    clippy::needless_pass_by_value,
    reason = "Platform owns the decoded ingress value"
)]
fn lookup(binding: Binding) -> Result<Record, Error> {
    backup::lookup(&binding)
}
#[cfg(feature = "fault-injection")]
#[runtime::update(runtime = runtime, name = "snapshot_fault", guard = guard,
    decode = decode::<u8, 82>, encode = encode::<(), 82>, max_bytes = 128, on_error = fail)]
fn inject(point: u8) {
    backup::inject(point);
}
fn guard() -> Result<(), &'static str> {
    if runtime::system::is_controller(&runtime::system::caller()) {
        Ok(())
    } else {
        Err("controller required")
    }
}
fn decode<T: serde::de::DeserializeOwned, const METHOD: u64>(
    bytes: &[u8],
) -> Result<T, ic_wire::Error> {
    ic_wire::decode(
        bytes,
        ic_wire::Encoding::Frame {
            method: METHOD,
            direction: ic_wire::Direction::Request,
        },
        LIMITS,
    )
}
fn encode<T: serde::Serialize, const METHOD: u64>(value: T) -> Result<Vec<u8>, ic_wire::Error> {
    ic_wire::encode(
        &value,
        ic_wire::Encoding::Frame {
            method: METHOD,
            direction: ic_wire::Direction::Response,
        },
        LIMITS,
    )
}
fn fail(error: impl std::fmt::Debug) -> ! {
    runtime::system::trap(&format!("snapshot endpoint: {error:?}"))
}
