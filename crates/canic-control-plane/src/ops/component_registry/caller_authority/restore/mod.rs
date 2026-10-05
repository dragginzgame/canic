//! Module: ops::component_registry::caller_authority::restore
//!
//! Cold validation of immutable publication censuses and independently retained progress.

use super::*;

impl RootCallerOps {
    pub fn restore(issuer: &CallerRootAuthority) -> Result<(), InternalError> {
        let rows = RootComponentRegistryStore::caller_rows();
        if rows.len() != RootComponentRegistryStore::caller_row_count() as usize {
            return Err(InternalError::invariant());
        }
        let mut pending = None;
        for (key, row) in rows {
            match (key, row) {
                (
                    CallerJournalKey::Operation(operation),
                    CallerJournalRowRecord::Operation(record),
                ) => {
                    if record.operation_id != operation
                        || record.issuer != *issuer
                        || record.cursor > record.recipient_count
                    {
                        return Err(InternalError::conflict());
                    }
                    validate_census(&record)?;
                    if !matches!(
                        record.phase,
                        CallerJournalPhase::Complete
                            | CallerJournalPhase::Compacting
                            | CallerJournalPhase::Compacted
                    ) && pending.replace(operation).is_some()
                    {
                        return Err(InternalError::conflict());
                    }
                }
                (
                    CallerJournalKey::Receiver(principal),
                    CallerJournalRowRecord::Receiver(receiver),
                ) => {
                    if receiver.authority.issuer != *issuer
                        || receiver.authority.receiver.canister() != principal
                    {
                        return Err(InternalError::conflict());
                    }
                }
                (
                    CallerJournalKey::Recipient { operation, ordinal },
                    CallerJournalRowRecord::Recipient(_),
                ) => {
                    let operation =
                        Self::operation(operation).ok_or_else(InternalError::invariant)?;
                    if ordinal >= operation.recipient_count {
                        return Err(InternalError::invariant());
                    }
                    if operation.phase == CallerJournalPhase::Compacted
                        || (operation.phase == CallerJournalPhase::Compacting
                            && ordinal < operation.cursor)
                    {
                        return Err(InternalError::invariant());
                    }
                }
                (
                    CallerJournalKey::Step {
                        operation,
                        recipient,
                        ordinal,
                    },
                    CallerJournalRowRecord::Step(_),
                ) => {
                    if ordinal >= Self::recipient(operation, recipient)?.step_count {
                        return Err(InternalError::invariant());
                    }
                    let progress =
                        Self::operation(operation).ok_or_else(InternalError::invariant)?;
                    if progress.phase == CallerJournalPhase::Compacting
                        && recipient == progress.cursor
                        && ordinal < progress.cleanup_step
                    {
                        return Err(InternalError::invariant());
                    }
                }
                (
                    CallerJournalKey::ReceiverRole { role, receiver },
                    CallerJournalRowRecord::Index,
                ) => {
                    if Self::receiver(receiver)
                        .is_none_or(|entry| *entry.authority.receiver.role() != role)
                    {
                        return Err(InternalError::invariant());
                    }
                }
                (
                    CallerJournalKey::SourceReceiver { receiver, .. },
                    CallerJournalRowRecord::Index,
                ) => {
                    if Self::receiver(receiver).is_none() {
                        return Err(InternalError::invariant());
                    }
                }
                _ => return Err(InternalError::invariant()),
            }
        }
        if pending != RootComponentRegistryStore::caller_pending() {
            return Err(InternalError::conflict());
        }
        Ok(())
    }
}

fn validate_census(operation: &CallerJournalRecord) -> Result<(), InternalError> {
    if operation.phase == CallerJournalPhase::Compacted {
        return if operation.cursor == 0 && operation.cleanup_step == 0 {
            Ok(())
        } else {
            Err(InternalError::invariant())
        };
    }
    if operation.phase == CallerJournalPhase::Compacting {
        return validate_compaction(operation);
    }
    if operation.cleanup_step != 0 {
        return Err(InternalError::invariant());
    }
    let mut hash = Sha256::new();
    hash.update(b"canic.caller-census.v1\0");
    let mut steps = 0_u32;
    let mut principals = BTreeSet::new();
    for ordinal in 0..operation.recipient_count {
        let mut recipient = RootCallerOps::recipient(operation.operation_id, ordinal)?;
        if recipient.authority.issuer != operation.issuer
            || !principals.insert(recipient.authority.receiver.canister())
            || recipient.before_count > recipient.step_count
            || recipient.completed_steps > recipient.step_count
        {
            return Err(InternalError::conflict());
        }
        let completed_steps = recipient.completed_steps;
        recipient.completed_steps = 0;
        recipient.reserved = false;
        recipient.startup_released = recipient.kind != CallerRecipientKind::Enrollment;
        hash.update(
            canic_core::cdk::serialize::serialize(&recipient)
                .map_err(|_| InternalError::invariant())?,
        );
        for index in 0..recipient.step_count {
            let step = RootCallerOps::step(operation.operation_id, ordinal, index)?;
            let original = build_step(
                operation.operation_id,
                ordinal,
                index,
                &recipient,
                CallerAuthorityOps::publication_to_dto(step.publication.clone()).change,
            )?;
            if step.publication != original {
                return Err(InternalError::conflict());
            }
            let valid_progress = match index.cmp(&completed_steps) {
                std::cmp::Ordering::Less => step.phase == Some(CallerReceiptPhase::Complete),
                std::cmp::Ordering::Greater => step.phase.is_none(),
                std::cmp::Ordering::Equal => step.phase != Some(CallerReceiptPhase::Complete),
            };
            if !valid_progress {
                return Err(InternalError::conflict());
            }
            hash.update(step.publication.content_hash);
        }
        steps = steps
            .checked_add(recipient.step_count)
            .ok_or_else(InternalError::invariant)?;
    }
    let digest: [u8; 32] = hash.finalize().into();
    if digest != operation.census_hash || steps != operation.step_count {
        return Err(InternalError::conflict());
    }
    Ok(())
}

fn validate_compaction(operation: &CallerJournalRecord) -> Result<(), InternalError> {
    for ordinal in operation.cursor..operation.recipient_count {
        let recipient = RootCallerOps::recipient(operation.operation_id, ordinal)?;
        if recipient.authority.issuer != operation.issuer
            || !recipient.reserved
            || !recipient.startup_released
            || recipient.completed_steps != recipient.step_count
        {
            return Err(InternalError::conflict());
        }
        let first = if ordinal == operation.cursor {
            operation.cleanup_step
        } else {
            0
        };
        if first > recipient.step_count {
            return Err(InternalError::invariant());
        }
        for index in first..recipient.step_count {
            let step = RootCallerOps::step(operation.operation_id, ordinal, index)?;
            let expected = build_step(
                operation.operation_id,
                ordinal,
                index,
                &recipient,
                CallerAuthorityOps::publication_to_dto(step.publication.clone()).change,
            )?;
            if step.publication != expected || step.phase != Some(CallerReceiptPhase::Complete) {
                return Err(InternalError::conflict());
            }
        }
    }
    Ok(())
}
