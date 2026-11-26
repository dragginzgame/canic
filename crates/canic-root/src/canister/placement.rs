//! Root approves child creation, places children and retains their allocation evidence.
//! Hubs own application routing and registries own keyed claims.

use canic::guard::require_controller;
use canic::sharding::LIMITS;
use canic::sharding::Placement;
use canic_management::sharding;
use canic_management_protocol::{Refusal, stored};
use ic_platform_runtime as runtime;

// Approve the creating Root’s permitted effect, requester, limits and artifact.
#[canic::update(contract = stored::ApproveChildCreation, guard = require_controller)]
fn approve_child_creation(grant: stored::ChildCreationGrant) -> Result<(), Refusal> {
    use stored::ChildCreationGrant;
    match grant {
        ChildCreationGrant::Shared(a) => sharding::configure(a),
        ChildCreationGrant::Dedicated(a) => sharding::configure_instances(a),
        ChildCreationGrant::Worker(a) => sharding::configure_workers(a),
        ChildCreationGrant::PerParent(a) => sharding::configure_singleton(a),
    }
}

// configure_subnet_capacity
// Set this subnet owner's shared placement ceiling.
// guard  : Root controller
// limits : stored::LIMITS request (65,536 bytes), sharding::LIMITS reply (1,024 bytes)
canic::__canic_inspect!(wire, "configure_subnet_capacity", update, stored::CONFIGURE_SUBNET_CAPACITY, stored::SubnetCapacity, Result<(), Refusal>, stored::LIMITS, LIMITS);
#[runtime::update(
    runtime = runtime,
    name = "configure_subnet_capacity",
    guard = require_controller,
    decode = approval::<stored::SubnetCapacity, {stored::CONFIGURE_SUBNET_CAPACITY}>,
    encode = configured::<{stored::CONFIGURE_SUBNET_CAPACITY}>,
    max_bytes = 65536,
    on_error = fail,
)]
fn configure_subnet_capacity(input: stored::SubnetCapacity) -> Result<(), Refusal> {
    sharding::configure_subnet_capacity(input)
}

// increase_pool_limit
// Raise one pool count under its existing funding and deadline.
// guard  : Root controller
// limits : stored::LIMITS request (65,536 bytes), sharding::LIMITS reply (1,024 bytes)
canic::__canic_inspect!(wire, "increase_pool_limit", update, stored::INCREASE_POOL_LIMIT, stored::IncreasePoolLimit, Result<(), Refusal>, stored::LIMITS, LIMITS);
#[runtime::update(
    runtime = runtime,
    name = "increase_pool_limit",
    guard = require_controller,
    decode = approval::<stored::IncreasePoolLimit, {stored::INCREASE_POOL_LIMIT}>,
    encode = configured::<{stored::INCREASE_POOL_LIMIT}>,
    max_bytes = 65536,
    on_error = fail,
)]
fn increase_pool_limit(input: stored::IncreasePoolLimit) -> Result<(), Refusal> {
    sharding::increase_pool_limit(input)
}

// lookup_placement
// Read the calling hub's retained assignment without allocating.
#[canic::update(contract = canic::sharding::LookupPlacement)]
fn lookup_placement(request: canic::sharding::Lookup) -> Placement {
    use canic::sharding::Lookup;
    match request {
        Lookup::Instance(key) => sharding::lookup_instance(&key),
        Lookup::Worker(pool, ordinal) => sharding::lookup_worker(&pool, ordinal),
    }
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
    runtime::system::trap(&format!("child placement: {error:?}"))
}

// prepare_role_key
// Create or recover the same Root verifying key for receiver setup.
// guard  : Root controller
// limits : default

#[canic::update(contract = canic::topology::attestation::PrepareRoleKey, guard = require_controller)]
async fn prepare_role_key() -> Result<ic_wire::Bytes<32>, canic::topology::attestation::RoleError> {
    canic_management::role_issuer::prepare().await
}

// issue_role_proof
// Issue a two-hour receiver-bound credential for a ready shard, instance or named service.
// guard  : ready caller with an assigned role
// limits : default

#[canic::update(contract = canic::topology::attestation::Issue)]
fn issue_role_proof(
    receiver: ic_platform_types::Principal,
) -> Result<canic::topology::attestation::Proof, canic::topology::attestation::RoleError> {
    canic_management::role_issuer::issue(receiver)
}

// release_child_allocation
// The approved parent acknowledges release of its original child generation.
#[canic::update(contract = canic::sharding::ReleaseChildAllocation)]
fn release_child_allocation(
    (child, allocation): (ic_platform_types::Principal, canic::sharding::Allocation),
) -> Result<(), canic::sharding::ReleaseError> {
    sharding::release_child_allocation(child, allocation)
}

// close_role_issuance
// Return the original proof deadline for an empty, drained allocation.
// guard  : Root controller
// limits : default

#[canic::update(
    contract = canic::topology::attestation::CloseRoleIssuance,
    guard = require_controller,
)]
fn close_role_issuance(
    (subject, generation): (ic_platform_types::Principal, ic_wire::Bytes<32>),
) -> Result<u64, canic::topology::attestation::RoleError> {
    canic_management::role_issuer::close(subject, &generation)
}

// retirement_status
// Report the proof deadline and live placement count for this closed generation.
// guard  : Root controller
// limits : default

#[canic::query(guard = require_controller)]
fn retirement_status(
    (subject, generation): (ic_platform_types::Principal, ic_wire::Bytes<32>),
) -> Result<(u64, u32), canic::topology::attestation::RoleError> {
    canic_management::role_issuer::retirement_status(subject, &generation)
}

// refill_standby
// Create one empty canister under the exact expiring refill approval.
// guard  : Root controller
// limits : default Wire

#[canic::update(contract = canic::sharding::standby::RefillStandby, guard = require_controller)]
async fn refill_standby(
    input: canic::sharding::standby::Refill,
) -> Result<canic::sharding::standby::Asset, canic::sharding::standby::StandbyPoolError> {
    sharding::standby::refill(input).await
}

// observe_standby
// Read original refill, allocation and transfer custody.
// guard  : Root controller
// limits : default request, 16384-byte reply

#[canic::update(contract = canic::sharding::standby::ObserveStandby, guard = require_controller)]
fn observe_standby() -> Vec<(
    canic::sharding::standby::Source,
    canic::sharding::standby::Asset,
)> {
    sharding::standby::inspect()
}

// confirm_worker_release
// Withdraw an exact worker allocation once its hub confirms durable release.
// guard  : Root controller
// limits : sharding request, default reply
#[canic::update(contract = canic::sharding::standby::ConfirmRelease, guard = require_controller)]
async fn confirm_worker_release(
    target: canic::sharding::ShardTarget,
) -> Result<(), canic::sharding::standby::StandbyPoolError> {
    sharding::standby::confirm_release(target).await
}

// reset_worker
// Advance an exact released worker through stop and uninstall.
// guard  : Root controller
// limits : sharding request, default reply
#[canic::update(contract = canic::sharding::standby::ResetWorker, guard = require_controller)]
async fn reset_worker(
    (target, deadline): (canic::sharding::ShardTarget, u64),
) -> Result<canic::sharding::standby::ResetPhase, canic::sharding::standby::StandbyPoolError> {
    let admit_receiver = crate::auth::require_no_worker_grant;
    sharding::standby::reset(target, deadline, admit_receiver).await
}

// retire_released_child
// Advance the approved dormant installation or verify return after proof expiry.
// guard  : Root controller
// limits : stored request limits, default reply

#[canic::update(contract = stored::RetireReleasedChild, guard = require_controller)]
async fn retire_released_child(
    input: stored::Stage<stored::ChildRetirement>,
) -> Result<stored::RetirementPhase, canic::sharding::standby::StandbyPoolError> {
    let grant = crate::auth::require_no_worker_grant;
    sharding::standby::retire(input, grant).await
}

// configure_standby
// Retain the standby pool policy and start bounded fill.
// guard  : Root controller
// limits : stored request, default reply

#[canic::update(contract = stored::ConfigureStandby, guard = require_controller)]
fn configure_standby(
    policy: stored::StandbyPolicy,
) -> Result<(), canic::sharding::standby::StandbyPoolError> {
    for text in &policy.imports {
        crate::auth::require_no_worker_grant(text.principal.parse().map_err(|_| {
            canic::sharding::standby::StandbyPoolError::InvalidApproval(text.principal.clone())
        })?)
        .map_err(canic::sharding::standby::StandbyPoolError::CustodyOccupied)?;
    }
    sharding::standby::policy::configure(policy)
}

// import_standby
// Observe an approved external handoff or finish its proof quarantine.
// guard  : controller or declared former Root; exact principal checked in handler
// limits : default Wire

#[canic::update(contract = canic::sharding::standby::ImportStandby, guard = sharding::standby::policy::import_caller)]
async fn import_standby(
    id: ic_platform_types::Principal,
) -> Result<canic::sharding::standby::Asset, canic::sharding::standby::StandbyPoolError> {
    sharding::standby::policy::import(id).await
}

// export_standby
// Reserve one empty spare and hand it to its named destination Root.
// guard  : Root controller
// limits : default Wire

#[canic::update(contract = canic::sharding::standby::ExportStandby, guard = require_controller)]
async fn export_standby(
    input: (
        ic_platform_types::Principal,
        ic_platform_types::Principal,
        u64,
    ),
) -> Result<canic::sharding::standby::TransferPhase, canic::sharding::standby::StandbyPoolError> {
    sharding::standby::transfer::export(input.0, input.1, input.2).await
}

// observe_standby_handoff
// Report the destination's retained import state to its declared former Root.
// guard  : exact former Root for this principal
// limits : default Wire

#[canic::update(contract = canic::sharding::standby::ObserveStandbyHandoff, guard = sharding::standby::policy::import_caller)]
fn observe_standby_handoff(
    id: ic_platform_types::Principal,
) -> Result<canic::sharding::standby::Asset, canic::sharding::standby::StandbyPoolError> {
    sharding::standby::policy::handoff_status(id)
}

// standby_status
// Report the subnet fleet's aggregate ready count, target and refill availability.
// guard  : public
// limits : default Wire

#[canic::query(contract = stored::ReadStandbyStatus)]
fn standby_status() -> Option<stored::SpareCounts> {
    sharding::standby::policy::spare_counts()
}

// configure_child_funding
// Start fixed deposits below an approved balance or observe the original grant.
// guard  : Root controller
// limits : default Wire limits
#[canic::update(contract = stored::ConfigureChildFunding, guard = require_controller)]
fn configure_child_funding(
    input: stored::ChildFunding,
) -> Result<stored::ChildFundingStatus, stored::FundingError> {
    sharding::funding::configure(input)
}

// stop_child_funding
// Revoke future payments at their approved owner; retain prior and unresolved debits.
#[canic::update(contract = stored::StopChildFunding, guard = require_controller)]
fn stop_child_funding(
    child: ic_platform_types::Principal,
) -> Result<stored::ChildFundingStatus, stored::FundingError> {
    sharding::funding::stop(child)
}

// acquire_placement
// Recover or pay for the original demand under a requester-funded grant.
// guard  : exact approved hub checked before reading or changing any assignment
// limits : 1 KiB request and reply
#[canic::update(contract = canic::sharding::payment::AcquirePlacement)]
async fn acquire_placement(
    input: (canic::sharding::payment::Demand, u128),
) -> Result<canic::sharding::payment::Payment, canic::sharding::payment::Refusal> {
    sharding::payment::acquire_placement(input.0, input.1).await
}

// observe_child_funding
// Read only the calling child's settled operating grants.
#[canic::update(contract = canic::funding::ObserveChildFunding)]
fn observe_child_funding(_: ()) -> Option<canic::funding::FundingReceipts> {
    sharding::funding::receipts()
}
