//! Module: restore::plan::ordering
//!
//! Responsibility: order restore members so parents are restored before children.
//! Does not own: member mapping, artifact validation, or command rendering.
//! Boundary: annotates restore ordering dependencies for dry-run and apply workflows.

use crate::restore::{
    RestoreOrderingDependency, RestoreOrderingRelationship, RestoreOrderingSummary,
    RestorePlanError, RestorePlanMember,
};

use ic_backup::model::effect_graph::{EffectGraphRecord, EffectNodeRecord, EffectNodeRequest};
use std::collections::BTreeMap;

pub(super) fn order_members(
    members: Vec<RestorePlanMember>,
) -> Result<Vec<RestorePlanMember>, RestorePlanError> {
    if members.is_empty() {
        return Ok(members);
    }
    let sources: BTreeMap<_, _> = members
        .iter()
        .enumerate()
        .map(|(index, member)| (member.source_canister.as_str(), index as u64))
        .collect();
    let nodes = members
        .iter()
        .enumerate()
        .map(|(index, member)| {
            EffectNodeRecord::new(EffectNodeRequest {
                operation_sequence: index as u64,
                depends_on: member
                    .parent_source_canister
                    .as_deref()
                    .and_then(|parent| sources.get(parent))
                    .copied()
                    .into_iter()
                    .collect(),
            })
        })
        .collect::<Result<_, _>>()?;
    let graph = EffectGraphRecord::new(nodes)?;
    let mut members: BTreeMap<_, _> = members
        .into_iter()
        .enumerate()
        .map(|(index, member)| (index as u64, member))
        .collect();
    Ok(graph
        .ordered_nodes()
        .enumerate()
        .map(|(order, node)| {
            let mut member = members
                .remove(&node.operation_sequence())
                .expect("graph names an original member exactly once");
            member.member_order = order;
            member.ordering_dependency = ordering_dependency(&member);
            member
        })
        .collect())
}

fn ordering_dependency(member: &RestorePlanMember) -> Option<RestoreOrderingDependency> {
    let parent_source = member.parent_source_canister.as_ref()?;
    let parent_target = member.parent_target_canister.as_ref()?;
    let relationship = RestoreOrderingRelationship::ParentBeforeChild;

    Some(RestoreOrderingDependency {
        source_canister: parent_source.clone(),
        target_canister: parent_target.clone(),
        relationship,
    })
}

pub(super) fn restore_ordering_summary(members: &[RestorePlanMember]) -> RestoreOrderingSummary {
    let mut summary = RestoreOrderingSummary {
        ordered_members: members.len(),
        dependency_free_members: 0,
        parent_edges: 0,
    };

    for member in members {
        if member.ordering_dependency.is_some() {
            summary.parent_edges += 1;
        } else {
            summary.dependency_free_members += 1;
        }
    }

    summary
}
