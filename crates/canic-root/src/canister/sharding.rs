//! Explicit Root operations for controller-approved shard pools.

use super::controller;
use canic::sharding::{
    AssignInstance, AssignShard, Key, LIMITS, LookupInstance, LookupShard, Placement,
};
use canic_management::sharding;
use canic_management_protocol::{
    Refusal,
    stored::{self, ShardPool},
};
use ic_platform_runtime as runtime;

// configure
// Approve the hub and limits for this Root's shard pool.
// guard  : Root controller
// limits : stored::LIMITS request (65,536 bytes), sharding::LIMITS reply (1,024 bytes)
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

// configure_instances
// Retain the project hub's independent dedicated-instance approval.
// guard  : Root controller
// limits : stored::LIMITS request (65,536 bytes), sharding::LIMITS reply (1,024 bytes)
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

// configure_workers
// Approve a hub's worker release, count, funding and deadline.
// guard  : Root controller
// limits : stored::LIMITS request, sharding::LIMITS reply
#[runtime::update(runtime = runtime, name = "configure_workers", guard = controller,
    decode = approval::<stored::InstancePool, {stored::CONFIGURE_WORKERS}>,
    encode = configured::<{stored::CONFIGURE_WORKERS}>, max_bytes = 65536, on_error = fail)]
fn configure_workers(input: stored::InstancePool) -> Result<(), Refusal> {
    sharding::configure_workers(input)
}

// assign_worker
// Recover an ordinal or reserve the next worker.
// guard  : approved worker hub
// limits : default request, 1,024-byte placement reply
#[canic::update(contract = canic::sharding::AssignWorker, guard = sharding::worker_hub)]
fn assign_worker(ordinal: u8) -> Placement {
    sharding::worker(ordinal, true)
}

// lookup_worker
// Observe the original worker allocation for an ordinal.
// guard  : approved worker hub
// limits : default request, 1,024-byte placement reply
#[canic::update(contract = canic::sharding::LookupWorker, guard = sharding::worker_hub)]
fn lookup_worker(ordinal: u8) -> Placement {
    sharding::worker(ordinal, false)
}

// configure_capacity
// Set this subnet owner's shared placement ceiling.
// guard  : Root controller
// limits : stored::LIMITS request (65,536 bytes), sharding::LIMITS reply (1,024 bytes)
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

// increase_pool
// Raise one pool count under its existing funding and deadline.
// guard  : Root controller
// limits : stored::LIMITS request (65,536 bytes), sharding::LIMITS reply (1,024 bytes)
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

// assign_shard
// Return the hub's retained placement for a key or assign it within the approved limits.
// guard  : approved shard hub
// limits : canic::sharding::LIMITS (1,024 bytes each way)
#[canic::update(contract = AssignShard, guard = hub)]
fn assign_shard(key: Key) -> Placement {
    sharding::assign(key)
}

// lookup_shard
// Read a placement for the admitted hub without allocating a shard.
// guard  : approved shard hub
// limits : canic::sharding::LIMITS (1,024 bytes each way)
#[canic::update(contract = LookupShard, guard = hub)]
fn lookup_shard(key: Key) -> Placement {
    sharding::lookup(&key)
}

// assign_instance
// Return the hub's retained placement for a key or assign it within the approved limits.
// guard  : approved project hub
// limits : canic::sharding::LIMITS (1,024 bytes each way)
#[canic::update(contract = AssignInstance, guard = sharding::project_hub)]
fn assign_instance(key: Key) -> Placement {
    sharding::assign_instance(key)
}

// lookup_instance
// Read a placement for the admitted hub without allocating a dedicated child.
// guard  : approved project hub
// limits : canic::sharding::LIMITS (1,024 bytes each way)
#[canic::update(contract = LookupInstance, guard = sharding::project_hub)]
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

fn fail(error: impl std::fmt::Debug) -> ! {
    runtime::system::trap(&format!("shard placement: {error:?}"))
}

// prepare_role_key
// Create or recover the same Root verifying key for receiver setup.
// guard  : Root controller
// limits : default
#[cfg(feature = "role-issuer")]
#[canic::update(contract = canic::topology::attestation::PrepareRoleKey, guard = controller)]
async fn prepare_role_key() -> Result<ic_wire::Bytes<32>, String> {
    canic_management::role_issuer::prepare().await
}

// issue_role
// Issue a two-hour receiver-bound credential for a ready shard, instance or named service.
// limits : default
#[cfg(feature = "role-issuer")]
#[canic::update(contract = canic::topology::attestation::Issue)]
fn issue_role(
    receiver: ic_platform_types::Principal,
) -> Result<canic::topology::attestation::Proof, String> {
    canic_management::role_issuer::issue(receiver)
}

// inspect_shard
// Observe placement custody using the signed operator call path.
// guard  : Root controller
// limits : canic::sharding::LIMITS (1,024 bytes each way)
#[canic::update(contract = canic::sharding::InspectShard, guard = controller)]
fn inspect_shard(
    shard: ic_platform_types::Principal,
) -> Result<canic::sharding::ShardStatus, String> {
    sharding::inspect_shard(shard)
}

// begin_shard_drain
// Stop accepting new keys while continuing to serve existing routes.
// guard  : Root controller
// limits : canic::sharding::LIMITS (1,024 bytes each way)
#[canic::update(contract = canic::sharding::BeginShardDrain, guard = controller)]
fn begin_shard_drain(target: canic::sharding::ShardTarget) -> Result<u32, String> {
    sharding::begin_shard_drain(&target)
}

// release_shard
// Release an exact placement after its hub has committed its application change.
// guard  : approved shard hub
// limits : canic::sharding::LIMITS (1,024 bytes each way)
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
) -> Result<(u64, u32), String> {
    canic_management::role_issuer::retirement_status(subject, &generation)
}

// refill_standby
// Create one empty canister under the exact expiring refill approval.
// guard  : Root controller
// limits : default Wire
#[canic::update(contract = canic::sharding::standby::RefillStandby, guard = controller)]
async fn refill_standby(
    input: canic::sharding::standby::Refill,
) -> Result<canic::sharding::standby::Asset, String> {
    sharding::standby::refill(input).await
}

// inspect_standby
// Read original refill and allocation custody.
// guard  : Root controller
// limits : default request, 4096-byte reply
#[canic::update(contract = canic::sharding::standby::InspectStandby, guard = controller)]
fn inspect_standby() -> Vec<(
    canic::sharding::standby::Refill,
    canic::sharding::standby::Asset,
)> {
    sharding::standby::inspect()
}

// confirm_worker_release
// Withdraw an exact standby allocation once its hub confirms durable release.
// guard  : Root controller
// limits : sharding request, default reply
#[canic::update(contract = canic::sharding::standby::ConfirmRelease, guard = controller)]
async fn confirm_worker_release(target: canic::sharding::ShardTarget) -> Result<(), String> {
    sharding::standby::confirm_release(target).await
}
