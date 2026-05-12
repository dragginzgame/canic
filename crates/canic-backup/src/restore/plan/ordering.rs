use super::{
    RestoreOrderingDependency, RestoreOrderingRelationship, RestoreOrderingSummary,
    RestorePlanError, RestorePlanMember,
};
use std::collections::BTreeSet;

pub(super) fn order_members(
    members: Vec<RestorePlanMember>,
) -> Result<Vec<RestorePlanMember>, RestorePlanError> {
    let mut remaining = members;
    let group_sources = remaining
        .iter()
        .map(|member| member.source_canister.clone())
        .collect::<BTreeSet<_>>();
    let mut emitted = BTreeSet::new();
    let mut ordered = Vec::with_capacity(remaining.len());

    while !remaining.is_empty() {
        let Some(index) = remaining
            .iter()
            .position(|member| parent_satisfied(member, &group_sources, &emitted))
        else {
            return Err(RestorePlanError::RestoreOrderCycle);
        };

        let mut member = remaining.remove(index);
        member.member_order = ordered.len();
        member.ordering_dependency = ordering_dependency(&member);
        emitted.insert(member.source_canister.clone());
        ordered.push(member);
    }

    Ok(ordered)
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

fn parent_satisfied(
    member: &RestorePlanMember,
    group_sources: &BTreeSet<String>,
    emitted: &BTreeSet<String>,
) -> bool {
    match &member.parent_source_canister {
        Some(parent) if group_sources.contains(parent) => emitted.contains(parent),
        _ => true,
    }
}
