//! Installed Auth receiver and checkpoint-interruption fixture with direct Canic Root enrollment.
//! Fixture control is installation-owned, never browser-selected authority.
#[cfg(not(feature = "session-mount"))]
mod canister;
mod frames;
mod issuer;
#[cfg(all(feature = "profile", not(feature = "session-mount")))]
mod profile;
#[cfg(feature = "session-mount")]
mod stable;

pub use canic::auth::{enrollment, protocol};
// The interruption witness deliberately skips Auth checkpointing on upgrade.
// All participants still own real leases in its single application directory.
#[cfg(any(not(feature = "session-mount"), feature = "profile"))]
fn witness_runtime<M: ic_storage::MemoryBacking>(
    memory: M,
    fresh: bool,
) -> ic_storage::MemoryRuntime<M> {
    use ic_storage::{
        AllocationGrants, GenericAllocationPolicy, MemoryAuthority, MemoryRequest, MemoryRuntime,
        SealedDeclarationSnapshot, StorageConfig,
    };
    assert_eq!(memory.size() == 0, fresh);
    let requests = ["auth.receiver.engine.v1", issuer::KEY, issuer::HOST_KEY]
        .map(|key| MemoryRequest::new("auth", key).unwrap())
        .to_vec();
    let grants = vec![MemoryAuthority::new("auth", "auth.").unwrap()];
    #[cfg(feature = "profile")]
    let (requests, grants) = {
        use canic_profile_worker::state::{STORAGE_AUTHORITY, STORAGE_PREFIX, memory_requests};
        let mut requests = requests;
        requests.extend(memory_requests());
        let mut grants = grants;
        grants.push(MemoryAuthority::new(STORAGE_AUTHORITY, STORAGE_PREFIX).unwrap());
        (requests, grants)
    };
    let mut runtime = MemoryRuntime::new_with_config(
        memory,
        StorageConfig {
            allocations: requests.len(),
            pages: 320,
            ..Default::default()
        },
    )
    .unwrap();
    if !fresh {
        assert_eq!(runtime.memory_allocations().unwrap().len(), requests.len());
        for request in &requests {
            assert!(runtime.allocation_id(request.stable_key()).is_some());
        }
    }
    runtime
        .bootstrap(
            &SealedDeclarationSnapshot::new(requests).unwrap(),
            &AllocationGrants::new(grants).unwrap(),
            &GenericAllocationPolicy,
        )
        .unwrap();
    runtime
}

/// Read the witness profiles from the original composed directory.
/// # Panics
/// Requires an intact profile witness image.
#[cfg(feature = "profile")]
pub fn open_profiles<M: ic_storage::MemoryBacking>(
    backing: M,
    fresh: bool,
) -> canic_profile_worker::state::State<M> {
    let runtime = witness_runtime(backing, fresh);
    if fresh {
        canic_profile_worker::state::State::create(&runtime)
    } else {
        canic_profile_worker::state::State::open(&runtime)
    }
}
use ic_auth_protocol_types::{AudienceId, CanonicalId};
use serde::{Deserialize, Serialize};

/// A fixed test namespace, independent of credential-supplied audience fields.
#[must_use]
pub const fn audience() -> AudienceId {
    AudienceId {
        canonical_network_id: CanonicalId::from_bytes([0x11; 32]),
        fleet_id: CanonicalId::from_bytes([0x22; 32]),
    }
}
/// Public Ed25519 test verification key (private seed is `[7; 32]`).
pub const KEY: [u8; 32] = [
    234, 74, 108, 99, 226, 156, 82, 10, 190, 245, 80, 123, 19, 46, 197, 249, 149, 71, 118, 174,
    190, 190, 123, 146, 66, 30, 234, 105, 20, 70, 210, 44,
];
/// Test observation contract.
pub const OBSERVE: u64 = 901;
/// Application endpoint budget for the witness's retained issuer callback.
pub const CALLBACK_LIMITS: ic_wire::Limits = {
    let limits = canic::auth::issuer::REQUEST_LIMITS;
    ic_wire::Limits {
        bytes: limits.bytes,
        allocation: limits.allocation,
        nodes: limits.nodes,
        depth: limits.depth,
    }
};
/// Small fixture control/observation budget.
pub const LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 4096,
    allocation: 16384,
    nodes: 128,
    depth: 8,
};

/// Passive counts; never a readiness or authority capability.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Observation {
    /// Whether the original volatile receiver still exists.
    pub present: bool,
    /// Mutating handlers entered, including ordinary refusals.
    pub calls: u32,
    /// Application writes performed under current Auth authorization.
    pub writes: u32,
    /// Sessions, replay records and declared encoded bytes for the fixed account.
    pub usage: (u64, u64, u64),
    /// Retained proof-verification work credit.
    pub credit: u32,
    /// Supplier metadata revision/generation, independent of host entry projection.
    pub enrollment: Option<(u64, u64)>,
}

/// Independent fixture counters at actual enrollment commit boundaries.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RefreshObservation {
    /// Successful local reservations, before the entropy await.
    pub reserved: u32,
    /// Prepared direct Root calls, before the snapshot await.
    pub dispatched: u32,
    /// Authenticated Root replies captured before an optional fixture-only barrier.
    pub fetched: u32,
    /// Committed supplier metadata and application entry publications.
    pub installed: u32,
    /// Original receipt deadline of the latest successful reservation.
    pub deadline_ns: u64,
}

/// Issuer fixture observations independent of returned credentials.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct IssuerObservation {
    /// Original volatile owner still exists.
    pub present: bool,
    /// Completed local signatures committed with the corresponding result.
    pub signatures: u32,
    /// Supplier retained operation count.
    pub operations: u32,
    /// Supplier retained login count.
    pub logins: u32,
    /// Supplier current admission epoch.
    pub epoch: u64,
}

/// Application-owned refusal returned in the project's normal Wire reply.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProjectError {
    /// The verified subject does not own this fixture project.
    Forbidden,
}
