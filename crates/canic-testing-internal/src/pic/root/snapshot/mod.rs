//! Pure snapshot inventory selection shared by Root fixtures and native tests.

use super::RootBaselineMetadata;
use ic_testkit::pic::CanisterSnapshotTarget;
use std::collections::BTreeMap;

pub(super) fn root_baseline_snapshot_targets(
    metadata: &RootBaselineMetadata,
) -> Vec<CanisterSnapshotTarget> {
    let mut senders = BTreeMap::from([(metadata.root_id, None), (metadata.coordinator_id, None)]);
    for canister_id in metadata
        .snapshot_pids
        .iter()
        .chain(&metadata.managed_store_pids)
    {
        senders
            .entry(*canister_id)
            .or_insert(Some(metadata.root_id));
    }
    senders
        .into_iter()
        .map(|(canister_id, sender)| CanisterSnapshotTarget {
            canister_id,
            sender,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn root_baseline_snapshot_targets_are_unique_and_controller_exact() {
        let root_id = ic_platform::principal::ExternalPrincipal::from_slice(&[0x41; 29]);
        let child_id = ic_platform::principal::ExternalPrincipal::from_slice(&[0x42; 29]);
        let store_id = ic_platform::principal::ExternalPrincipal::from_slice(&[0x43; 29]);
        let metadata = RootBaselineMetadata {
            root_id,
            coordinator_id: ic_platform::principal::ExternalPrincipal::from_slice(&[0x44; 29]),
            pool_assets: vec![
                ic_platform::principal::ExternalPrincipal::from_slice(&[0x45; 29]),
                child_id,
            ],
            component_canisters: HashMap::new(),
            snapshot_pids: vec![child_id, store_id],
            managed_store_pids: vec![store_id, root_id],
        };

        let targets = root_baseline_snapshot_targets(&metadata);

        assert_eq!(targets.len(), 4);
        assert_eq!(targets[0].canister_id, root_id);
        assert_eq!(targets[0].sender, None);
        assert_eq!(targets[1].canister_id, child_id);
        assert_eq!(targets[1].sender, Some(root_id));
        assert_eq!(targets[2].canister_id, store_id);
        assert_eq!(targets[2].sender, Some(root_id));
        assert_eq!(targets[3].canister_id, metadata.coordinator_id);
        assert_eq!(targets[3].sender, None);
        assert!(
            !targets
                .iter()
                .any(|target| target.canister_id == metadata.pool_assets[0])
        );
    }
}
