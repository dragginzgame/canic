use crate::dto::prelude::*;

//
// PlacementBindingRegistryEntry
//

#[derive(CandidType, Clone, Debug, Deserialize)]
pub struct PlacementBindingRegistryEntry {
    pub pool: String,
    pub key_value: String,
    pub status: PlacementBindingStatusResponse,
}

//
// PlacementBindingRegistryResponse
//

#[derive(CandidType, Clone, Debug, Deserialize)]
pub struct PlacementBindingRegistryResponse(pub Vec<PlacementBindingRegistryEntry>);

//
// PlacementBindingStatusResponse
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub enum PlacementBindingStatusResponse {
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
// PlacementBindingRecoveryResponse
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub enum PlacementBindingRecoveryResponse {
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
