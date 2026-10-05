//! Module: ops::component_registry::caller_authority
//!
//! Exact publication preparation and indexed progress in the Component Registry owner.
//!
//! Workflow supplies a frozen receiver plan; this module never performs remote calls.

mod cleanup;
mod projection;
mod restore;

use crate::storage::stable::component_registry::{
    RootComponentRegistryStore,
    caller_authority::{
        CallerEnrolledReceiverRecord, CallerJournalKey, CallerJournalPhase, CallerJournalRecord,
        CallerJournalRowRecord, CallerLifecycleScope, CallerRecipientKind, CallerRecipientRecord,
        CallerStepRecord,
    },
};
use candid::Principal;
use canic_core::{
    control_plane_support::{
        error::InternalError,
        model::caller_authority::{CallerPublicationRecord, CallerReceiptPhase},
        ops::caller_authority::CallerAuthorityOps,
    },
    dto::caller_authority::{
        CallerAuthorityChange, CallerAuthorityPhase, CallerAuthorityReceipt, CallerAuthorityStatus,
    },
    ids::{CallerReceiverAuthority, CallerRootAuthority, CanisterRole},
};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

/// Complete semantic recipient changes selected before any publication effect.
pub struct CallerReceiverPlan {
    pub authority: CallerReceiverAuthority,
    pub base_generation: u64,
    pub enroll: bool,
    pub retire: bool,
    pub before: Vec<CallerAuthorityChange>,
    pub after: Vec<CallerAuthorityChange>,
}

/// Single-owner facade for the membership operation's durable publication rows.
pub struct RootCallerOps;

impl RootCallerOps {
    pub fn operation(operation: [u8; 32]) -> Option<CallerJournalRecord> {
        match RootComponentRegistryStore::caller_get(CallerJournalKey::Operation(operation)) {
            Some(CallerJournalRowRecord::Operation(row)) => Some(row),
            _ => None,
        }
    }

    pub fn require_mutation_allowed(operation: Option<[u8; 32]>) -> Result<(), InternalError> {
        if RootComponentRegistryStore::caller_pending()
            .is_some_and(|pending| Some(pending) != operation)
        {
            return Err(InternalError::conflict());
        }
        Ok(())
    }

    pub fn receivers(
        role: &CanisterRole,
    ) -> Result<Vec<CallerEnrolledReceiverRecord>, InternalError> {
        RootComponentRegistryStore::caller_receiver_role(role)
            .into_iter()
            .map(|canister| Self::receiver(canister).ok_or_else(InternalError::invariant))
            .filter(|entry| !entry.as_ref().is_ok_and(|entry| entry.retired))
            .collect()
    }

    pub fn source_receivers(
        source: &canic_core::ids::CallerInstallation,
    ) -> Result<Vec<CallerEnrolledReceiverRecord>, InternalError> {
        RootComponentRegistryStore::caller_source_receivers(source)
            .into_iter()
            .filter_map(|key| {
                let CallerJournalKey::SourceReceiver {
                    receiver,
                    receiver_installation,
                    ..
                } = key
                else {
                    return Some(Err(InternalError::invariant()));
                };
                let Some(current) = Self::receiver(receiver) else {
                    return Some(Err(InternalError::invariant()));
                };
                (current.authority.receiver.install_id == receiver_installation && !current.retired)
                    .then_some(Ok(current))
            })
            .collect()
    }

    pub fn receiver(canister: Principal) -> Option<CallerEnrolledReceiverRecord> {
        match RootComponentRegistryStore::caller_get(CallerJournalKey::Receiver(canister)) {
            Some(CallerJournalRowRecord::Receiver(row)) => Some(row),
            _ => None,
        }
    }

    pub fn recipient(
        operation: [u8; 32],
        ordinal: u32,
    ) -> Result<CallerRecipientRecord, InternalError> {
        match RootComponentRegistryStore::caller_get(CallerJournalKey::Recipient {
            operation,
            ordinal,
        }) {
            Some(CallerJournalRowRecord::Recipient(row)) => Ok(row),
            _ => Err(InternalError::invariant()),
        }
    }

    pub fn step(
        operation: [u8; 32],
        recipient: u32,
        ordinal: u32,
    ) -> Result<CallerStepRecord, InternalError> {
        match RootComponentRegistryStore::caller_get(CallerJournalKey::Step {
            operation,
            recipient,
            ordinal,
        }) {
            Some(CallerJournalRowRecord::Step(row)) => Ok(*row),
            _ => Err(InternalError::invariant()),
        }
    }

    /// Freeze exact source/receiver authority and all capacity obligations once.
    pub fn begin(
        operation: [u8; 32],
        issuer: CallerRootAuthority,
        scope: CallerLifecycleScope,
        mut plans: Vec<CallerReceiverPlan>,
    ) -> Result<CallerJournalRecord, InternalError> {
        Self::require_mutation_allowed(Some(operation))?;
        let original = Self::operation(operation);
        if original
            .as_ref()
            .is_some_and(|original| original.issuer != issuer || original.scope != scope)
        {
            return Err(InternalError::conflict());
        }
        if operation == [0; 32] || issuer.install_id == [0; 32] {
            return Err(InternalError::invalid_input());
        }
        plans.sort_by_key(|plan| plan.authority.receiver.canister());
        let recipient_count =
            u32::try_from(plans.len()).map_err(|_| InternalError::unavailable())?;
        let mut rows = Vec::new();
        let mut seen = BTreeSet::new();
        let mut steps = 0_u32;
        let mut digest = Sha256::new();
        digest.update(b"canic.caller-census.v1\0");
        for (ordinal, plan) in plans.into_iter().enumerate() {
            let recipient = u32::try_from(ordinal).map_err(|_| InternalError::unavailable())?;
            if plan.authority.issuer != issuer || !seen.insert(plan.authority.receiver.canister()) {
                return Err(InternalError::conflict());
            }
            let PreparedRecipient {
                record,
                rows: recipient_rows,
            } = prepare_recipient(operation, recipient, plan)?;
            steps = steps
                .checked_add(record.step_count)
                .ok_or_else(InternalError::unavailable)?;
            digest.update(
                canic_core::cdk::serialize::serialize(&record)
                    .map_err(|_| InternalError::invariant())?,
            );
            for (_, row) in &recipient_rows {
                if let CallerJournalRowRecord::Step(step) = row {
                    digest.update(step.publication.content_hash);
                }
            }
            rows.extend(recipient_rows);
        }
        let record = CallerJournalRecord {
            operation_id: operation,
            issuer,
            scope,
            census_hash: digest.finalize().into(),
            recipient_count,
            step_count: steps,
            phase: CallerJournalPhase::Reserving,
            cursor: 0,
            cleanup_step: 0,
        };
        rows.push((
            CallerJournalKey::Operation(operation),
            CallerJournalRowRecord::Operation(record.clone()),
        ));
        if let Some(original) = original {
            if original.census_hash != record.census_hash
                || original.recipient_count != record.recipient_count
                || original.step_count != record.step_count
            {
                return Err(InternalError::conflict());
            }
            return Ok(original);
        }
        RootComponentRegistryStore::caller_begin(operation, rows)?;
        Ok(record)
    }

    /// Preflight every exact receiver before the first staging or denial effect.
    pub fn reserve(
        operation: [u8; 32],
        observed: &CallerAuthorityStatus,
        maximum_entries: u32,
        maximum_bytes: u32,
    ) -> Result<(), InternalError> {
        let current = Self::operation(operation).ok_or_else(InternalError::invariant)?;
        if current.phase != CallerJournalPhase::Reserving {
            return Err(InternalError::conflict());
        }
        let mut recipient = Self::recipient(operation, current.cursor)?;
        let expected = CallerReceiverHead {
            authority: &recipient.authority,
            generation: recipient.base_generation,
        };
        let observed_head = CallerReceiverHead {
            authority: &observed.authority,
            generation: observed.generation,
        };
        let exact = observed_head == expected;
        if !exact
            || observed.retired
            || observed.pending_operation.is_some()
            || observed.open == (recipient.kind == CallerRecipientKind::Enrollment)
        {
            return Err(InternalError::conflict());
        }
        let entries = observed
            .entries
            .checked_add(recipient.additional_entries)
            .ok_or_else(InternalError::unavailable)?;
        let bytes = recipient
            .additional_entries
            .checked_mul(8192)
            .and_then(|extra| observed.reserved_bytes.checked_add(extra))
            .ok_or_else(InternalError::unavailable)?;
        if entries > maximum_entries || bytes > maximum_bytes {
            return Err(InternalError::unavailable());
        }
        recipient.reserved = true;
        Self::replace_recipient(operation, current.cursor, recipient)?;
        Self::advance_cursor(current)
    }

    pub fn next_phase(operation: [u8; 32]) -> Result<CallerJournalRecord, InternalError> {
        let mut current = Self::operation(operation).ok_or_else(InternalError::invariant)?;
        if current.cursor != current.recipient_count {
            return Ok(current);
        }
        current.phase = match current.phase {
            CallerJournalPhase::Reserving => CallerJournalPhase::Preparing,
            CallerJournalPhase::Preparing => CallerJournalPhase::Prepared,
            CallerJournalPhase::Publishing => CallerJournalPhase::Published,
            other => other,
        };
        current.cursor = 0;
        Self::replace_operation(current.clone())?;
        Ok(current)
    }

    pub fn mark_membership_committed(operation: [u8; 32]) -> Result<(), InternalError> {
        let mut current = Self::operation(operation).ok_or_else(InternalError::invariant)?;
        if matches!(
            current.phase,
            CallerJournalPhase::Publishing
                | CallerJournalPhase::Published
                | CallerJournalPhase::Complete
                | CallerJournalPhase::Compacting
                | CallerJournalPhase::Compacted
        ) {
            return Ok(());
        }
        if current.phase != CallerJournalPhase::Prepared {
            return Err(InternalError::conflict());
        }
        current.phase = CallerJournalPhase::Publishing;
        current.cursor = 0;
        Self::replace_operation(current)
    }

    pub fn acknowledge(
        operation: [u8; 32],
        recipient_index: u32,
        step_index: u32,
        observed: CallerAuthorityReceipt,
    ) -> Result<(), InternalError> {
        let current = Self::operation(operation).ok_or_else(InternalError::invariant)?;
        let mut recipient = Self::recipient(operation, recipient_index)?;
        let mut step = Self::step(operation, recipient_index, step_index)?;
        if CallerAuthorityOps::publication_from_dto(observed.publication) != step.publication {
            return Err(InternalError::conflict());
        }
        let phase = match observed.phase {
            CallerAuthorityPhase::Prepared => CallerReceiptPhase::Prepared,
            CallerAuthorityPhase::Committed => CallerReceiptPhase::Committed,
            CallerAuthorityPhase::Complete => CallerReceiptPhase::Complete,
        };
        if step.phase.is_some_and(|retained| retained >= phase) {
            return Ok(());
        }
        if current.cursor != recipient_index
            || recipient.completed_steps != step_index
            || !recipient.reserved
        {
            return Err(InternalError::conflict());
        }
        if !matches!(
            current.phase,
            CallerJournalPhase::Preparing | CallerJournalPhase::Publishing
        ) {
            return Err(InternalError::conflict());
        }
        if current.phase == CallerJournalPhase::Preparing && step_index >= recipient.before_count {
            return Err(InternalError::conflict());
        }
        step.phase = Some(phase);
        RootComponentRegistryStore::caller_replace(
            CallerJournalKey::Step {
                operation,
                recipient: recipient_index,
                ordinal: step_index,
            },
            CallerJournalRowRecord::Step(Box::new(step)),
        )?;
        if phase == CallerReceiptPhase::Complete {
            recipient.completed_steps += 1;
            Self::replace_recipient(operation, recipient_index, recipient)?;
        }
        Ok(())
    }

    pub fn advance_recipient(operation: [u8; 32]) -> Result<(), InternalError> {
        let current = Self::operation(operation).ok_or_else(InternalError::invariant)?;
        let recipient = Self::recipient(operation, current.cursor)?;
        let expected = match current.phase {
            CallerJournalPhase::Preparing => recipient.before_count,
            CallerJournalPhase::Publishing => recipient.step_count,
            _ => return Err(InternalError::conflict()),
        };
        if recipient.completed_steps != expected {
            return Err(InternalError::conflict());
        }
        Self::advance_cursor(current)
    }

    pub fn release_receipt(
        operation: [u8; 32],
        ordinal: u32,
    ) -> Result<Option<CallerPublicationRecord>, InternalError> {
        let recipient = Self::recipient(operation, ordinal)?;
        if recipient.kind != CallerRecipientKind::Enrollment {
            return Ok(None);
        }
        let step = Self::step(
            operation,
            ordinal,
            recipient
                .step_count
                .checked_sub(1)
                .ok_or_else(InternalError::invariant)?,
        )?;
        if step.phase != Some(CallerReceiptPhase::Complete)
            || !matches!(step.publication.change, canic_core::control_plane_support::model::caller_authority::CallerChangeRecord::OpenReceiver)
        {
            return Err(InternalError::conflict());
        }
        Ok(Some(step.publication))
    }

    pub fn mark_startup_released(operation: [u8; 32], ordinal: u32) -> Result<(), InternalError> {
        let current = Self::operation(operation).ok_or_else(InternalError::invariant)?;
        if current.phase != CallerJournalPhase::Published {
            return Err(InternalError::conflict());
        }
        let mut recipient = Self::recipient(operation, ordinal)?;
        recipient.startup_released = true;
        Self::replace_recipient(operation, ordinal, recipient)
    }

    pub fn complete(operation: [u8; 32]) -> Result<(), InternalError> {
        let mut current = Self::operation(operation).ok_or_else(InternalError::invariant)?;
        if matches!(
            current.phase,
            CallerJournalPhase::Complete
                | CallerJournalPhase::Compacting
                | CallerJournalPhase::Compacted
        ) {
            return Ok(());
        }
        if current.phase != CallerJournalPhase::Published {
            return Err(InternalError::conflict());
        }
        for ordinal in 0..current.recipient_count {
            let recipient = Self::recipient(operation, ordinal)?;
            if !recipient.startup_released || recipient.completed_steps != recipient.step_count {
                return Err(InternalError::unavailable());
            }
            let receiver = CallerEnrolledReceiverRecord {
                authority: recipient.authority.clone(),
                generation: recipient.base_generation + u64::from(recipient.step_count),
                retired: recipient.kind == CallerRecipientKind::Retirement,
            };
            RootComponentRegistryStore::caller_replace(
                CallerJournalKey::Receiver(recipient.authority.receiver.canister()),
                CallerJournalRowRecord::Receiver(receiver),
            )?;
        }
        current.phase = CallerJournalPhase::Complete;
        Self::replace_operation(current)?;
        RootComponentRegistryStore::caller_finish(operation)
    }

    fn replace_operation(record: CallerJournalRecord) -> Result<(), InternalError> {
        RootComponentRegistryStore::caller_replace(
            CallerJournalKey::Operation(record.operation_id),
            CallerJournalRowRecord::Operation(record),
        )
    }

    fn replace_recipient(
        operation: [u8; 32],
        ordinal: u32,
        record: CallerRecipientRecord,
    ) -> Result<(), InternalError> {
        RootComponentRegistryStore::caller_replace(
            CallerJournalKey::Recipient { operation, ordinal },
            CallerJournalRowRecord::Recipient(record),
        )
    }

    fn advance_cursor(mut current: CallerJournalRecord) -> Result<(), InternalError> {
        current.cursor = current
            .cursor
            .checked_add(1)
            .ok_or_else(InternalError::invariant)?;
        Self::replace_operation(current)
    }
}

fn reservation_entries(plan: &CallerReceiverPlan, receipts: u32) -> Result<u32, InternalError> {
    let mut sources = BTreeSet::new();
    let mut components = BTreeSet::new();
    for change in plan.before.iter().chain(&plan.after) {
        match change {
            CallerAuthorityChange::StageSource(source) | CallerAuthorityChange::Grant(source) => {
                sources.insert(source.canister());
            }
            CallerAuthorityChange::DenyComponent(component) => {
                components.insert((component.binding.component, component.install_id));
            }
            CallerAuthorityChange::DenySource(_)
            | CallerAuthorityChange::OpenReceiver
            | CallerAuthorityChange::RetireReceiver => {}
        }
    }
    let rows = u32::try_from(sources.len() + components.len())
        .map_err(|_| InternalError::unavailable())?;
    receipts
        .checked_add(rows)
        .ok_or_else(InternalError::unavailable)
}

fn build_step(
    operation: [u8; 32],
    recipient: u32,
    index: u32,
    plan: &CallerRecipientRecord,
    change: CallerAuthorityChange,
) -> Result<CallerPublicationRecord, InternalError> {
    let mut hash = Sha256::new();
    hash.update(b"canic.caller-step.v1\0");
    hash.update(operation);
    hash.update(recipient.to_be_bytes());
    hash.update(index.to_be_bytes());
    let operation_id = hash.finalize().into();
    let generation = plan
        .base_generation
        .checked_add(u64::from(index))
        .ok_or_else(InternalError::invariant)?;
    CallerAuthorityOps::build_publication(plan.authority.clone(), operation_id, generation, change)
        .map(CallerAuthorityOps::publication_from_dto)
        .map_err(|_| InternalError::invariant())
}

impl super::ComponentRegistryOps {
    /// Read exact current installations from the Registry's own allocation and child indexes.
    pub(crate) fn caller_sources(
        selected_component: Option<canic_core::ids::ComponentInstanceId>,
    ) -> Result<Vec<canic_core::ids::CallerInstallation>, InternalError> {
        use canic_core::{
            dto::component_registry::ComponentLifecycleStatus,
            ids::{CallerInstallation, ManagedCanisterBinding},
        };
        let mut sources = Vec::new();
        for partition in Self::root_component_partitions()? {
            let selected = selected_component == Some(partition.binding.component);
            if !selected && partition.status != ComponentLifecycleStatus::Active {
                continue;
            }
            let component_install_id = Self::component_install_id(partition.binding.component)?;
            sources.push(CallerInstallation {
                binding: ManagedCanisterBinding::Component(partition.binding.clone()),
                install_id: component_install_id,
                component_install_id,
            });
            let mut selection = crate::view::component_registry::ComponentDirectoryPageSelection {
                parent_canister_id: None,
                role: None,
                status: Some(ComponentLifecycleStatus::Active),
                start_after: None,
            };
            loop {
                let page = Self::directory_page(partition.binding.component, &selection, 100)?;
                for child in page.entries {
                    sources.push(CallerInstallation {
                        binding: ManagedCanisterBinding::ComponentChild(child.binding),
                        install_id: child.allocation_operation_id,
                        component_install_id,
                    });
                }
                selection.start_after = page.next_cursor;
                if selection.start_after.is_none() {
                    break;
                }
            }
        }
        sources.retain(|source| {
            !RootCallerOps::receiver(source.canister())
                .is_some_and(|receiver| receiver.retired && receiver.authority.receiver == *source)
        });
        Ok(sources)
    }
}

#[derive(Eq, PartialEq)]
struct CallerReceiverHead<'a> {
    authority: &'a CallerReceiverAuthority,
    generation: u64,
}

struct PreparedRecipient {
    record: CallerRecipientRecord,
    rows: Vec<(CallerJournalKey, CallerJournalRowRecord)>,
}

fn prepare_recipient(
    operation: [u8; 32],
    recipient: u32,
    plan: CallerReceiverPlan,
) -> Result<PreparedRecipient, InternalError> {
    let mut rows = Vec::new();
    let before_count =
        u32::try_from(plan.before.len()).map_err(|_| InternalError::unavailable())?;
    let step_count = u32::try_from(plan.before.len() + plan.after.len())
        .map_err(|_| InternalError::unavailable())?;
    let record = CallerRecipientRecord {
        authority: plan.authority.clone(),
        base_generation: plan.base_generation,
        before_count,
        step_count,
        completed_steps: 0,
        additional_entries: reservation_entries(&plan, step_count)?,
        reserved: false,
        startup_released: !plan.enroll,
        kind: match (plan.enroll, plan.retire) {
            (true, false) => CallerRecipientKind::Enrollment,
            (false, true) => CallerRecipientKind::Retirement,
            (false, false) => CallerRecipientKind::Update,
            (true, true) => return Err(InternalError::conflict()),
        },
    };
    for (index, change) in plan.before.into_iter().chain(plan.after).enumerate() {
        let index = u32::try_from(index).map_err(|_| InternalError::unavailable())?;
        if let CallerAuthorityChange::Grant(source) = &change {
            let key = CallerJournalKey::SourceReceiver {
                source: source.canister(),
                installation: source.install_id,
                receiver: record.authority.receiver.canister(),
                receiver_installation: record.authority.receiver.install_id,
            };
            if RootComponentRegistryStore::caller_get(key.clone()).is_none() {
                rows.push((key, CallerJournalRowRecord::Index));
            }
        }
        let publication = build_step(operation, recipient, index, &record, change)?;
        rows.push((
            CallerJournalKey::Step {
                operation,
                recipient,
                ordinal: index,
            },
            CallerJournalRowRecord::Step(Box::new(CallerStepRecord {
                publication,
                phase: None,
            })),
        ));
    }
    if plan.enroll {
        rows.push((
            CallerJournalKey::Receiver(plan.authority.receiver.canister()),
            CallerJournalRowRecord::Receiver(CallerEnrolledReceiverRecord {
                authority: plan.authority.clone(),
                generation: plan.base_generation,
                retired: false,
            }),
        ));
        rows.push((
            CallerJournalKey::ReceiverRole {
                role: plan.authority.receiver.role().clone(),
                receiver: plan.authority.receiver.canister(),
            },
            CallerJournalRowRecord::Index,
        ));
    }
    rows.push((
        CallerJournalKey::Recipient {
            operation,
            ordinal: recipient,
        },
        CallerJournalRowRecord::Recipient(record.clone()),
    ));
    Ok(PreparedRecipient { record, rows })
}
