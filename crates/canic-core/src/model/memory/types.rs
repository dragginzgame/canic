use crate::{ids::CanisterRole, memory::impl_storable_bounded};
use candid::{CandidType, Principal};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

///
/// CanisterEntry
/// Full registry entry (authoritative)
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct CanisterEntry {
    pub pid: Principal,
    pub ty: CanisterRole,
    pub parent_pid: Option<Principal>,
    pub module_hash: Option<Vec<u8>>,
    pub created_at: u64,
}

impl CanisterEntry {
    pub const STORABLE_MAX_SIZE: u32 = 256;
}

impl_storable_bounded!(CanisterEntry, CanisterEntry::STORABLE_MAX_SIZE, false);

///
/// CanisterSummary
/// Minimal view for children/subnet directories
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CanisterSummary {
    pub pid: Principal,
    pub ty: CanisterRole,
    pub parent_pid: Option<Principal>,
}

impl CanisterSummary {
    pub const STORABLE_MAX_SIZE: u32 = 128;
}

impl From<CanisterEntry> for CanisterSummary {
    fn from(e: CanisterEntry) -> Self {
        Self {
            pid: e.pid,
            ty: e.ty.clone(),
            parent_pid: e.parent_pid,
        }
    }
}

impl_storable_bounded!(CanisterSummary, CanisterSummary::STORABLE_MAX_SIZE, false);

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
            ty: CanisterRole::new("really_long_canister_type"),
            parent_pid: Some(pid),
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

    #[test]
    fn canister_view_fits_within_max() {
        let pid =
            Principal::from_str("pzp6e-ekpqk-3c5x7-2h6so-njoeq-mt45d-h3h6c-q3mxf-vpeq5-fk5o7-yae")
                .unwrap();

        let entry = CanisterSummary {
            pid,
            ty: CanisterRole::new("really_long_canister_type"),
            parent_pid: Some(pid),
        };
        let bytes = Encode!(&entry).unwrap();

        assert!(
            bytes.len() <= CanisterSummary::STORABLE_MAX_SIZE as usize,
            "Size {} exceeded bound: {} bytes",
            CanisterEntry::STORABLE_MAX_SIZE,
            bytes.len()
        );
    }
}
