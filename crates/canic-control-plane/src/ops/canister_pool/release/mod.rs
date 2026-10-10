//! Read the bounded pool singleton without applying admission rules for new effects.

use crate::{
    dto::root::{RootPoolBootstrapReleaseEvidence, RootPoolReleaseResponse},
    ops::canister_pool::{CanisterPoolOps, capacity_import, creation_to_dto},
    storage::stable::canister_pool::CanisterPoolStore,
};
use canic_contracts::dto::pool::CanisterPoolHandoff;
use canic_core::{cdk::types::Principal, control_plane_support::error::InternalError};

impl CanisterPoolOps {
    /// Preserve issued effects, exhaustion and historical receipts without touching physical assets.
    pub(crate) fn release_status(
        root: Principal,
    ) -> Result<RootPoolReleaseResponse, InternalError> {
        let state = CanisterPoolStore::state();
        let import_matches = state
            .capacity_import
            .as_ref()
            .is_none_or(|record| record.reservation.root == root);
        let creation_matches = state
            .creation
            .as_ref()
            .is_none_or(|record| record.root == root);
        if !import_matches || !creation_matches {
            return Err(InternalError::conflict());
        }
        Ok(RootPoolReleaseResponse {
            root,
            bootstrap: state
                .bootstrap_import
                .map(|hold| RootPoolBootstrapReleaseEvidence {
                    review_sha256: hold.review_sha256,
                    install_id: hold.install_id,
                    operator: hold.operator,
                    store: hold.store,
                    sources: hold.sources,
                }),
            capacity_import: state.capacity_import.as_ref().map(capacity_import::status),
            creation: state.creation.map(creation_to_dto),
            handoff: state.handoff.map(|handoff| CanisterPoolHandoff {
                canister_id: handoff.canister_id,
                recipient: handoff.recipient,
                prepared_at_ns: handoff.prepared_at_ns,
            }),
        })
    }
}
