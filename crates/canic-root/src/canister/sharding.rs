//! Explicit Root operations for controller-approved shard pools.

use canic::guard::require_controller;
#[cfg(feature = "registry")]
use canic::sharding::Key;
use canic::sharding::LIMITS;
#[cfg(any(feature = "registry", feature = "worker-pool", feature = "shard-pool"))]
use canic::sharding::Placement;
#[cfg(feature = "registry")]
use canic::sharding::{AssignInstance, LookupInstance};
use canic_management::sharding;
use canic_management_protocol::{Refusal, stored};
use ic_platform_runtime as runtime;

// configure
// Approve the hub and limits for this Root's shard pool.
// guard  : Root controller
// limits : stored::LIMITS request (65,536 bytes), sharding::LIMITS reply (1,024 bytes)
#[cfg(feature = "shard-pool")]
#[runtime::update(
    runtime = runtime,
    name = "configure_shards",
    guard = require_controller,
    decode = approval::<stored::ShardPool, {stored::CONFIGURE_SHARDS}>,
    encode = configured::<{stored::CONFIGURE_SHARDS}>,
    max_bytes = 65536,
    on_error = fail,
)]
fn configure(input: stored::ShardPool) -> Result<(), Refusal> {
    sharding::configure(input)
}

// configure_instances
// Retain the project hub's independent dedicated-instance approval.
// guard  : Root controller
// limits : stored::LIMITS request (65,536 bytes), sharding::LIMITS reply (1,024 bytes)
#[cfg(feature = "registry")]
#[runtime::update(
    runtime = runtime,
    name = "configure_instances",
    guard = require_controller,
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
#[cfg(feature = "worker-pool")]
#[runtime::update(runtime = runtime, name = "configure_workers", guard = require_controller,
    decode = approval::<stored::InstancePool, {stored::CONFIGURE_WORKERS}>,
    encode = configured::<{stored::CONFIGURE_WORKERS}>, max_bytes = 65536, on_error = fail)]
fn configure_workers(input: stored::InstancePool) -> Result<(), Refusal> {
    sharding::configure_workers(input)
}

// assign_worker
// Recover an ordinal or reserve the next worker.
// guard  : caller must own the named worker grant
// limits : default request, 1,024-byte placement reply
#[cfg(feature = "worker-pool")]
#[canic::update(contract = canic::sharding::AssignWorker)]
fn assign_worker(request: (String, u8)) -> Placement {
    sharding::worker(&request.0, request.1, true)
}

// lookup_worker
// Observe the original worker allocation for an ordinal.
// guard  : caller must own the named worker grant
// limits : default request, 1,024-byte placement reply
#[cfg(feature = "worker-pool")]
#[canic::update(contract = canic::sharding::LookupWorker)]
fn lookup_worker(request: (String, u8)) -> Placement {
    sharding::worker(&request.0, request.1, false)
}

// configure_capacity
// Set this subnet owner's shared placement ceiling.
// guard  : Root controller
// limits : stored::LIMITS request (65,536 bytes), sharding::LIMITS reply (1,024 bytes)
#[runtime::update(
    runtime = runtime,
    name = "configure_capacity",
    guard = require_controller,
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
    guard = require_controller,
    decode = approval::<stored::IncreasePool, {stored::INCREASE_POOL}>,
    encode = configured::<{stored::INCREASE_POOL}>,
    max_bytes = 65536,
    on_error = fail,
)]
fn increase_pool(input: stored::IncreasePool) -> Result<(), Refusal> {
    sharding::increase_pool(input)
}

// assign_instance
// Return the hub's retained placement for a key or assign it within the approved limits.
// guard  : caller must own the dedicated-instance grant
// limits : canic::sharding::LIMITS (1,024 bytes each way)
#[cfg(feature = "registry")]
#[canic::update(contract = AssignInstance)]
fn assign_instance(key: Key) -> Placement {
    sharding::assign_instance(key)
}

// lookup_instance
// Read a placement for the admitted hub without allocating a dedicated child.
// guard  : caller must own the dedicated-instance grant
// limits : canic::sharding::LIMITS (1,024 bytes each way)
#[cfg(feature = "registry")]
#[canic::update(contract = LookupInstance)]
fn lookup_instance(key: Key) -> Placement {
    sharding::lookup_instance(&key)
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
#[canic::update(contract = canic::topology::attestation::PrepareRoleKey, guard = require_controller)]
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
#[cfg(feature = "shard-pool")]
#[canic::update(contract = canic::sharding::InspectShard, guard = require_controller)]
async fn inspect_shard(
    shard: ic_platform_types::Principal,
) -> Result<canic::sharding::ShardStatus, String> {
    sharding::inspect_shard(shard).await
}

// begin_shard_drain
// Stop accepting new keys while continuing to serve existing routes.
// guard  : Root controller
// limits : canic::sharding::LIMITS (1,024 bytes each way)
#[cfg(feature = "shard-pool")]
#[canic::update(contract = canic::sharding::BeginShardDrain, guard = require_controller)]
async fn begin_shard_drain(target: canic::sharding::ShardTarget) -> Result<u32, String> {
    sharding::begin_shard_drain(&target).await
}

// close_role_issuance
// Return the original proof deadline for an empty, drained allocation.
// guard  : Root controller
// limits : default
#[cfg(feature = "role-issuer")]
#[canic::update(
    contract = canic::topology::attestation::CloseRoleIssuance,
    guard = require_controller,
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
#[canic::query(guard = require_controller)]
fn retirement_status(
    (subject, generation): (ic_platform_types::Principal, ic_wire::Bytes<32>),
) -> Result<(u64, u32), String> {
    canic_management::role_issuer::retirement_status(subject, &generation)
}

// refill_standby
// Create one empty canister under the exact expiring refill approval.
// guard  : Root controller
// limits : default Wire
#[cfg(feature = "reserve-pool")]
#[canic::update(contract = canic::sharding::standby::RefillStandby, guard = require_controller)]
async fn refill_standby(
    input: canic::sharding::standby::Refill,
) -> Result<canic::sharding::standby::Asset, String> {
    sharding::standby::refill(input).await
}

// inspect_standby
// Read original refill, allocation and transfer custody.
// guard  : Root controller
// limits : default request, 16384-byte reply
#[cfg(feature = "reserve-pool")]
#[canic::update(contract = canic::sharding::standby::InspectStandby, guard = require_controller)]
fn inspect_standby() -> Vec<(
    canic::sharding::standby::Source,
    canic::sharding::standby::Asset,
)> {
    sharding::standby::inspect()
}

// confirm_worker_release
// Withdraw an exact worker allocation once its hub confirms durable release.
// guard  : Root controller
// limits : sharding request, default reply
#[cfg(feature = "worker-pool")]
#[canic::update(contract = canic::sharding::standby::ConfirmRelease, guard = require_controller)]
async fn confirm_worker_release(target: canic::sharding::ShardTarget) -> Result<(), String> {
    sharding::standby::confirm_release(target).await
}

// reset_worker
// Advance an exact released worker through stop and uninstall.
// guard  : Root controller
// limits : sharding request, default reply
#[cfg(feature = "worker-pool")]
#[canic::update(contract = canic::sharding::standby::ResetWorker, guard = require_controller)]
async fn reset_worker(
    (target, deadline): (canic::sharding::ShardTarget, u64),
) -> Result<canic::sharding::standby::ResetPhase, String> {
    #[cfg(feature = "auth")]
    let admit_receiver = crate::auth::require_no_worker_grant;
    #[cfg(not(feature = "auth"))]
    let admit_receiver = |_| Ok(());
    sharding::standby::reset(target, deadline, admit_receiver).await
}

// retire_shard
// Advance the approved dormant installation or verify return after proof expiry.
// guard  : Root controller
// limits : stored request limits, default reply
#[cfg(feature = "role-issuer")]
#[canic::update(contract = stored::RetireShard, guard = require_controller)]
async fn retire_shard(
    input: stored::Stage<stored::ShardRetirement>,
) -> Result<stored::RetirementPhase, String> {
    #[cfg(feature = "auth")]
    let grant = crate::auth::require_no_worker_grant;
    #[cfg(not(feature = "auth"))]
    let grant = |_| Ok(());
    sharding::standby::retire(input, grant).await
}

// configure_standby
// Retain the reserve policy and start bounded fill.
// guard  : Root controller
// limits : stored request, default reply
#[cfg(feature = "reserve-pool")]
#[canic::update(contract = stored::ConfigureStandby, guard = require_controller)]
fn configure_standby(policy: stored::StandbyPolicy) -> Result<(), String> {
    #[cfg(feature = "auth")]
    for text in &policy.imports {
        crate::auth::require_no_worker_grant(
            text.principal
                .parse()
                .map_err(|_| "invalid import principal")?,
        )?;
    }
    sharding::standby::policy::configure(policy)
}

// import_standby
// Observe an approved external handoff or finish its proof quarantine.
// guard  : controller or declared former Root; exact principal checked in handler
// limits : default Wire
#[cfg(feature = "reserve-pool")]
#[canic::update(contract = canic::sharding::standby::ImportStandby, guard = sharding::standby::policy::import_caller)]
async fn import_standby(
    id: ic_platform_types::Principal,
) -> Result<canic::sharding::standby::Asset, String> {
    sharding::standby::policy::import(id).await
}

// export_standby
// Reserve one empty spare and hand it to its named destination Root.
// guard  : Root controller
// limits : default Wire
#[cfg(feature = "reserve-pool")]
#[canic::update(contract = canic::sharding::standby::ExportStandby, guard = require_controller)]
async fn export_standby(
    input: (
        ic_platform_types::Principal,
        ic_platform_types::Principal,
        u64,
    ),
) -> Result<canic::sharding::standby::TransferPhase, String> {
    sharding::standby::transfer::export(input.0, input.1, input.2).await
}

// standby_handoff_status
// Report the destination's retained import state to its declared former Root.
// guard  : exact former Root for this principal
// limits : default Wire
#[cfg(feature = "reserve-pool")]
#[canic::update(contract = canic::sharding::standby::HandoffStatus, guard = sharding::standby::policy::import_caller)]
fn standby_handoff_status(
    id: ic_platform_types::Principal,
) -> Result<canic::sharding::standby::Asset, String> {
    sharding::standby::policy::handoff_status(id)
}

// spare_counts
// Report the subnet fleet's aggregate ready count, target and refill availability.
// guard  : public
// limits : default Wire
#[cfg(feature = "reserve-pool")]
#[canic::query(contract = stored::ReadSpareCounts)]
fn spare_counts() -> Option<stored::SpareCounts> {
    sharding::standby::policy::spare_counts()
}

// configure_child_funding
// Start fixed deposits below an approved balance or observe the original grant.
// guard  : Root controller
// limits : default Wire limits
#[cfg(feature = "child-funding")]
#[canic::update(contract = stored::ConfigureChildFunding, guard = require_controller)]
async fn configure_child_funding(
    input: stored::ChildFunding,
) -> Result<stored::ChildFundingStatus, String> {
    if input.parent == ic_platform_runtime::system::canister_self() {
        return sharding::funding::configure(input);
    }
    sharding::funding::check_parent(input.parent)?;
    canic::method::call::<stored::ConfigureChildFunding>(input.parent, 15, &input)
        .await
        .map_err(|_| "parent funding approval pending")?
}

// stop_child_funding
// Cancel future deposits while preserving charged and unresolved attempts.
// guard  : Root controller
// limits : default Wire limits
#[canic::update(contract = stored::StopChildFunding, guard = require_controller)]
fn stop_child_funding(
    child: ic_platform_types::Principal,
) -> Result<stored::ChildFundingStatus, String> {
    sharding::funding::stop(child)
}

// stop_application_child_funding
// Revoke future payments at the managed parent that owns the grant.
// guard  : Root controller
// limits : default Wire limits
#[canic::update(contract = stored::StopApplicationChildFunding, guard = require_controller)]
async fn stop_application_child_funding(
    input: (ic_platform_types::Principal, ic_platform_types::Principal),
) -> Result<stored::ChildFundingStatus, String> {
    let (parent, child) = input;
    sharding::funding::check_parent(parent)?;
    canic::method::call::<stored::StopChildFunding>(parent, 10, &child)
        .await
        .map_err(|_| "parent revocation pending")?
}

// fund_placement
// Recover or pay for the original demand under a requester-funded grant.
// guard  : exact approved hub checked before reading or changing any assignment
// limits : 1 KiB request and reply
#[canic::update(contract = canic::sharding::payment::FundPlacement)]
async fn fund_placement(
    input: (canic::sharding::payment::Demand, u128),
) -> Result<canic::sharding::payment::Payment, String> {
    sharding::payment::fund(input.0, input.1).await
}

// grow_shard
// Reserve or recover one shard generation for the named hub-local pool ordinal.
// guard  : approved hub, checked against the named pool
// limits : 1 KiB request and reply
#[cfg(feature = "shard-pool")]
#[canic::update(contract = canic::sharding::GrowShard)]
fn grow_shard(input: (String, u8)) -> (Placement, u32) {
    sharding::grow_shard(&input.0, input.1)
}

// configure_singleton
// Approve a registry's singleton child release, payer and limits.
// guard  : Root controller
// limits : stored::LIMITS request, management protocol reply limits
#[cfg(feature = "registry")]
#[canic::update(contract = stored::ConfigureSingleton, guard = require_controller)]
fn configure_singleton(input: stored::SingletonGrant) -> Result<(), Refusal> {
    sharding::configure_singleton(input)
}

// request_singleton
// Reserve or recover the calling registry instance's own child.
// guard  : ready project allocation checked by Root
// limits : sharding::LIMITS
#[cfg(feature = "registry")]
#[canic::update(contract = canic::sharding::RequestSingleton)]
fn request_singleton(role: String) -> Placement {
    sharding::request_singleton(&role)
}
