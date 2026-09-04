//! Module: ops::cascade
//!
//! Responsibility: send state and topology cascade snapshots through RPC.
//! Does not own: cascade workflow decisions, snapshot construction, or endpoint auth.
//! Boundary: ops wrapper around the RPC transport for cascade message names.

use crate::{
    InternalError,
    dto::cascade::{StateSnapshotInput, TopologyPathNode, TopologySnapshotInput},
    ids::CanisterRole,
    ops::{prelude::*, rpc::RpcOps},
    protocol,
};
use candid::CandidType;
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use thiserror::Error as ThisError;

///
/// TopologySnapshotValidationError
///
/// Typed structural rejection for one received topology branch snapshot.
///

#[derive(Debug, Eq, PartialEq, ThisError)]
pub enum TopologySnapshotValidationError {
    #[error("topology parent chain is empty")]
    EmptyParentChain,

    #[error("topology parent chain begins at {found}, expected receiver {expected}")]
    ReceiverMismatch {
        expected: Principal,
        found: Principal,
    },

    #[error("topology receiver {pid} has role {found}, expected {expected}")]
    ReceiverRoleMismatch {
        pid: Principal,
        expected: CanisterRole,
        found: CanisterRole,
    },

    #[error("topology receiver {pid} records parent {found:?}, expected {expected}")]
    ImmediateParentMismatch {
        pid: Principal,
        expected: Principal,
        found: Option<Principal>,
    },

    #[error("topology parent chain repeats canister {0}")]
    DuplicatePathNode(Principal),

    #[error("topology parent chain for {pid} records parent {found:?}, expected {expected}")]
    BrokenParentLink {
        pid: Principal,
        expected: Principal,
        found: Option<Principal>,
    },

    #[error("topology children map repeats parent {0}")]
    DuplicateChildrenRow(Principal),

    #[error("topology children map is missing parent {0}")]
    MissingChildrenRow(Principal),

    #[error("topology children map contains parent {0} outside the branch")]
    UnexpectedChildrenRow(Principal),

    #[error("topology child list for parent {parent} repeats child {child}")]
    DuplicateChild { parent: Principal, child: Principal },

    #[error(
        "topology child {child} appears under both parent {first_parent} and parent {second_parent}"
    )]
    ConflictingChildParent {
        child: Principal,
        first_parent: Principal,
        second_parent: Principal,
    },

    #[error("topology parent {parent} lists itself as a child")]
    SelfChild { parent: Principal },

    #[error("topology next hop {child} is absent from parent {parent}'s direct-child list")]
    NextHopMissing { parent: Principal, child: Principal },

    #[error(
        "topology next hop {child} has role {found} in parent {parent}'s child list, expected {expected}"
    )]
    NextHopRoleMismatch {
        parent: Principal,
        child: Principal,
        expected: CanisterRole,
        found: CanisterRole,
    },
}

impl From<TopologySnapshotValidationError> for InternalError {
    fn from(err: TopologySnapshotValidationError) -> Self {
        use crate::diagnostics::codes;

        let code = match err {
            TopologySnapshotValidationError::EmptyParentChain => codes::SECURITY_INVALID,
            TopologySnapshotValidationError::ReceiverMismatch { .. }
            | TopologySnapshotValidationError::ReceiverRoleMismatch { .. }
            | TopologySnapshotValidationError::ImmediateParentMismatch { .. }
            | TopologySnapshotValidationError::BrokenParentLink { .. }
            | TopologySnapshotValidationError::ConflictingChildParent { .. }
            | TopologySnapshotValidationError::NextHopRoleMismatch { .. } => {
                codes::AUTHORITY_CONFLICT
            }
            TopologySnapshotValidationError::DuplicatePathNode(_)
            | TopologySnapshotValidationError::DuplicateChildrenRow(_)
            | TopologySnapshotValidationError::DuplicateChild { .. } => codes::POSITION_DUPLICATE,
            TopologySnapshotValidationError::MissingChildrenRow(_)
            | TopologySnapshotValidationError::NextHopMissing { .. } => {
                codes::COLLECTION_UNAVAILABLE
            }
            TopologySnapshotValidationError::UnexpectedChildrenRow(_) => codes::COLLECTION_INVALID,
            TopologySnapshotValidationError::SelfChild { .. } => codes::COLLECTION_INVALID_STATE,
        };
        Self::public(code)
    }
}

fn validate_topology_receiver(
    first: &TopologyPathNode,
    receiver: Principal,
    expected_parent: Principal,
    expected_role: &CanisterRole,
) -> Result<(), TopologySnapshotValidationError> {
    if first.pid != receiver {
        return Err(TopologySnapshotValidationError::ReceiverMismatch {
            expected: receiver,
            found: first.pid,
        });
    }
    if first.role != *expected_role {
        return Err(TopologySnapshotValidationError::ReceiverRoleMismatch {
            pid: receiver,
            expected: expected_role.clone(),
            found: first.role.clone(),
        });
    }
    if first.parent_pid != Some(expected_parent) {
        return Err(TopologySnapshotValidationError::ImmediateParentMismatch {
            pid: receiver,
            expected: expected_parent,
            found: first.parent_pid,
        });
    }
    Ok(())
}

///
/// CascadeOps
///
/// Operations-layer facade for cascade snapshot RPC sends.
///

pub struct CascadeOps;

#[derive(CandidType)]
enum StoreCommandFragment<'a> {
    SynchronizeState(&'a StateSnapshotInput),
    SynchronizeTopology(&'a TopologySnapshotInput),
}

#[derive(CandidType, Deserialize)]
enum StoreCommandResponseFragment {
    SynchronizeState,
    SynchronizeTopology,
}

impl CascadeOps {
    pub(crate) fn validate_topology_snapshot(
        snapshot: &TopologySnapshotInput,
        receiver: Principal,
        expected_parent: Principal,
        expected_role: &CanisterRole,
    ) -> Result<(), TopologySnapshotValidationError> {
        let first = snapshot
            .parents
            .first()
            .ok_or(TopologySnapshotValidationError::EmptyParentChain)?;
        validate_topology_receiver(first, receiver, expected_parent, expected_role)?;

        let mut path_pids = HashSet::with_capacity(snapshot.parents.len());
        for (index, node) in snapshot.parents.iter().enumerate() {
            if !path_pids.insert(node.pid) {
                return Err(TopologySnapshotValidationError::DuplicatePathNode(node.pid));
            }
            if let Some(previous) = index.checked_sub(1).map(|index| &snapshot.parents[index])
                && node.parent_pid != Some(previous.pid)
            {
                return Err(TopologySnapshotValidationError::BrokenParentLink {
                    pid: node.pid,
                    expected: previous.pid,
                    found: node.parent_pid,
                });
            }
        }

        let mut children_by_parent = HashMap::with_capacity(snapshot.children_map.len());
        let mut owner_by_child = HashMap::new();
        for row in &snapshot.children_map {
            if children_by_parent.insert(row.parent_pid, row).is_some() {
                return Err(TopologySnapshotValidationError::DuplicateChildrenRow(
                    row.parent_pid,
                ));
            }

            let mut children = HashSet::with_capacity(row.children.len());
            for child in &row.children {
                if child.pid == row.parent_pid {
                    return Err(TopologySnapshotValidationError::SelfChild {
                        parent: row.parent_pid,
                    });
                }
                if !children.insert(child.pid) {
                    return Err(TopologySnapshotValidationError::DuplicateChild {
                        parent: row.parent_pid,
                        child: child.pid,
                    });
                }
                if let Some(first_parent) = owner_by_child.insert(child.pid, row.parent_pid)
                    && first_parent != row.parent_pid
                {
                    return Err(TopologySnapshotValidationError::ConflictingChildParent {
                        child: child.pid,
                        first_parent,
                        second_parent: row.parent_pid,
                    });
                }
            }
        }

        for parent in &snapshot.parents {
            if !children_by_parent.contains_key(&parent.pid) {
                return Err(TopologySnapshotValidationError::MissingChildrenRow(
                    parent.pid,
                ));
            }
        }
        for parent in children_by_parent.keys() {
            if !path_pids.contains(parent) {
                return Err(TopologySnapshotValidationError::UnexpectedChildrenRow(
                    *parent,
                ));
            }
        }

        for pair in snapshot.parents.windows(2) {
            let parent = &pair[0];
            let child = &pair[1];
            let row = children_by_parent
                .get(&parent.pid)
                .expect("path rows were validated above");
            let Some(listed_child) = row.children.iter().find(|entry| entry.pid == child.pid)
            else {
                return Err(TopologySnapshotValidationError::NextHopMissing {
                    parent: parent.pid,
                    child: child.pid,
                });
            };
            if listed_child.role != child.role {
                return Err(TopologySnapshotValidationError::NextHopRoleMismatch {
                    parent: parent.pid,
                    child: child.pid,
                    expected: child.role.clone(),
                    found: listed_child.role.clone(),
                });
            }
        }

        Ok(())
    }

    pub async fn send_state_snapshot(
        pid: Principal,
        snapshot: &StateSnapshotInput,
    ) -> Result<(), InternalError> {
        let response: StoreCommandResponseFragment = RpcOps::call_rpc_result(
            pid,
            protocol::CANIC_WASM_STORE_COMMAND,
            StoreCommandFragment::SynchronizeState(snapshot),
        )
        .await?;
        match response {
            StoreCommandResponseFragment::SynchronizeState => Ok(()),
            StoreCommandResponseFragment::SynchronizeTopology => Err(InternalError::conflict()),
        }
    }

    pub async fn send_topology_snapshot(
        pid: Principal,
        snapshot: &TopologySnapshotInput,
    ) -> Result<(), InternalError> {
        let response: StoreCommandResponseFragment = RpcOps::call_rpc_result(
            pid,
            protocol::CANIC_WASM_STORE_COMMAND,
            StoreCommandFragment::SynchronizeTopology(snapshot),
        )
        .await?;
        match response {
            StoreCommandResponseFragment::SynchronizeTopology => Ok(()),
            StoreCommandResponseFragment::SynchronizeState => Err(InternalError::conflict()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{CascadeOps, TopologySnapshotValidationError};
    use crate::{
        cdk::types::Principal,
        dto::cascade::{
            TopologyChildren, TopologyDirectChild, TopologyPathNode, TopologySnapshotInput,
        },
        ids::CanisterRole,
    };

    fn p(byte: u8) -> Principal {
        Principal::from_slice(&[byte; 29])
    }

    fn role(name: &str) -> CanisterRole {
        CanisterRole::owned(name.to_string())
    }

    fn valid_branch() -> TopologySnapshotInput {
        TopologySnapshotInput {
            parents: vec![
                TopologyPathNode {
                    pid: p(2),
                    role: role("hub"),
                    parent_pid: Some(p(1)),
                },
                TopologyPathNode {
                    pid: p(3),
                    role: role("shard"),
                    parent_pid: Some(p(2)),
                },
            ],
            children_map: vec![
                TopologyChildren {
                    parent_pid: p(2),
                    children: vec![TopologyDirectChild {
                        pid: p(3),
                        role: role("shard"),
                    }],
                },
                TopologyChildren {
                    parent_pid: p(3),
                    children: Vec::new(),
                },
            ],
        }
    }

    fn validate(snapshot: &TopologySnapshotInput) -> Result<(), TopologySnapshotValidationError> {
        CascadeOps::validate_topology_snapshot(snapshot, p(2), p(1), &role("hub"))
    }

    #[test]
    fn topology_snapshot_validation_accepts_exact_branch_and_explicit_leaf_row() {
        validate(&valid_branch()).expect("exact topology branch");
    }

    #[test]
    fn topology_snapshot_validation_rejects_missing_duplicate_and_extra_rows() {
        let mut missing = valid_branch();
        missing.children_map.pop();
        assert_eq!(
            validate(&missing),
            Err(TopologySnapshotValidationError::MissingChildrenRow(p(3)))
        );

        let mut duplicate = valid_branch();
        duplicate
            .children_map
            .push(duplicate.children_map[0].clone());
        assert_eq!(
            validate(&duplicate),
            Err(TopologySnapshotValidationError::DuplicateChildrenRow(p(2)))
        );

        let mut extra = valid_branch();
        extra.children_map.push(TopologyChildren {
            parent_pid: p(9),
            children: Vec::new(),
        });
        assert_eq!(
            validate(&extra),
            Err(TopologySnapshotValidationError::UnexpectedChildrenRow(p(9)))
        );
    }

    #[test]
    fn topology_snapshot_validation_rejects_broken_path_and_child_evidence() {
        let mut broken_parent = valid_branch();
        broken_parent.parents[1].parent_pid = Some(p(9));
        assert_eq!(
            validate(&broken_parent),
            Err(TopologySnapshotValidationError::BrokenParentLink {
                pid: p(3),
                expected: p(2),
                found: Some(p(9)),
            })
        );

        let mut missing_next = valid_branch();
        missing_next.children_map[0].children.clear();
        assert_eq!(
            validate(&missing_next),
            Err(TopologySnapshotValidationError::NextHopMissing {
                parent: p(2),
                child: p(3),
            })
        );

        let mut wrong_role = valid_branch();
        wrong_role.children_map[0].children[0].role = role("other");
        assert_eq!(
            validate(&wrong_role),
            Err(TopologySnapshotValidationError::NextHopRoleMismatch {
                parent: p(2),
                child: p(3),
                expected: role("shard"),
                found: role("other"),
            })
        );
    }

    #[test]
    fn topology_snapshot_validation_rejects_wrong_receiver_identity() {
        let mut wrong_parent = valid_branch();
        wrong_parent.parents[0].parent_pid = Some(p(8));
        assert_eq!(
            validate(&wrong_parent),
            Err(TopologySnapshotValidationError::ImmediateParentMismatch {
                pid: p(2),
                expected: p(1),
                found: Some(p(8)),
            })
        );

        let mut wrong_role = valid_branch();
        wrong_role.parents[0].role = role("other");
        assert_eq!(
            validate(&wrong_role),
            Err(TopologySnapshotValidationError::ReceiverRoleMismatch {
                pid: p(2),
                expected: role("hub"),
                found: role("other"),
            })
        );
    }
}
