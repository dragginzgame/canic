//
// PlacementIndexRegistryEntry
//

use crate::dto::prelude::*;

#[derive(CandidType, Clone, Debug, Deserialize)]
#[candid_path("ic_platform::candid::contract")]
pub struct PlacementIndexRegistryEntry {
    pub pool: String,
    pub key_value: String,
    pub status: PlacementIndexStatusResponse,
}

//
// PlacementIndexRegistryResponse
//

#[derive(CandidType, Clone, Debug, Deserialize)]
#[candid_path("ic_platform::candid::contract")]
pub struct PlacementIndexRegistryResponse(pub Vec<PlacementIndexRegistryEntry>);

//
// PlacementIndexStatusResponse
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
#[candid_path("ic_platform::candid::contract")]
pub enum PlacementIndexStatusResponse {
    Pending {
        owner_pid: Principal,
        created_at: u64,
        provisional_pid: Option<Principal>,
    },
    Bound {
        instance_pid: Principal,
        bound_at: u64,
    },
}

//
// PlacementIndexRecoveryResponse
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
#[candid_path("ic_platform::candid::contract")]
pub enum PlacementIndexRecoveryResponse {
    ReleasedUnavailableBinding {
        instance_pid: Principal,
        bound_at: u64,
        released_at: u64,
    },
    Missing,
    FreshPending {
        owner_pid: Principal,
        created_at: u64,
        provisional_pid: Option<Principal>,
    },
    Bound {
        instance_pid: Principal,
        bound_at: u64,
    },
    RepairedToBound {
        instance_pid: Principal,
        bound_at: u64,
    },
    ResumedToBound {
        instance_pid: Principal,
        bound_at: u64,
    },
    ReleasedStalePending {
        owner_pid: Principal,
        created_at: u64,
        provisional_pid: Option<Principal>,
        released_at: u64,
    },
}
