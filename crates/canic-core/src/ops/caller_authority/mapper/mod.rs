//! Module: ops::caller_authority::mapper
//!
//! Conversion between passive publication DTOs and authoritative receiver records.

use crate::{dto::caller_authority::*, model::caller_authority::*};

pub(super) fn publication_from_dto(
    publication: CallerAuthorityPublication,
) -> CallerPublicationRecord {
    CallerPublicationRecord {
        operation_id: publication.operation_id,
        authority: publication.authority,
        previous_generation: publication.previous_generation,
        generation: publication.generation,
        content_hash: publication.content_hash,
        change: match publication.change {
            CallerAuthorityChange::StageSource(source) => CallerChangeRecord::StageSource(source),
            CallerAuthorityChange::Grant(source) => CallerChangeRecord::Grant(source),
            CallerAuthorityChange::DenySource(source) => CallerChangeRecord::DenySource(source),
            CallerAuthorityChange::DenyComponent(component) => {
                CallerChangeRecord::DenyComponent(component)
            }
            CallerAuthorityChange::OpenReceiver => CallerChangeRecord::OpenReceiver,
            CallerAuthorityChange::RetireReceiver => CallerChangeRecord::RetireReceiver,
        },
    }
}

pub(super) fn receipt_to_dto(receipt: CallerReceiptRecord) -> CallerAuthorityReceipt {
    let publication = receipt.publication;
    CallerAuthorityReceipt {
        phase: match receipt.phase {
            CallerReceiptPhase::Prepared => CallerAuthorityPhase::Prepared,
            CallerReceiptPhase::Committed => CallerAuthorityPhase::Committed,
            CallerReceiptPhase::Complete => CallerAuthorityPhase::Complete,
        },
        publication: publication_to_dto(publication),
    }
}

pub(super) fn status_to_dto(
    header: CallerReceiverRecord,
    receipt: Option<CallerReceiptRecord>,
) -> CallerAuthorityStatus {
    CallerAuthorityStatus {
        readiness: if !crate::ops::runtime::ready::ReadyOps::is_ready() {
            crate::dto::caller_authority::CallerAuthorityReadiness::FrameworkPending
        } else if crate::ops::storage::fleet_activation::FleetActivationOps::require_application_started().is_ok() {
            crate::dto::caller_authority::CallerAuthorityReadiness::ApplicationReady
        } else {
            crate::dto::caller_authority::CallerAuthorityReadiness::FrameworkReady
        },
        authority: header.authority,
        generation: header.generation,
        open: header.open,
        retired: header.retired,
        entries: header.entries,
        reserved_bytes: header.reserved_bytes,
        pending_operation: header.pending_operation,
        receipt: receipt.map(receipt_to_dto),
    }
}

pub(super) fn publication_to_dto(
    publication: CallerPublicationRecord,
) -> CallerAuthorityPublication {
    CallerAuthorityPublication {
        operation_id: publication.operation_id,
        authority: publication.authority,
        previous_generation: publication.previous_generation,
        generation: publication.generation,
        content_hash: publication.content_hash,
        change: match publication.change {
            CallerChangeRecord::StageSource(source) => CallerAuthorityChange::StageSource(source),
            CallerChangeRecord::Grant(source) => CallerAuthorityChange::Grant(source),
            CallerChangeRecord::DenySource(source) => CallerAuthorityChange::DenySource(source),
            CallerChangeRecord::DenyComponent(component) => {
                CallerAuthorityChange::DenyComponent(component)
            }
            CallerChangeRecord::OpenReceiver => CallerAuthorityChange::OpenReceiver,
            CallerChangeRecord::RetireReceiver => CallerAuthorityChange::RetireReceiver,
        },
    }
}
