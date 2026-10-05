//! Module: ops::caller_authority
//!
//! Indexed receiver publication storage, conversion, and local admission projection.
//!
//! Endpoint authentication and publication orchestration remain outside this module.

mod cleanup;
mod mapper;
#[cfg(test)]
mod tests;

use mapper::publication_from_dto;

use crate::{
    cdk::serialize::serialize,
    config::caller_authority::CompiledCallerPolicy,
    ids::{CallerReceiverAuthority, ComponentInstanceId},
    model::caller_authority::{
        CallerAdmissionError, CallerChangeRecord, CallerPublicationError, CallerPublicationRecord,
        CallerReceiptPhase, CallerReceiptRecord, CallerReceiverRecord, CallerReservation,
        CallerRowKey, CallerSourceRecord,
    },
    storage::stable::caller_authority::{
        CallerReceiverStore, CallerRowRecord, CallerRowStore, MAX_CALLER_ROW_BYTES,
    },
    view::caller_authority::CallerAdmissionView,
};
use candid::Principal;
use sha2::{Digest, Sha256};

/// Deterministic owner of receiver state and exact original-operation receipts.
pub struct CallerAuthorityOps;

impl CallerAuthorityOps {
    /// Project an immutable stored publication for exact protected delivery.
    #[must_use]
    pub fn publication_to_dto(
        publication: CallerPublicationRecord,
    ) -> crate::dto::caller_authority::CallerAuthorityPublication {
        mapper::publication_to_dto(publication)
    }

    /// Convert authenticated boundary data to the protected publication model.
    #[must_use]
    pub fn publication_from_dto(
        publication: crate::dto::caller_authority::CallerAuthorityPublication,
    ) -> CallerPublicationRecord {
        mapper::publication_from_dto(publication)
    }

    /// Project retained publication evidence without recomputing authority.
    #[must_use]
    pub fn receipt_to_dto(
        receipt: CallerReceiptRecord,
    ) -> crate::dto::caller_authority::CallerAuthorityReceipt {
        mapper::receipt_to_dto(receipt)
    }

    /// Project the control header and one original-operation receipt.
    #[must_use]
    pub fn status_to_dto(
        header: CallerReceiverRecord,
        receipt: Option<CallerReceiptRecord>,
    ) -> crate::dto::caller_authority::CallerAuthorityStatus {
        mapper::status_to_dto(header, receipt)
    }
    /// Convert the issuer's typed command into the canonical bounded publication DTO.
    pub fn build_publication(
        authority: CallerReceiverAuthority,
        operation_id: [u8; 32],
        previous_generation: u64,
        change: crate::dto::caller_authority::CallerAuthorityChange,
    ) -> Result<crate::dto::caller_authority::CallerAuthorityPublication, CallerPublicationError>
    {
        let draft =
            publication_from_dto(crate::dto::caller_authority::CallerAuthorityPublication {
                authority,
                operation_id,
                previous_generation,
                generation: 0,
                content_hash: [0; 32],
                change,
            });
        let publication = Self::publication(
            draft.authority,
            operation_id,
            previous_generation,
            draft.change,
        )?;
        Ok(mapper::publication_to_dto(publication))
    }

    /// Restore and verify durable rows synchronously before lifecycle hooks are scheduled.
    pub fn restore(
        expected: &CallerReceiverAuthority,
        policy: &CompiledCallerPolicy,
    ) -> Result<(), CallerPublicationError> {
        let header = Self::receiver().ok_or(CallerPublicationError::AuthorityConflict)?;
        if header.authority != *expected
            || expected.policy_digest != policy.digest
            || expected.receiver.role() != &policy.role
            || !policy.is_canonical()
        {
            return Err(CallerPublicationError::AuthorityConflict);
        }
        let rows = CallerRowStore::rows();
        if rows.len() != header.entries as usize
            || header
                .entries
                .checked_mul(MAX_CALLER_ROW_BYTES)
                .and_then(|bytes| {
                    bytes.checked_add(crate::config::caller_authority::CALLER_HEADER_BYTES)
                })
                != Some(header.reserved_bytes)
            || (header.open && (header.generation == 0 || header.retired))
        {
            return Err(CallerPublicationError::InvalidPublication);
        }
        let capacity_matches = policy.configuration.as_ref().map_or(
            header.entries <= 8 && header.reserved_bytes <= 65_536,
            |config| {
                header.entries <= config.maximum_entries
                    && header.reserved_bytes <= config.maximum_bytes
            },
        );
        if !capacity_matches {
            return Err(CallerPublicationError::Capacity);
        }
        let mut pending = None;
        for (key, row) in rows {
            match row {
                CallerRowRecord::Receipt(receipt) => {
                    validate_hash(&receipt.publication)?;
                    if receipt.publication.authority != *expected
                        || key != CallerRowKey::receipt(receipt.publication.operation_id)
                    {
                        return Err(CallerPublicationError::AuthorityConflict);
                    }
                    if receipt.phase == CallerReceiptPhase::Complete {
                        if receipt.publication.generation > header.generation {
                            return Err(CallerPublicationError::GenerationConflict);
                        }
                    } else if pending.replace(receipt.publication.operation_id).is_some()
                        || receipt.publication.previous_generation != header.generation
                        || header.generation.checked_add(1) != Some(receipt.publication.generation)
                    {
                        return Err(CallerPublicationError::GenerationConflict);
                    }
                }
                CallerRowRecord::Source(source) => {
                    validate_source_installation(&source.installation, &expected.issuer)?;
                    validate_source_generation(&source, &header)?;
                    if key != CallerRowKey::source(source.installation.canister())
                        || source.installation.component().authority != expected.issuer.registry
                        || source.installation.component().fleet_subnet_root != expected.issuer.root
                        || (source.denied && source.open)
                    {
                        return Err(CallerPublicationError::AuthorityConflict);
                    }
                }
                CallerRowRecord::ComponentFence(component) => {
                    if key
                        != CallerRowKey::component(
                            component.binding.component,
                            component.install_id,
                        )
                        || component.binding.authority != expected.issuer.registry
                        || component.binding.fleet_subnet_root != expected.issuer.root
                        || component.install_id == [0; 32]
                    {
                        return Err(CallerPublicationError::AuthorityConflict);
                    }
                }
            }
        }
        if pending != header.pending_operation {
            return Err(CallerPublicationError::ReplayConflict);
        }
        if header.cleanup_cursor.is_some()
            && !pending.and_then(Self::receipt).is_some_and(|receipt| {
                receipt.phase == CallerReceiptPhase::Committed
                    && matches!(
                        receipt.publication.change,
                        CallerChangeRecord::DenyComponent(_)
                    )
            })
        {
            return Err(CallerPublicationError::PhaseConflict);
        }
        Ok(())
    }
    /// Initialize once from the protected install payload, even for an empty receiver policy.
    pub fn initialize(authority: CallerReceiverAuthority) -> Result<(), CallerPublicationError> {
        if CallerReceiverStore::get().is_some() || !CallerRowStore::rows().is_empty() {
            return Err(CallerPublicationError::AuthorityConflict);
        }
        CallerReceiverStore::set(CallerReceiverRecord::new(authority));
        Ok(())
    }

    /// Inspect current authority without querying Root or the management canister.
    #[must_use]
    pub fn receiver() -> Option<CallerReceiverRecord> {
        CallerReceiverStore::get()
    }

    /// Reconcile an original operation from its retained indexed receipt.
    #[must_use]
    pub fn receipt(operation_id: [u8; 32]) -> Option<CallerReceiptRecord> {
        match CallerRowStore::get(&CallerRowKey::receipt(operation_id)) {
            Some(CallerRowRecord::Receipt(receipt)) => Some(*receipt),
            _ => None,
        }
    }

    /// Construct an exact, bounded publication identity before persisting the Root journal.
    pub fn publication(
        authority: CallerReceiverAuthority,
        operation_id: [u8; 32],
        previous_generation: u64,
        change: CallerChangeRecord,
    ) -> Result<CallerPublicationRecord, CallerPublicationError> {
        let generation = previous_generation
            .checked_add(1)
            .ok_or(CallerPublicationError::GenerationConflict)?;
        let mut publication = CallerPublicationRecord {
            authority,
            operation_id,
            previous_generation,
            generation,
            change,
            content_hash: [0; 32],
        };
        publication.content_hash = publication_hash(&publication)?;
        Ok(publication)
    }

    /// Reserve every row and byte before writing any denial fence.
    pub(crate) fn prepare(
        publication: CallerPublicationRecord,
        policy: &CompiledCallerPolicy,
        source_permitted: bool,
    ) -> Result<CallerReceiptRecord, CallerPublicationError> {
        validate_hash(&publication)?;
        if let Some(receipt) = Self::receipt(publication.operation_id) {
            return receipt.advance(&publication, CallerReceiptPhase::Prepared);
        }
        let header = CallerReceiverStore::get().ok_or(CallerPublicationError::AuthorityConflict)?;
        if policy.digest != header.authority.policy_digest
            || policy.role != *header.authority.receiver.role()
        {
            return Err(CallerPublicationError::AuthorityConflict);
        }
        let changed = prepare_row(&publication, source_permitted)?;
        let receipt = CallerReceiptRecord {
            publication: publication.clone(),
            phase: CallerReceiptPhase::Prepared,
        };
        let receipt_key = CallerRowKey::receipt(publication.operation_id);
        let receipt_row = CallerRowRecord::Receipt(Box::new(receipt.clone()));
        let (mut entries, mut bytes) = row_reservation(&receipt_key, &receipt_row)?;
        if let Some((key, row)) = &changed {
            let (added_entries, added_bytes) = row_reservation(key, row)?;
            entries += added_entries;
            bytes = bytes
                .checked_add(added_bytes)
                .ok_or(CallerPublicationError::Capacity)?;
        }
        let (maximum_entries, maximum_bytes) =
            policy.configuration.as_ref().map_or((8, 65_536), |config| {
                (config.maximum_entries, config.maximum_bytes)
            });
        let successor = header.reserve(
            &publication,
            CallerReservation {
                entries,
                bytes,
                maximum_entries,
                maximum_bytes,
            },
        )?;
        if let Some((key, row)) = changed {
            CallerRowStore::insert(key, row);
        }
        CallerRowStore::insert(receipt_key, receipt_row);
        CallerReceiverStore::set(successor);
        Ok(receipt)
    }

    /// Open only a prepared grant after Root has committed active membership.
    pub fn commit(
        publication: &CallerPublicationRecord,
    ) -> Result<CallerReceiptRecord, CallerPublicationError> {
        let original =
            Self::receipt(publication.operation_id).ok_or(CallerPublicationError::PhaseConflict)?;
        let receipt = original.advance(publication, CallerReceiptPhase::Committed)?;
        if original.phase != CallerReceiptPhase::Prepared {
            return Ok(receipt);
        }
        if let CallerChangeRecord::Grant(source) = &publication.change {
            let key = CallerRowKey::source(source.canister());
            let Some(CallerRowRecord::Source(mut row)) = CallerRowStore::get(&key) else {
                return Err(CallerPublicationError::AuthorityConflict);
            };
            if row.installation != *source || row.generation != publication.generation || row.denied
            {
                return Err(CallerPublicationError::AuthorityConflict);
            }
            row.open = true;
            CallerRowStore::insert(key, CallerRowRecord::Source(row));
        }
        CallerRowStore::insert(
            CallerRowKey::receipt(publication.operation_id),
            CallerRowRecord::Receipt(Box::new(receipt.clone())),
        );
        Ok(receipt)
    }

    /// Release the serial receiver boundary without deleting earlier receipt evidence.
    pub fn complete(
        publication: &CallerPublicationRecord,
    ) -> Result<CallerReceiptRecord, CallerPublicationError> {
        let original =
            Self::receipt(publication.operation_id).ok_or(CallerPublicationError::PhaseConflict)?;
        let receipt = original.advance(publication, CallerReceiptPhase::Complete)?;
        if original.phase == CallerReceiptPhase::Complete {
            return Ok(receipt);
        }
        if !Self::cleanup_component(&original)? {
            return Ok(original);
        }
        let header = CallerReceiverStore::get().ok_or(CallerPublicationError::AuthorityConflict)?;
        let successor = header.finish(&original)?;
        CallerRowStore::insert(
            CallerRowKey::receipt(publication.operation_id),
            CallerRowRecord::Receipt(Box::new(receipt.clone())),
        );
        CallerReceiverStore::set(successor);
        Ok(receipt)
    }

    /// Borrow an indexed admission projection for the workflow-selected pure decision.
    pub(crate) fn with_admission<T>(
        caller: Principal,
        check: impl FnOnce(&CallerAdmissionView<'_>) -> Result<T, CallerAdmissionError>,
    ) -> Result<T, CallerAdmissionError> {
        let header =
            CallerReceiverStore::get().ok_or(CallerAdmissionError::AuthorityUnavailable)?;
        let source = match CallerRowStore::get(&CallerRowKey::source(caller)) {
            Some(CallerRowRecord::Source(source)) => Some(source),
            _ => None,
        };
        let component_fenced = source.as_ref().is_some_and(|source| {
            component_fenced(
                source.installation.component().component,
                source.installation.component_install_id,
            )
        });
        check(&CallerAdmissionView {
            receiver: &header.authority,
            generation: header.generation,
            receiver_open: header.open && !header.retired,
            source: source.as_ref().map(|source| &source.installation),
            source_open: source
                .as_ref()
                .is_some_and(|source| source.open && !source.denied),
            component_fenced,
        })
    }
}

fn prepare_row(
    publication: &CallerPublicationRecord,
    source_permitted: bool,
) -> Result<Option<(CallerRowKey, CallerRowRecord)>, CallerPublicationError> {
    match &publication.change {
        CallerChangeRecord::StageSource(source)
        | CallerChangeRecord::Grant(source)
        | CallerChangeRecord::DenySource(source) => {
            let grant = !matches!(publication.change, CallerChangeRecord::DenySource(_));
            validate_source_installation(source, &publication.authority.issuer)?;
            if grant && !source_permitted {
                return Err(CallerPublicationError::AuthorityConflict);
            }
            let key = CallerRowKey::source(source.canister());
            if let Some(CallerRowRecord::Source(existing)) = CallerRowStore::get(&key) {
                let same_installation = existing.installation == *source;
                let previous_fenced = existing.denied
                    || component_fenced(
                        existing.installation.component().component,
                        existing.installation.component_install_id,
                    );
                if !same_installation && (!grant || !previous_fenced) {
                    return Err(CallerPublicationError::AuthorityConflict);
                }
                if grant && same_installation && existing.denied {
                    return Err(CallerPublicationError::Retired);
                }
            }
            if grant && component_fenced(source.component().component, source.component_install_id)
            {
                return Err(CallerPublicationError::Retired);
            }
            Ok(Some((
                key,
                CallerRowRecord::Source(CallerSourceRecord {
                    installation: source.clone(),
                    generation: publication.generation,
                    open: false,
                    denied: !grant,
                }),
            )))
        }
        CallerChangeRecord::DenyComponent(component) => {
            if component.install_id == [0; 32]
                || component.binding.authority != publication.authority.issuer.registry
                || component.binding.fleet_subnet_root != publication.authority.issuer.root
            {
                return Err(CallerPublicationError::AuthorityConflict);
            }
            let key = CallerRowKey::component(component.binding.component, component.install_id);
            if let Some(CallerRowRecord::ComponentFence(existing)) = CallerRowStore::get(&key)
                && existing != *component
            {
                return Err(CallerPublicationError::AuthorityConflict);
            }
            Ok(Some((
                key,
                CallerRowRecord::ComponentFence(component.clone()),
            )))
        }
        CallerChangeRecord::OpenReceiver | CallerChangeRecord::RetireReceiver => Ok(None),
    }
}

fn validate_source_installation(
    source: &crate::ids::CallerInstallation,
    issuer: &crate::ids::CallerRootAuthority,
) -> Result<(), CallerPublicationError> {
    let owns_installation = !matches!(
        source.binding,
        crate::ids::ManagedCanisterBinding::Component(_)
    ) || source.install_id == source.component_install_id;
    let exact_root = source.component().authority == issuer.registry
        && source.component().fleet_subnet_root == issuer.root;
    if !owns_installation
        || !exact_root
        || source.install_id == [0; 32]
        || source.component_install_id == [0; 32]
    {
        return Err(CallerPublicationError::AuthorityConflict);
    }
    Ok(())
}

fn validate_source_generation(
    source: &CallerSourceRecord,
    header: &CallerReceiverRecord,
) -> Result<(), CallerPublicationError> {
    if source.generation == 0 {
        return Err(CallerPublicationError::GenerationConflict);
    }
    if source.generation <= header.generation {
        return Ok(());
    }
    let pending = header
        .pending_operation
        .and_then(CallerAuthorityOps::receipt)
        .ok_or(CallerPublicationError::GenerationConflict)?;
    let matching_source = match &pending.publication.change {
        CallerChangeRecord::StageSource(installation)
        | CallerChangeRecord::Grant(installation)
        | CallerChangeRecord::DenySource(installation) => installation == &source.installation,
        _ => false,
    };
    if pending.publication.generation != source.generation || !matching_source {
        return Err(CallerPublicationError::GenerationConflict);
    }
    Ok(())
}

fn component_fenced(component: ComponentInstanceId, installation: [u8; 32]) -> bool {
    matches!(CallerRowStore::get(&CallerRowKey::component(component, installation)), Some(CallerRowRecord::ComponentFence(fence)) if fence.install_id == installation)
}

fn row_reservation(
    key: &CallerRowKey,
    row: &CallerRowRecord,
) -> Result<(u32, u32), CallerPublicationError> {
    let new_size = serialize(row)
        .map_err(|_| CallerPublicationError::InvalidPublication)?
        .len();
    if new_size > MAX_CALLER_ROW_BYTES as usize {
        return Err(CallerPublicationError::Capacity);
    }
    // Reserve the fixed encoded-record ceiling once. Later phases cannot fail for known capacity.
    Ok(if CallerRowStore::get(key).is_some() {
        (0, 0)
    } else {
        (1, MAX_CALLER_ROW_BYTES)
    })
}

fn validate_hash(publication: &CallerPublicationRecord) -> Result<(), CallerPublicationError> {
    if publication_hash(publication)? != publication.content_hash {
        return Err(CallerPublicationError::InvalidPublication);
    }
    Ok(())
}

fn publication_hash(
    publication: &CallerPublicationRecord,
) -> Result<[u8; 32], CallerPublicationError> {
    let mut canonical = publication.clone();
    canonical.content_hash = [0; 32];
    let bytes = serialize(&canonical).map_err(|_| CallerPublicationError::InvalidPublication)?;
    if bytes.len() > MAX_CALLER_ROW_BYTES as usize {
        return Err(CallerPublicationError::Capacity);
    }
    let mut hash = Sha256::new();
    hash.update(b"canic.caller-publication.v1\0");
    hash.update(bytes);
    Ok(hash.finalize().into())
}
