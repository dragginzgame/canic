//! Explicit Root operations for controller-approved shard pools.

use super::controller;
use canic::sharding::{ASSIGN, ASSIGN_INSTANCE, Key, LIMITS, LOOKUP, LOOKUP_INSTANCE, Placement};
use canic_management::sharding;
use canic_management_protocol::{
    Refusal,
    stored::{self, ShardPool},
};
use ic_platform_runtime as runtime;

/// Approve the hub and limits for this Root's shard pool.
#[runtime::update(
    runtime = runtime,
    name = "configure_shards",
    guard = controller,
    decode = approval::<ShardPool, {stored::CONFIGURE_SHARDS}>,
    encode = configured::<{stored::CONFIGURE_SHARDS}>,
    max_bytes = 65536,
    on_error = fail,
)]
fn configure(input: ShardPool) -> Result<(), Refusal> {
    sharding::configure(input)
}

/// Retain the project hub's independent dedicated-instance approval.
#[runtime::update(
    runtime = runtime,
    name = "configure_instances",
    guard = controller,
    decode = approval::<stored::InstancePool, {stored::CONFIGURE_INSTANCES}>,
    encode = configured::<{stored::CONFIGURE_INSTANCES}>,
    max_bytes = 65536,
    on_error = fail,
)]
fn configure_instances(input: stored::InstancePool) -> Result<(), Refusal> {
    sharding::configure_instances(input)
}

/// Set this subnet owner's shared placement ceiling.
#[runtime::update(
    runtime = runtime,
    name = "configure_capacity",
    guard = controller,
    decode = approval::<stored::SubnetCapacity, {stored::CONFIGURE_CAPACITY}>,
    encode = configured::<{stored::CONFIGURE_CAPACITY}>,
    max_bytes = 65536,
    on_error = fail,
)]
fn configure_capacity(input: stored::SubnetCapacity) -> Result<(), Refusal> {
    sharding::configure_capacity(input)
}

/// Raise one pool count under its existing funding and deadline.
#[runtime::update(
    runtime = runtime,
    name = "increase_pool",
    guard = controller,
    decode = approval::<stored::IncreasePool, {stored::INCREASE_POOL}>,
    encode = configured::<{stored::INCREASE_POOL}>,
    max_bytes = 65536,
    on_error = fail,
)]
fn increase_pool(input: stored::IncreasePool) -> Result<(), Refusal> {
    sharding::increase_pool(input)
}

/// Return the hub's retained placement for a key or assign it within the approved limits.
#[runtime::update(
    runtime = runtime,
    name = "assign_shard",
    guard = hub,
    decode = decode::<ASSIGN>,
    encode = encode::<ASSIGN>,
    max_bytes = 1024,
    on_error = fail,
)]
fn assign(key: Key) -> Placement {
    sharding::assign(key)
}

/// Read a placement for the admitted hub without allocating a shard.
#[runtime::update(
    runtime = runtime,
    name = "lookup_shard",
    guard = hub,
    decode = decode::<LOOKUP>,
    encode = encode::<LOOKUP>,
    max_bytes = 1024,
    on_error = fail,
)]
fn lookup(key: Key) -> Placement {
    sharding::lookup(&key)
}

/// Return the hub's retained placement for a key or assign it within the approved limits.
#[runtime::update(
    runtime = runtime,
    name = "assign_instance",
    guard = sharding::project_hub,
    decode = decode::<ASSIGN_INSTANCE>,
    encode = encode::<ASSIGN_INSTANCE>,
    max_bytes = 1024,
    on_error = fail,
)]
fn assign_instance(key: Key) -> Placement {
    sharding::assign_instance(key)
}

/// Read a placement for the admitted hub without allocating a dedicated child.
#[runtime::update(
    runtime = runtime,
    name = "lookup_instance",
    guard = sharding::project_hub,
    decode = decode::<LOOKUP_INSTANCE>,
    encode = encode::<LOOKUP_INSTANCE>,
    max_bytes = 1024,
    on_error = fail,
)]
fn lookup_instance(key: Key) -> Placement {
    sharding::lookup_instance(&key)
}

/// Require the caller bound as the approved pool hub.
fn hub() -> Result<(), &'static str> {
    sharding::hub()
}

fn frame(method: u64, direction: ic_wire::Direction) -> ic_wire::Encoding {
    ic_wire::Encoding::Frame { method, direction }
}

fn approval<T: serde::de::DeserializeOwned, const METHOD: u64>(
    b: &[u8],
) -> Result<T, ic_wire::Error> {
    ic_wire::decode(
        b,
        frame(METHOD, ic_wire::Direction::Request),
        stored::LIMITS,
    )
}

fn configured<const METHOD: u64>(value: Result<(), Refusal>) -> Result<Vec<u8>, ic_wire::Error> {
    ic_wire::encode(&value, frame(METHOD, ic_wire::Direction::Response), LIMITS)
}

fn decode<const METHOD: u64>(b: &[u8]) -> Result<Key, ic_wire::Error> {
    ic_wire::decode(b, frame(METHOD, ic_wire::Direction::Request), LIMITS)
}

#[allow(clippy::needless_pass_by_value)]
fn encode<const METHOD: u64>(value: Placement) -> Result<Vec<u8>, ic_wire::Error> {
    ic_wire::encode(&value, frame(METHOD, ic_wire::Direction::Response), LIMITS)
}

fn fail(error: impl std::fmt::Debug) -> ! {
    runtime::system::trap(&format!("shard placement: {error:?}"))
}

/// Create or recover the same Root verifying key for receiver setup.
#[cfg(feature = "role-issuer")]
#[canic::update(contract = canic::topology::attestation::PrepareRoleKey, guard = controller)]
async fn prepare_role_key() -> Result<ic_wire::Bytes<32>, String> {
    canic_management::role_issuer::prepare().await
}

/// Issue a two-hour credential only for a ready shared-shard caller.
#[cfg(feature = "role-issuer")]
#[canic::update(contract = canic::topology::attestation::Issue)]
fn issue_role(
    receiver: ic_platform_types::Principal,
) -> Result<canic::topology::attestation::Proof, String> {
    canic_management::role_issuer::issue(receiver)
}

/// Observe placement custody using the signed operator call path.
#[canic::update(contract = canic::sharding::InspectShard, guard = controller)]
fn inspect_shard(
    shard: ic_platform_types::Principal,
) -> Result<canic::sharding::ShardStatus, String> {
    sharding::inspect_shard(shard)
}

/// Stop accepting new keys while continuing to serve existing routes.
#[canic::update(contract = canic::sharding::BeginShardDrain, guard = controller)]
fn begin_shard_drain(target: canic::sharding::ShardTarget) -> Result<u16, String> {
    sharding::begin_shard_drain(&target)
}

/// Release an exact placement after its hub has committed its application change.
#[canic::update(contract = canic::sharding::ReleaseShard, guard = hub)]
fn release_shard((key, target): (Key, canic::sharding::ShardTarget)) -> Result<(), String> {
    sharding::release_shard(&key, &target)
}

// close_role_issuance
// Return the original proof deadline for an empty, drained allocation.
// guard  : Root controller
// limits : default
#[cfg(feature = "role-issuer")]
#[canic::update(
    contract = canic::topology::attestation::CloseRoleIssuance,
    guard = controller,
)]
fn close_role_issuance(
    (subject, generation): (ic_platform_types::Principal, ic_wire::Bytes<32>),
) -> Result<u64, String> {
    canic_management::role_issuer::close(subject, &generation)
}

// retirement_status
// Report the proof deadline and live placement count for this closed generation.
// guard  : Root controller
// limits : default
#[cfg(feature = "role-issuer")]
#[canic::query(guard = controller)]
fn retirement_status(
    (subject, generation): (ic_platform_types::Principal, ic_wire::Bytes<32>),
) -> Result<(u64, u16), String> {
    canic_management::role_issuer::retirement_status(subject, &generation)
}
