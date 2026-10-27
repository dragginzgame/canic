//
// PlacementIndexRegistryEntry
//

use crate::dto::prelude::*;

#[derive(Clone, Debug, Deserialize, serde::Serialize)]
pub struct PlacementIndexRegistryEntry {
    pub pool: String,
    pub key_value: String,
    pub status: PlacementIndexStatusResponse,
}

//
// PlacementIndexRegistryResponse
//

#[derive(Clone, Debug, Deserialize, serde::Serialize)]
pub struct PlacementIndexRegistryResponse(pub Vec<PlacementIndexRegistryEntry>);

//
// PlacementIndexStatusResponse
//

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, serde::Serialize)]
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

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, serde::Serialize)]
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
