//! Module: storage::stable::component_registry::caller_authority
//!
//! Indexed caller-publication evidence retained by the existing Component Registry owner.
//!
//! Rows bind the original lifecycle operation; acknowledgements never rewrite its census.

use candid::Principal;
use canic_core::{
    control_plane_support::model::caller_authority::{CallerPublicationRecord, CallerReceiptPhase},
    ids::{CallerInstallation, CallerReceiverAuthority, CallerRootAuthority, CanisterRole},
};
use serde::{Deserialize, Serialize};

///
/// CallerLifecycleScope
///
/// Exact lifecycle scope, independent of a later Registry census.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum CallerLifecycleScope {
    ActivateComponent(CallerInstallation),
    ActivateChild(CallerInstallation),
    DenyComponent(CallerInstallation),
    DenySubtree(CallerInstallation),
}

///
/// CallerJournalPhase
///
/// Fixed aggregate phase; only the enclosing membership workflow advances it.
///

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum CallerJournalPhase {
    Reserving,
    Preparing,
    Prepared,
    Publishing,
    Published,
    Complete,
    Compacting,
    Compacted,
}

///
/// CallerJournalRecord
///
/// Immutable original census plus bounded independently advanced progress.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CallerJournalRecord {
    pub operation_id: [u8; 32],
    pub issuer: CallerRootAuthority,
    pub scope: CallerLifecycleScope,
    pub census_hash: [u8; 32],
    pub recipient_count: u32,
    pub step_count: u32,
    pub phase: CallerJournalPhase,
    pub cursor: u32,
    pub cleanup_step: u32,
}

///
/// CallerRecipientRecord
///
/// Per-recipient reservation and progress; source changes remain separate indexed rows.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CallerRecipientRecord {
    pub authority: CallerReceiverAuthority,
    pub base_generation: u64,
    pub before_count: u32,
    pub step_count: u32,
    pub completed_steps: u32,
    pub additional_entries: u32,
    pub reserved: bool,
    pub startup_released: bool,
    pub kind: CallerRecipientKind,
}

///
/// CallerRecipientKind
///
/// Enrollment and retirement are mutually exclusive lifecycle obligations.
///

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum CallerRecipientKind {
    Enrollment,
    Update,
    Retirement,
}

///
/// CallerStepRecord
///
/// One immutable publication and its exact observed receiver receipt phase.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CallerStepRecord {
    pub publication: CallerPublicationRecord,
    #[serde(deserialize_with = "required_option")]
    pub phase: Option<CallerReceiptPhase>,
}

///
/// CallerEnrolledReceiverRecord
///
/// Root's confirmed receiver head, updated only from original-operation completion.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CallerEnrolledReceiverRecord {
    pub authority: CallerReceiverAuthority,
    pub generation: u64,
    pub retired: bool,
}

///
/// CallerJournalRowRecord
///
/// Tagged rows share memory ID 20 and the same Registry mutable owner.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum CallerJournalRowRecord {
    Operation(CallerJournalRecord),
    Recipient(CallerRecipientRecord),
    Step(Box<CallerStepRecord>),
    Receiver(CallerEnrolledReceiverRecord),
    Index,
}

///
/// CallerJournalKey
///
/// Exact primary and derived indexes for original operations and receiver relationships.
///

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub enum CallerJournalKey {
    Operation([u8; 32]),
    Recipient {
        operation: [u8; 32],
        ordinal: u32,
    },
    Step {
        operation: [u8; 32],
        recipient: u32,
        ordinal: u32,
    },
    Receiver(Principal),
    ReceiverRole {
        role: CanisterRole,
        receiver: Principal,
    },
    SourceReceiver {
        source: Principal,
        installation: [u8; 32],
        receiver: Principal,
        receiver_installation: [u8; 32],
    },
}

#[cfg(feature = "root-control-plane")]
mod store {
    use super::super::{
        COMPONENT_REGISTRY_ENTRIES, ComponentRegistryEntryIndexKey, ComponentRegistryEntryKey,
        ComponentRegistryEntryRecord, ROOT_COMPONENT_REGISTRY, RootComponentRegistryStore,
    };
    use super::*;
    use canic_core::control_plane_support::error::InternalError;

    /// Retained row budget for the entire publication index, including terminal evidence.
    pub const MAX_CALLER_JOURNAL_ROWS: u32 = 262_144;

    const fn key(index: CallerJournalKey) -> ComponentRegistryEntryKey {
        ComponentRegistryEntryKey {
            component: [0; 32],
            index: ComponentRegistryEntryIndexKey::CallerAuthority(index),
        }
    }

    impl RootComponentRegistryStore {
        pub(crate) fn caller_rows() -> Vec<(CallerJournalKey, CallerJournalRowRecord)> {
            COMPONENT_REGISTRY_ENTRIES.with_borrow(|map| {
                map.range(key(CallerJournalKey::Operation([0; 32]))..)
                    .take_while(|entry| entry.key().component == [0; 32])
                    .filter_map(|entry| {
                        let (key, row) = entry.into_pair();
                        match (key.index, row) {
                            (
                                ComponentRegistryEntryIndexKey::CallerAuthority(key),
                                ComponentRegistryEntryRecord::CallerAuthority(row),
                            ) => Some((key, *row)),
                            _ => None,
                        }
                    })
                    .collect()
            })
        }

        pub(crate) fn caller_row_count() -> u32 {
            ROOT_COMPONENT_REGISTRY.with_borrow(|cell| cell.get().caller_rows)
        }

        pub(crate) fn caller_pending() -> Option<[u8; 32]> {
            ROOT_COMPONENT_REGISTRY.with_borrow(|cell| cell.get().caller_pending)
        }

        pub(crate) fn caller_get(index: CallerJournalKey) -> Option<CallerJournalRowRecord> {
            COMPONENT_REGISTRY_ENTRIES.with_borrow(|map| match map.get(&key(index)) {
                Some(ComponentRegistryEntryRecord::CallerAuthority(row)) => Some(*row),
                _ => None,
            })
        }

        /// Reserve and write the immutable full census atomically before any remote fence.
        pub(crate) fn caller_begin(
            operation: [u8; 32],
            rows: Vec<(CallerJournalKey, CallerJournalRowRecord)>,
        ) -> Result<(), InternalError> {
            if Self::caller_pending().is_some() {
                return Err(InternalError::conflict());
            }
            let mut added = 0_u32;
            let mut removed = Vec::new();
            let mut keys = std::collections::BTreeSet::new();
            for (index, row) in &rows {
                if !keys.insert(index.clone()) {
                    return Err(InternalError::conflict());
                }
                match (Self::caller_get(index.clone()), row) {
                    (None, _) => added += 1,
                    (
                        Some(CallerJournalRowRecord::Receiver(previous)),
                        CallerJournalRowRecord::Receiver(next),
                    ) => {
                        let exact_successor = previous.retired
                            && previous.authority.issuer == next.authority.issuer
                            && previous.authority.receiver.canister()
                                == next.authority.receiver.canister()
                            && previous.authority.receiver.install_id
                                != next.authority.receiver.install_id;
                        if !exact_successor || next.retired || next.generation != 0 {
                            return Err(InternalError::conflict());
                        }
                        if previous.authority.receiver.role() != next.authority.receiver.role() {
                            removed.push(CallerJournalKey::ReceiverRole {
                                role: previous.authority.receiver.role().clone(),
                                receiver: previous.authority.receiver.canister(),
                            });
                        }
                    }
                    (Some(CallerJournalRowRecord::Index), CallerJournalRowRecord::Index)
                        if matches!(index, CallerJournalKey::ReceiverRole { .. }) => {}
                    _ => return Err(InternalError::conflict()),
                }
                Self::caller_validate_row(row)?;
            }
            let removed_count =
                u32::try_from(removed.len()).map_err(|_| InternalError::invariant())?;
            let next_count = ROOT_COMPONENT_REGISTRY
                .with_borrow(|cell| cell.get().caller_rows.checked_add(added))
                .and_then(|count| count.checked_sub(removed_count))
                .filter(|count| *count <= MAX_CALLER_JOURNAL_ROWS)
                .ok_or_else(InternalError::unavailable)?;
            ROOT_COMPONENT_REGISTRY.with_borrow_mut(|cell| {
                let mut state = cell.get().clone();
                state.caller_pending = Some(operation);
                state.caller_rows = next_count;
                cell.set(state);
            });
            for index in removed {
                COMPONENT_REGISTRY_ENTRIES.with_borrow_mut(|map| {
                    map.remove(&key(index));
                });
            }
            for (index, row) in rows {
                Self::caller_write(index, row);
            }
            Ok(())
        }

        pub(crate) fn caller_replace(
            index: CallerJournalKey,
            row: CallerJournalRowRecord,
        ) -> Result<(), InternalError> {
            if Self::caller_get(index.clone()).is_none() {
                return Err(InternalError::invariant());
            }
            Self::caller_validate_row(&row)?;
            Self::caller_write(index, row);
            Ok(())
        }

        pub(crate) fn caller_remove(index: CallerJournalKey) -> Result<(), InternalError> {
            if Self::caller_get(index.clone()).is_none() {
                return Err(InternalError::invariant());
            }
            ROOT_COMPONENT_REGISTRY.with_borrow_mut(|cell| -> Result<(), InternalError> {
                let mut state = cell.get().clone();
                state.caller_rows = state
                    .caller_rows
                    .checked_sub(1)
                    .ok_or_else(InternalError::invariant)?;
                cell.set(state);
                Ok(())
            })?;
            COMPONENT_REGISTRY_ENTRIES.with_borrow_mut(|map| {
                map.remove(&key(index));
            });
            Ok(())
        }

        pub(crate) fn caller_finish(operation: [u8; 32]) -> Result<(), InternalError> {
            ROOT_COMPONENT_REGISTRY.with_borrow_mut(|cell| {
                let mut state = cell.get().clone();
                if state.caller_pending != Some(operation) {
                    return Err(InternalError::conflict());
                }
                state.caller_pending = None;
                cell.set(state);
                Ok(())
            })
        }

        pub(crate) fn caller_receiver_role(role: &CanisterRole) -> Vec<Principal> {
            let start = key(CallerJournalKey::ReceiverRole {
                role: role.clone(),
                receiver: Principal::management_canister(),
            });
            COMPONENT_REGISTRY_ENTRIES.with_borrow(|map| {
                map.range(start..).map(|entry| entry.key().clone()).take_while(|entry| matches!(&entry.index, ComponentRegistryEntryIndexKey::CallerAuthority(CallerJournalKey::ReceiverRole { role: found, .. }) if found == role))
                    .filter_map(|entry| match entry.index { ComponentRegistryEntryIndexKey::CallerAuthority(CallerJournalKey::ReceiverRole { receiver, .. }) => Some(receiver), _ => None }).collect()
            })
        }

        pub(crate) fn caller_source_receivers(
            source: &CallerInstallation,
        ) -> Vec<CallerJournalKey> {
            let start = key(CallerJournalKey::SourceReceiver {
                source: source.canister(),
                installation: source.install_id,
                receiver: Principal::management_canister(),
                receiver_installation: [0; 32],
            });
            COMPONENT_REGISTRY_ENTRIES.with_borrow(|map| {
                map.range(start..).map(|entry| entry.key().clone())
                    .take_while(|entry| matches!(&entry.index, ComponentRegistryEntryIndexKey::CallerAuthority(CallerJournalKey::SourceReceiver { source: found, installation, .. }) if *found == source.canister() && *installation == source.install_id))
                    .filter_map(|entry| match entry.index { ComponentRegistryEntryIndexKey::CallerAuthority(index) => Some(index), _ => None }).collect()
            })
        }

        fn caller_validate_row(row: &CallerJournalRowRecord) -> Result<(), InternalError> {
            let encoded = canic_core::cdk::serialize::serialize(
                &ComponentRegistryEntryRecord::CallerAuthority(Box::new(row.clone())),
            )
            .map_err(|_| InternalError::invariant())?;
            if encoded.len() > super::super::COMPONENT_REGISTRY_ENTRY_RECORD_MAX_BYTES as usize {
                return Err(InternalError::unavailable());
            }
            Ok(())
        }

        fn caller_write(index: CallerJournalKey, row: CallerJournalRowRecord) {
            COMPONENT_REGISTRY_ENTRIES.with_borrow_mut(|map| {
                map.insert(
                    key(index),
                    ComponentRegistryEntryRecord::CallerAuthority(Box::new(row)),
                );
            });
        }
    }
}

/// Keep nullable fields mandatory in the maintained stable-record contract.
pub(super) fn required_option<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer)
}
