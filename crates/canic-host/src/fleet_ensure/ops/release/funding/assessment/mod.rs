//! Project exact collected DTOs into passive funding facts; policy owns their interpretation.

use crate::fleet_ensure::{
    ops::release::funding::{ReleaseFundingError, ReleaseFundingStage},
    view::release::{
        FleetReleaseFundingView,
        funding::{RecordedRefillCycles, ReleaseRefillFacts, ReleaseRootFundingFacts},
    },
};
use std::collections::BTreeSet;

pub(in crate::fleet_ensure) fn assessment_facts(
    evidence: &FleetReleaseFundingView,
) -> Result<Vec<ReleaseRootFundingFacts>, ReleaseFundingError> {
    evidence
        .roots
        .iter()
        .map(|root| {
            let header = root.pages.first().ok_or(ReleaseFundingError::Observation {
                root: root.root,
                stage: ReleaseFundingStage::Binding,
            })?;
            let coordinator = evidence
                .coordinator
                .roots
                .iter()
                .find(|entry| entry.fleet_subnet_root == root.root)
                .ok_or(ReleaseFundingError::Observation {
                    root: root.root,
                    stage: ReleaseFundingStage::Binding,
                })?;
            let coordinator_operations = header
                .current_request
                .as_ref()
                .map(|r| r.operation_id)
                .into_iter()
                .chain(
                    header
                        .accepted_grant
                        .as_ref()
                        .map(|r| r.request.operation_id),
                )
                .chain(
                    coordinator
                        .current_operation
                        .as_ref()
                        .map(|r| r.operation_id),
                )
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect();
            Ok(ReleaseRootFundingFacts {
                root: root.root,
                coordinator_operations,
                rotation_operation: header.rotation_current.as_ref().map(|r| r.operation_id),
                refills: root
                    .pages
                    .iter()
                    .flat_map(|page| &page.icp_refills)
                    .map(|entry| ReleaseRefillFacts {
                        transfer_uncertain: entry.transfer_uncertain,
                        record_id: entry.record_id,
                        operation_id: entry.response.operation_id,
                        status: entry.response.status,
                        error_code: entry.response.error_code,
                        ledger_block_index: entry.response.ledger_block_index,
                        cycles: entry.response.cycles_sent.as_ref().map_or(
                            RecordedRefillCycles::Missing,
                            |value| {
                                u128::try_from(value.0.clone()).map_or(
                                    RecordedRefillCycles::Overflow,
                                    RecordedRefillCycles::Recorded,
                                )
                            },
                        ),
                        refund_block_index: entry.refund_block_index,
                        expired_before_block: entry.transaction_too_old_min_block_index,
                    })
                    .collect(),
            })
        })
        .collect()
}
