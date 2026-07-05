//! Module: domain::state
//!
//! Responsibility: define pure state-domain value enums shared by stable
//! records, storage ops, and boundary DTOs.
//! Does not own: stable records, state mutation, DTO structs, or workflow
//! orchestration.
//! Boundary: storage and DTO modules re-export these values to preserve public
//! API paths while internal code imports the domain owner where practical.

use candid::CandidType;
use serde::{Deserialize, Serialize};
use std::fmt::{self, Display};

///
/// AppMode
///

#[derive(CandidType, Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub enum AppMode {
    #[default]
    Enabled,
    Readonly,
    Disabled,
}

impl Display for AppMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            Self::Enabled => "Enabled",
            Self::Readonly => "Readonly",
            Self::Disabled => "Disabled",
        };

        f.write_str(label)
    }
}
