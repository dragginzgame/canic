//! Module: domain::state
//!
//! Responsibility: define pure state-domain value enums shared by stable
//! records, storage ops, and boundary DTOs.
//! Does not own: stable records, state mutation, DTO structs, or workflow
//! orchestration.
//! Boundary: storage and DTO modules re-export these values to preserve public
//! API paths while internal code imports the domain owner where practical.

use ic_platform::candid::contract::CandidType;
use serde::{Deserialize, Serialize};
use std::{fmt, fmt::Display};

///
/// FleetMode
///

#[derive(CandidType, Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[candid_path("ic_platform::candid::contract")]
pub enum FleetMode {
    #[default]
    Enabled,
    Readonly,
    Disabled,
}

impl Display for FleetMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            Self::Enabled => "Enabled",
            Self::Readonly => "Readonly",
            Self::Disabled => "Disabled",
        };

        f.write_str(label)
    }
}

///
/// FleetStatus
///

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[candid_path("ic_platform::candid::contract")]
pub enum FleetStatus {
    Active,
    Readonly,
    Stopped,
}
