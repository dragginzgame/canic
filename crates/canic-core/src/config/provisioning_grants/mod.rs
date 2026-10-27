//! Module: config::provisioning_grants
//!
//! Responsibility: check directed provisioning-grant cycles for config and topology.
//! Does not own: grant limits, self-grant admission, diagnostics, or canonical encoding.

use canic_contracts::ids::ComponentSpecId;
use std::collections::{BTreeMap, VecDeque};

/// Borrowed graph of the Specs admitted at the calling boundary.
pub(super) struct ProvisioningGrantGraph<'a> {
    nodes: BTreeMap<&'a ComponentSpecId, GrantNode<'a>>,
}

struct GrantNode<'a> {
    incoming: u32,
    outgoing: Vec<&'a ComponentSpecId>,
}

/// Edge-admission failures mapped to each boundary's existing diagnostics.
#[derive(Debug)]
pub(super) enum ProvisioningGrantGraphError {
    IncomingCountOverflow,

    UnknownTarget,
}

impl<'a> ProvisioningGrantGraph<'a> {
    pub(super) fn new(specs: impl IntoIterator<Item = &'a ComponentSpecId>) -> Self {
        Self {
            nodes: specs
                .into_iter()
                .map(|spec| {
                    (
                        spec,
                        GrantNode {
                            incoming: 0,
                            outgoing: Vec::new(),
                        },
                    )
                })
                .collect(),
        }
    }

    /// Incoming grants from unadmitted requesters remain authority in a projected
    /// topology, but cannot participate in its internal cycles.
    pub(super) fn add_grant(
        &mut self,
        requester: &'a ComponentSpecId,
        target: &'a ComponentSpecId,
    ) -> Result<(), ProvisioningGrantGraphError> {
        if !self.nodes.contains_key(requester) {
            return Ok(());
        }
        let target_node = self
            .nodes
            .get_mut(target)
            .ok_or(ProvisioningGrantGraphError::UnknownTarget)?;
        target_node.incoming = target_node
            .incoming
            .checked_add(1)
            .ok_or(ProvisioningGrantGraphError::IncomingCountOverflow)?;
        self.nodes
            .get_mut(requester)
            .expect("requester membership was checked above")
            .outgoing
            .push(target);
        Ok(())
    }

    /// Return the first canonical Spec blocked by a cycle, matching each
    /// boundary's deterministic diagnostic even when it is downstream of a cycle.
    pub(super) fn cycle_blocked_spec(mut self) -> Option<&'a ComponentSpecId> {
        let mut ready = self
            .nodes
            .iter()
            .filter(|(_, node)| node.incoming == 0)
            .map(|(spec, _)| *spec)
            .collect::<VecDeque<_>>();
        while let Some(requester) = ready.pop_front() {
            let targets = std::mem::take(
                &mut self
                    .nodes
                    .get_mut(requester)
                    .expect("ready Specs belong to the graph")
                    .outgoing,
            );
            for target in targets {
                let node = self
                    .nodes
                    .get_mut(target)
                    .expect("grant targets were admitted before cycle detection");
                node.incoming -= 1;
                if node.incoming == 0 {
                    ready.push_back(target);
                }
            }
        }
        self.nodes
            .into_iter()
            .find(|(_, node)| node.incoming > 0)
            .map(|(spec, _)| spec)
    }
}
