//! Module: ops::component_registry::caller_authority::cleanup
//!
//! Bounded terminal journal compaction after application readiness or committed denial.
//!
//! Preserve original scope, census commitment and receiver indexes; never rewrite authority.

use super::*;

impl RootCallerOps {
    pub fn compact(operation: [u8; 32]) -> Result<bool, InternalError> {
        let mut current = Self::operation(operation).ok_or_else(InternalError::invariant)?;
        match current.phase {
            CallerJournalPhase::Compacted => return Ok(true),
            CallerJournalPhase::Compacting => {}
            CallerJournalPhase::Complete => {
                current.phase = CallerJournalPhase::Compacting;
                current.cursor = 0;
                current.cleanup_step = 0;
            }
            _ => return Err(InternalError::conflict()),
        }
        for _ in 0..64 {
            if current.cursor == current.recipient_count {
                current.phase = CallerJournalPhase::Compacted;
                current.cursor = 0;
                current.cleanup_step = 0;
                Self::replace_operation(current)?;
                return Ok(true);
            }
            let recipient = Self::recipient(operation, current.cursor)?;
            if current.cleanup_step < recipient.step_count {
                RootComponentRegistryStore::caller_remove(CallerJournalKey::Step {
                    operation,
                    recipient: current.cursor,
                    ordinal: current.cleanup_step,
                })?;
                current.cleanup_step += 1;
            } else {
                RootComponentRegistryStore::caller_remove(CallerJournalKey::Recipient {
                    operation,
                    ordinal: current.cursor,
                })?;
                current.cursor += 1;
                current.cleanup_step = 0;
            }
        }
        Self::replace_operation(current)?;
        Ok(false)
    }
}
