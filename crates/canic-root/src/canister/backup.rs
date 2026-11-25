//! Explicit private snapshot mounts. No storage, timer or service boot on mounting.
use canic_management::backup;
use canic_management_protocol::backup::{Binding, Error, LIMITS, Record, Request};
use ic_platform_runtime as runtime;

// execute
// Execute the controller's bound snapshot operation and retain its outcome.
// guard  : Root controller
// limits : backup::LIMITS (1,081,344 bytes each way)
canic::__canic_inspect!(wire, "snapshot_execute", update, canic_management_protocol::backup::EXECUTE, Request, Result<Record, Error>, LIMITS, LIMITS);
#[runtime::update(
    runtime = runtime,
    name = "snapshot_execute",
    guard = canic::guard::require_controller,
    decode = decode::<Request, 80>,
    encode = encode::<Result<Record, Error>, 80>,
    max_bytes = 1_081_344,
    on_error = fail,
)]
async fn execute(request: Request) -> Result<Record, Error> {
    backup::execute(request).await
}

// lookup
// Observe the original snapshot operation by its exact binding.
// guard  : Root controller
// limits : 4,096-byte request, backup::LIMITS reply
canic::__canic_inspect!(wire, "snapshot_lookup", update, canic_management_protocol::backup::LOOKUP, Binding, Result<Record, Error>, ic_wire::Limits { bytes: 4096, ..LIMITS }, LIMITS);
#[runtime::update(
    runtime = runtime,
    name = "snapshot_lookup",
    guard = canic::guard::require_controller,
    decode = decode::<Binding, 81>,
    encode = encode::<Result<Record, Error>, 81>,
    max_bytes = 4096,
    on_error = fail,
)]
#[allow(clippy::needless_pass_by_value)]
fn lookup(binding: Binding) -> Result<Record, Error> {
    backup::lookup(&binding)
}

// inject
// Select the controller's interruption point for the installed snapshot fixture.
// guard  : Root controller
// limits : 128-byte request, backup::LIMITS reply
#[cfg(feature = "fault-injection")]
canic::__canic_inspect!(
    wire,
    "snapshot_fault",
    update,
    82,
    u8,
    (),
    ic_wire::Limits {
        bytes: 128,
        ..LIMITS
    },
    LIMITS
);
#[cfg(feature = "fault-injection")]
#[runtime::update(
    runtime = runtime,
    name = "snapshot_fault",
    guard = canic::guard::require_controller,
    decode = decode::<u8, 82>,
    encode = encode::<(), 82>,
    max_bytes = 128,
    on_error = fail,
)]
fn inject(point: u8) {
    backup::inject(point);
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
