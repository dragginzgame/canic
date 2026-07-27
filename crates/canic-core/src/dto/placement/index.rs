use crate::dto::prelude::*;

//
// PlacementIndexRegistryEntry
//

#[derive(CandidType, Clone, Debug, Deserialize)]
pub struct PlacementIndexRegistryEntry {
    pub pool: String,
    pub key_value: String,
    pub status: PlacementIndexStatusResponse,
}

//
// PlacementIndexRegistryResponse
//

#[derive(CandidType, Clone, Debug, Deserialize)]
pub struct PlacementIndexRegistryResponse(pub Vec<PlacementIndexRegistryEntry>);

//
// PlacementIndexStatusResponse
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
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
pub enum PlacementIndexRecoveryResponse {
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
