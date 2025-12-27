use crate::{ids::CanisterRole, memory::impl_storable_bounded};
use candid::{CandidType, Principal};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

///
/// CanisterEntry
/// Full registry entry (authoritative)
///

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CanisterEntry {
    pub pid: Principal,
    pub role: CanisterRole,
    pub parent_pid: Option<Principal>,
    pub module_hash: Option<Vec<u8>>,
    pub created_at: u64,
}

impl CanisterEntry {
    pub const STORABLE_MAX_SIZE: u32 = 256;
}

impl_storable_bounded!(CanisterEntry, CanisterEntry::STORABLE_MAX_SIZE, false);

///
/// CanisterEntryView
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct CanisterEntryView {
    pub pid: Principal,
    pub role: CanisterRole,
    pub parent_pid: Option<Principal>,
    pub module_hash: Option<Vec<u8>>,
    pub created_at: u64,
}

impl From<&CanisterEntry> for CanisterEntryView {
    fn from(e: &CanisterEntry) -> Self {
        Self {
            pid: e.pid,
            role: e.role.clone(),
            parent_pid: e.parent_pid,
            module_hash: e.module_hash.clone(),
            created_at: e.created_at,
        }
    }
}

impl From<CanisterEntry> for CanisterEntryView {
    fn from(e: CanisterEntry) -> Self {
        Self {
            pid: e.pid,
            role: e.role,
            parent_pid: e.parent_pid,
            module_hash: e.module_hash,
            created_at: e.created_at,
        }
    }
}

///
/// CanisterSummary
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CanisterSummary {
    pub pid: Principal,
    pub role: CanisterRole,
    pub parent_pid: Option<Principal>,
}

impl CanisterSummary {
    pub const STORABLE_MAX_SIZE: u32 = 128;
}

impl From<CanisterEntry> for CanisterSummary {
    fn from(e: CanisterEntry) -> Self {
        Self {
            pid: e.pid,
            role: e.role.clone(),
            parent_pid: e.parent_pid,
        }
    }
}

impl_storable_bounded!(CanisterSummary, CanisterSummary::STORABLE_MAX_SIZE, false);

///
/// CanisterSummaryView
/// Minimal view for children/subnet directories
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CanisterSummaryView {
    pub pid: Principal,
    pub role: CanisterRole,
    pub parent_pid: Option<Principal>,
}

impl From<&CanisterSummary> for CanisterSummaryView {
    fn from(s: &CanisterSummary) -> Self {
        Self {
            pid: s.pid,
            role: s.role.clone(),
            parent_pid: s.parent_pid,
        }
    }
}

impl From<CanisterSummary> for CanisterSummaryView {
    fn from(s: CanisterSummary) -> Self {
        Self {
            pid: s.pid,
            role: s.role,
            parent_pid: s.parent_pid,
        }
    }
}

///
/// TESTS
///

#[cfg(test)]
pub mod test {
    use super::*;
    use crate::cdk::structures::Storable;
    use std::str::FromStr;

    #[test]
    fn canister_entry_fits_within_max() {
        let pid =
            Principal::from_str("pzp6e-ekpqk-3c5x7-2h6so-njoeq-mt45d-h3h6c-q3mxf-vpeq5-fk5o7-yae")
                .unwrap();

        let entry = CanisterEntry {
            pid,
            role: CanisterRole::new("really_long_canister_role"),
            parent_pid: Some(pid),
            module_hash: Some(vec![0u8; 32]),
            created_at: u64::MAX,
        };
        let bytes = entry.to_bytes();

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
            role: CanisterRole::new("really_long_canister_role"),
            parent_pid: Some(pid),
        };
        let bytes = entry.to_bytes();

        assert!(
            bytes.len() <= CanisterSummary::STORABLE_MAX_SIZE as usize,
            "Size {} exceeded bound: {} bytes",
            CanisterSummary::STORABLE_MAX_SIZE,
            bytes.len()
        );
    }
}
