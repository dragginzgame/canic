//! Explicit Root operations for controller-approved shard pools.
use canic::sharding::{ASSIGN, Key, LIMITS, LOOKUP, Placement};
use canic_management::sharding;
use canic_management_protocol::{
    Refusal,
    stored::{self, ShardPool},
};
use ic_platform_runtime as runtime;

#[runtime::update(runtime = runtime, name = "configure_shards", guard = controller, decode = approval,
 encode = configured, max_bytes = 65536, on_error = fail)]
fn configure(input: ShardPool) -> Result<(), Refusal> {
    sharding::configure(input)
}

#[runtime::update(runtime = runtime, name = "assign_shard", guard = hub, decode = decode::<ASSIGN>, encode = encode::<ASSIGN>, max_bytes = 1024, on_error = fail)]
fn assign(key: Key) -> Placement {
    sharding::assign(key)
}

#[runtime::update(runtime = runtime, name = "lookup_shard", guard = hub, decode = decode::<LOOKUP>, encode = encode::<LOOKUP>, max_bytes = 1024, on_error = fail)]
fn lookup(key: Key) -> Placement {
    sharding::lookup(&key)
}

fn hub() -> Result<(), &'static str> {
    sharding::hub()
}
fn controller() -> Result<(), &'static str> {
    runtime::system::is_controller(&runtime::system::caller())
        .then_some(())
        .ok_or("controller required")
}
fn frame(method: u64, direction: ic_wire::Direction) -> ic_wire::Encoding {
    ic_wire::Encoding::Frame { method, direction }
}
fn approval(b: &[u8]) -> Result<ShardPool, ic_wire::Error> {
    ic_wire::decode(
        b,
        frame(stored::CONFIGURE_SHARDS, ic_wire::Direction::Request),
        stored::LIMITS,
    )
}
fn configured(value: Result<(), Refusal>) -> Result<Vec<u8>, ic_wire::Error> {
    ic_wire::encode(
        &value,
        frame(stored::CONFIGURE_SHARDS, ic_wire::Direction::Response),
        LIMITS,
    )
}
fn decode<const METHOD: u64>(b: &[u8]) -> Result<Key, ic_wire::Error> {
    ic_wire::decode(b, frame(METHOD, ic_wire::Direction::Request), LIMITS)
}
#[expect(
    clippy::needless_pass_by_value,
    reason = "Platform passes the owned reply"
)]
fn encode<const METHOD: u64>(value: Placement) -> Result<Vec<u8>, ic_wire::Error> {
    ic_wire::encode(&value, frame(METHOD, ic_wire::Direction::Response), LIMITS)
}
fn fail(error: impl std::fmt::Debug) -> ! {
    runtime::system::trap(&format!("shard placement: {error:?}"))
}
