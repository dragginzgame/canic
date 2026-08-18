//! Module: replay_policy::tests::cost
//!
//! Responsibility: verify costed replay-policy entries declare guard labels.
//! Does not own: cost policy enforcement or manifest construction.
//! Boundary: test-only checks over manifest rows.

use super::*;

#[test]
fn costed_manifest_entries_declare_guards() {
    for entry in ENDPOINT_REPLAY_POLICY_MANIFEST
        .iter()
        .chain(STORE_ENDPOINT_REPLAY_POLICY_MANIFEST)
    {
        if entry.cost_class == CostClass::None {
            continue;
        }
        assert!(
            entry.quota_policy.is_some(),
            "costed entry {} missing quota policy",
            entry.endpoint
        );
        assert!(
            entry.cost_class == CostClass::RootCanisterSignaturePrepare
                || entry.cost_class == CostClass::RootChainKeySigning
                || entry.cost_class == CostClass::IssuerCanisterSignaturePrepare
                || entry.cycle_reserve_policy.is_some(),
            "costed entry {} missing cycle-reserve policy",
            entry.endpoint
        );
    }
}

#[test]
fn costed_root_capability_command_entries_declare_guards() {
    for entry in ROOT_CAPABILITY_COMMAND_REPLAY_POLICY_MANIFEST {
        if entry.cost_class == CostClass::None {
            continue;
        }
        assert!(
            entry.quota_policy.is_some(),
            "costed root capability command {} missing quota policy",
            entry.variant
        );
        assert!(
            entry.cycle_reserve_policy.is_some(),
            "costed root capability command {} missing cycle-reserve policy",
            entry.variant
        );
    }
}

#[test]
fn costed_role_command_entries_declare_guards() {
    for entry in ROOT_COMMAND_REPLAY_POLICY_MANIFEST
        .iter()
        .chain(COORDINATOR_COMMAND_REPLAY_POLICY_MANIFEST)
        .chain(MANAGED_COMMAND_REPLAY_POLICY_MANIFEST)
        .chain(STORE_COMMAND_REPLAY_POLICY_MANIFEST)
    {
        if entry.cost_class == CostClass::None {
            continue;
        }
        assert!(
            entry.quota_policy.is_some(),
            "costed role command {} missing quota policy",
            entry.variant
        );
        assert!(
            entry.cost_class == CostClass::RootCanisterSignaturePrepare
                || entry.cost_class == CostClass::RootChainKeySigning
                || entry.cost_class == CostClass::IssuerCanisterSignaturePrepare
                || entry.cycle_reserve_policy.is_some(),
            "costed role command {} missing cycle-reserve policy",
            entry.variant
        );
    }
}

#[test]
fn root_release_publication_is_the_role_owned_durable_publish_command() {
    let entry = ROOT_COMMAND_REPLAY_POLICY_MANIFEST
        .iter()
        .find(|entry| entry.variant == "PublishReleaseSet")
        .expect("Root release-publication command entry");

    assert_eq!(entry.cost_class, CostClass::DurablePublish);
    assert!(matches!(
        entry.replay_policy,
        ReplayPolicy::MonotonicTransition { .. }
    ));
    assert_eq!(entry.quota_policy, Some(DURABLE_PUBLISH_QUOTA_V1));
    assert_eq!(entry.cycle_reserve_policy, Some(DURABLE_PUBLISH_RESERVE_V1));
}

#[test]
fn store_deletion_cycle_reclamation_is_guarded_and_convergent() {
    let entry = STORE_COMMAND_REPLAY_POLICY_MANIFEST
        .iter()
        .find(|entry| entry.variant == "ReclaimDeletionCycles")
        .expect("Store deletion cycle-reclamation variant");

    assert!(matches!(
        entry.replay_policy,
        ReplayPolicy::SnapshotConvergent { command_kind }
            if command_kind.as_str() == "wasm_store.reclaim_deletion_cycles.v1"
    ));
    assert_eq!(entry.cost_class, CostClass::ValueTransfer);
    assert_eq!(entry.quota_policy, Some(VALUE_TRANSFER_QUOTA_V1));
    assert_eq!(entry.cycle_reserve_policy, Some(VALUE_TRANSFER_RESERVE_V1));
}
