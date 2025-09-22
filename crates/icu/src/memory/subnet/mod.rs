mod children;
mod directory;
mod parents;
mod registry;

pub use children::*;
pub use directory::*;
pub use parents::*;
pub use registry::*;

use crate::{ThisError, impl_storable_bounded, types::CanisterType};
use candid::{CandidType, Principal};
use derive_more::Display;
use serde::{Deserialize, Serialize};

///
/// CanisterEntry
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct CanisterEntry {
    pub pid: Principal,
    pub ty: CanisterType,
    pub parent_pid: Option<Principal>,
    pub status: CanisterStatus,
    pub module_hash: Option<Vec<u8>>,
    pub created_at: u64,
}

impl CanisterEntry {
    pub const STORABLE_MAX_SIZE: u32 = 256;
}

impl_storable_bounded!(CanisterEntry, CanisterEntry::STORABLE_MAX_SIZE, false);

///
/// CanisterStatus
///

#[derive(CandidType, Clone, Debug, Deserialize, Display, Eq, PartialEq, Serialize)]
pub enum CanisterStatus {
    Created,
    Installed,
}

///
/// SubnetError
///

#[derive(Debug, ThisError)]
pub enum SubnetError {
    #[error("canister not found: {0}")]
    PrincipalNotFound(Principal),

    #[error("canister not found: {0}")]
    TypeNotFound(CanisterType),

    #[error(transparent)]
    SubnetRegistryError(#[from] SubnetRegistryError),
}

///
/// TESTS
///

#[cfg(test)]
pub mod test {
    use super::*;
    use candid::Encode;
    use std::str::FromStr;

    #[test]
    fn canister_entry_fits_within_max() {
        let pid =
            Principal::from_str("pzp6e-ekpqk-3c5x7-2h6so-njoeq-mt45d-h3h6c-q3mxf-vpeq5-fk5o7-yae")
                .unwrap();

        let entry = CanisterEntry {
            pid,
            ty: CanisterType::new("really_long_canister_type"),
            parent_pid: Some(pid),
            status: CanisterStatus::Installed,
            module_hash: Some(vec![0u8; 32]),
            created_at: u64::MAX,
        };
        let bytes = Encode!(&entry).unwrap();

        assert!(
            bytes.len() <= CanisterEntry::STORABLE_MAX_SIZE as usize,
            "Size {} exceeded bound: {} bytes",
            CanisterEntry::STORABLE_MAX_SIZE,
            bytes.len()
        );
    }
}
