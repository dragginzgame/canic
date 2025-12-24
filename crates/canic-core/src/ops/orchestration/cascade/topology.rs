//! Topology synchronization helpers.
//!
//! Propagates topology snapshots along a targeted branch.
//!
//! Bundles carry a trimmed parent chain and the per-node direct children for
//! that chain only. Each hop receives a suffix of the chain (starting at self),
//! validates it (cycle/root termination checks), imports its direct children,
//! and forwards a further-trimmed bundle to the next hop. Failures are logged
//! and abort the cascade rather than continuing with partial data.

use super::warn_if_large;
use crate::{
    Error,
    ids::CanisterRole,
    log::Topic,
    ops::{
        OpsError,
        orchestration::cascade::CascadeOpsError,
        prelude::*,
        storage::{
            CanisterSummary,
            topology::subnet::{SubnetCanisterChildrenOps, SubnetCanisterRegistryOps},
        },
    },
};
use std::collections::{HashMap, HashSet};

///
/// TopologyBundle
///

#[derive(CandidType, Clone, Debug, Default, Deserialize)]
pub struct TopologyBundle {
    pub parents: Vec<CanisterSummary>,
    pub children_map: HashMap<Principal, Vec<CanisterSummary>>,
}

impl TopologyBundle {
    pub fn for_target(target_pid: Principal) -> Result<Self, Error> {
        let parents = parent_chain(target_pid)?;

        let mut children_map = HashMap::new();

        // Add children of every ancestor in the chain (root → ... → target)
        for parent in &parents {
            let children = SubnetCanisterRegistryOps::children(parent.pid);
            children_map.insert(parent.pid, children);
        }

        Ok(Self {
            parents,
            children_map,
        })
    }
}

//
// ===========================================================================
//  ROOT CASCADES
// ===========================================================================
//

pub(crate) async fn root_cascade_topology_for_pid(target_pid: Principal) -> Result<(), Error> {
    OpsError::require_root()?;

    let bundle = match TopologyBundle::for_target(target_pid) {
        Ok(bundle) => bundle,
        Err(err) => {
            log!(
                Topic::Sync,
                Error,
                "sync.topology: failed to build bundle for target {target_pid}: {err}"
            );
            return Err(err);
        }
    };
    let root_pid = canister_self();
    let Some(first_child) = (match next_child_on_path(root_pid, &bundle.parents) {
        Ok(next) => next,
        Err(err) => {
            log!(
                Topic::Sync,
                Error,
                "sync.topology: invalid parent chain for {target_pid}: {err}"
            );
            return Err(err);
        }
    }) else {
        log!(
            Topic::Sync,
            Warn,
            "sync.topology: no branch path to {target_pid}, skipping targeted cascade"
        );

        return Ok(());
    };

    let child_bundle = match slice_bundle_for_child(first_child, &bundle) {
        Ok(b) => b,
        Err(err) => {
            log!(
                Topic::Sync,
                Error,
                "sync.topology: failed to slice bundle for child {first_child}: {err}"
            );
            return Err(err);
        }
    };
    if let Err(err) = send_bundle(&first_child, &child_bundle).await {
        log!(
            Topic::Sync,
            Warn,
            "sync.topology: failed targeted cascade to first child {first_child}: {err}"
        );
    } else {
        log!(
            Topic::Sync,
            Info,
            "sync.topology: delegated targeted cascade to {first_child}"
        );
    }

    Ok(())
}

//
// ===========================================================================
//  NON-ROOT CASCADES
// ===========================================================================
//

pub async fn nonroot_cascade_topology(bundle: &TopologyBundle) -> Result<(), Error> {
    OpsError::deny_root()?;
    let self_pid = canister_self();

    let next = match next_child_on_path(self_pid, &bundle.parents) {
        Ok(next) => next,
        Err(err) => {
            log!(
                Topic::Sync,
                Error,
                "sync.topology: rejecting bundle for {self_pid} (invalid parent chain len={}): {err}",
                bundle.parents.len()
            );
            return Err(err);
        }
    };

    let children = bundle
        .children_map
        .get(&self_pid)
        .cloned()
        .unwrap_or_default();
    warn_if_large("nonroot fanout", children.len());
    SubnetCanisterChildrenOps::import(children);

    if let Some(next_pid) = next {
        let next_bundle = match slice_bundle_for_child(next_pid, bundle) {
            Ok(b) => b,
            Err(err) => {
                log!(
                    Topic::Sync,
                    Error,
                    "sync.topology: failed to slice bundle for child {next_pid}: {err}"
                );
                return Err(err);
            }
        };
        send_bundle(&next_pid, &next_bundle).await?;
    }

    Ok(())
}

//
// ===========================================================================
//  HELPERS
// ===========================================================================
//

fn parent_chain(mut pid: Principal) -> Result<Vec<CanisterSummary>, Error> {
    let registry_len = SubnetCanisterRegistryOps::export().len();
    let mut chain = Vec::new();
    let mut seen: HashSet<Principal> = HashSet::new();

    loop {
        if !seen.insert(pid) {
            return Err(CascadeOpsError::ParentChainCycle(pid).into());
        }

        let Some(entry) = SubnetCanisterRegistryOps::get(pid) else {
            return Err(CascadeOpsError::CanisterNotFound(pid).into());
        };

        if seen.len() > registry_len {
            return Err(CascadeOpsError::ParentChainTooLong(seen.len()).into());
        }

        chain.push(entry.clone().into());

        let Some(parent) = entry.parent_pid else {
            if entry.role != CanisterRole::ROOT {
                return Err(CascadeOpsError::ParentChainNotRootTerminated(pid).into());
            }
            break;
        };
        pid = parent;
    }

    chain.reverse();

    Ok(chain)
}

async fn send_bundle(pid: &Principal, bundle: &TopologyBundle) -> Result<(), Error> {
    call_and_decode::<Result<(), Error>>(
        *pid,
        crate::ops::rpc::methods::CANIC_SYNC_TOPOLOGY,
        bundle,
    )
    .await?
}

//
// ===========================================================================
//  PATH HELPERS
// ===========================================================================
//

fn next_child_on_path(
    self_pid: Principal,
    parents: &[CanisterSummary],
) -> Result<Option<Principal>, Error> {
    let Some(first) = parents.first() else {
        return Err(CascadeOpsError::InvalidParentChain.into());
    };

    if first.pid != self_pid {
        return Err(CascadeOpsError::ParentChainMissingSelf(self_pid).into());
    }

    Ok(parents.get(1).map(|p| p.pid))
}

fn slice_bundle_for_child(
    next_pid: Principal,
    bundle: &TopologyBundle,
) -> Result<TopologyBundle, Error> {
    // Slice parents chain so we start at the next hop
    let mut sliced_parents = Vec::new();
    let mut include = false;

    for parent in &bundle.parents {
        if parent.pid == next_pid {
            include = true;
        }
        if include {
            sliced_parents.push(parent.clone());
        }
    }

    if sliced_parents.is_empty() {
        return Err(CascadeOpsError::NextHopNotFound(next_pid).into());
    }

    // Slice children_map so it includes only nodes in the sliced chain
    let mut sliced_children_map = HashMap::new();
    for parent in &sliced_parents {
        let children = bundle
            .children_map
            .get(&parent.pid)
            .cloned()
            .unwrap_or_default();
        sliced_children_map.insert(parent.pid, children);
    }

    Ok(TopologyBundle {
        parents: sliced_parents,
        children_map: sliced_children_map,
    })
}

//
// ===========================================================================
//  TESTS
// ===========================================================================
//

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        cdk::utils::time::now_secs,
        ids::CanisterRole,
        model::memory::{CanisterEntry, topology::SubnetCanisterRegistry},
    };

    fn p(id: u8) -> Principal {
        Principal::from_slice(&[id; 29])
    }

    fn s(pid: Principal, parent_pid: Option<Principal>) -> CanisterSummary {
        CanisterSummary {
            pid,
            role: CanisterRole::new("test"),
            parent_pid,
        }
    }

    #[test]
    fn next_child_from_chain() {
        let root = p(1);
        let hub = p(2);
        let inst = p(3);
        let ledg = p(4);

        let parents = vec![
            s(root, None),
            s(hub, Some(root)),
            s(inst, Some(hub)),
            s(ledg, Some(inst)),
        ];

        assert_eq!(next_child_on_path(root, &parents).unwrap(), Some(hub));
        assert_eq!(next_child_on_path(hub, &parents[1..]).unwrap(), Some(inst));
        assert_eq!(next_child_on_path(ledg, &parents[3..]).unwrap(), None);
    }

    #[test]
    fn slice_bundle_trims_prefix_children() {
        let root = p(1);
        let hub = p(2);
        let inst = p(3);

        let parents = vec![s(root, None), s(hub, Some(root)), s(inst, Some(hub))];
        let mut children_map = HashMap::new();
        children_map.insert(root, vec![s(hub, Some(root))]);
        children_map.insert(hub, vec![s(inst, Some(hub))]);
        children_map.insert(inst, vec![]);

        let bundle = TopologyBundle {
            parents,
            children_map,
        };

        let sliced = slice_bundle_for_child(hub, &bundle).unwrap();

        assert!(!sliced.children_map.contains_key(&root));
        assert!(sliced.children_map.contains_key(&hub));
        assert!(sliced.children_map.contains_key(&inst));
    }

    #[test]
    fn next_child_errors_when_missing_self() {
        let root = p(1);
        let hub = p(2);

        let parents = vec![s(root, None), s(hub, Some(root))];
        assert!(next_child_on_path(p(42), &parents).is_err());
    }

    #[test]
    fn parent_chain_rejects_cycle() {
        SubnetCanisterRegistry::clear_for_tests();
        let root = p(1);
        let hub = p(2);

        SubnetCanisterRegistryOps::register_root(root);
        SubnetCanisterRegistryOps::register(hub, &CanisterRole::new("hub"), hub, vec![]);

        let err = TopologyBundle::for_target(hub).unwrap_err();
        assert!(err.to_string().contains("cycle"));
    }

    #[test]
    fn parent_chain_requires_root_termination() {
        SubnetCanisterRegistry::clear_for_tests();
        let orphan = p(9);
        let role = CanisterRole::new("orphan");

        SubnetCanisterRegistry::insert_for_tests(CanisterEntry {
            pid: orphan,
            role,
            parent_pid: None,
            module_hash: None,
            created_at: now_secs(),
        });

        let err = TopologyBundle::for_target(orphan).unwrap_err();
        assert!(
            err.to_string()
                .contains("parent chain did not terminate at root")
        );
    }
}
