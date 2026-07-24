use crate::dto::{canister::CanisterInfo, prelude::*};

//
// SubnetRegistryResponse
//
// External subnet registry view.
//

#[derive(CandidType, Deserialize)]
pub struct SubnetRegistryResponse(pub Vec<SubnetRegistryEntry>);

//
// SubnetRegistryEntry
//
// Subnet registry entry.
//

#[derive(CandidType, Deserialize)]
pub struct SubnetRegistryEntry {
    pub pid: Principal,
    pub role: CanisterRole,
    pub record: CanisterInfo,
}

//
// FleetDirectoryInput
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct FleetDirectoryInput(pub Vec<IndexEntryInput>);

//
// SubnetDirectoryInput
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct SubnetDirectoryInput(pub Vec<IndexEntryInput>);

//
// IndexEntryInput
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct IndexEntryInput {
    pub role: CanisterRole,
    pub pid: Principal,
}

//
// IndexEntryResponse
//

#[derive(CandidType, Debug, Deserialize, Eq, PartialEq)]
pub struct IndexEntryResponse {
    pub role: CanisterRole,
    pub pid: Principal,
}
