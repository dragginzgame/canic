//! Module: ops::caller_authority::cleanup
//!
//! Bounded source compaction under the original Component denial receipt.
//!
//! The installation-bound fence and publication ordering survive row removal.

use super::*;

const SOURCE_CLEANUP_PAGE: usize = 128;

impl CallerAuthorityOps {
    pub(super) fn cleanup_component(
        receipt: &CallerReceiptRecord,
    ) -> Result<bool, CallerPublicationError> {
        let CallerChangeRecord::DenyComponent(component) = &receipt.publication.change else {
            return Ok(true);
        };
        if receipt.phase != CallerReceiptPhase::Committed {
            return Err(CallerPublicationError::PhaseConflict);
        }
        let mut header = Self::receiver().ok_or(CallerPublicationError::AuthorityConflict)?;
        if header.pending_operation != Some(receipt.publication.operation_id)
            || header.authority != receipt.publication.authority
        {
            return Err(CallerPublicationError::ReplayConflict);
        }
        let mut page =
            CallerRowStore::source_page(header.cleanup_cursor.clone(), SOURCE_CLEANUP_PAGE + 1);
        let finished = page.len() <= SOURCE_CLEANUP_PAGE;
        page.truncate(SOURCE_CLEANUP_PAGE);
        if let Some((key, _)) = page.last() {
            header.cleanup_cursor = Some(key.clone());
        }
        let removed: Vec<_> = page
            .into_iter()
            .filter_map(|(key, source)| {
                (source.installation.component() == &component.binding
                    && source.installation.component_install_id == component.install_id)
                    .then_some(key)
            })
            .collect();
        let count = u32::try_from(removed.len()).map_err(|_| CallerPublicationError::Capacity)?;
        header.entries = header
            .entries
            .checked_sub(count)
            .ok_or(CallerPublicationError::InvalidPublication)?;
        header.reserved_bytes = header
            .reserved_bytes
            .checked_sub(count * MAX_CALLER_ROW_BYTES)
            .ok_or(CallerPublicationError::InvalidPublication)?;
        for key in removed {
            CallerRowStore::remove(&key);
        }
        CallerReceiverStore::set(header);
        Ok(finished)
    }
}
