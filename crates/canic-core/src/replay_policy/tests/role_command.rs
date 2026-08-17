//! Module: replay_policy::tests::role_command
//!
//! Responsibility: verify exact Root and Coordinator command-variant replay ownership.
//! Does not own: endpoint dispatch or workflow implementation.
//! Boundary: source-declared closed command unions must match their policy manifests.

use super::*;

#[test]
fn root_command_variants_have_one_exact_replay_policy_each() {
    assert_command_manifest_matches(
        ROOT_COMMAND_REPLAY_POLICY_MANIFEST,
        root_command_variant_names(),
        "Root",
    );
}

#[test]
fn coordinator_command_variants_have_one_exact_replay_policy_each() {
    assert_command_manifest_matches(
        COORDINATOR_COMMAND_REPLAY_POLICY_MANIFEST,
        coordinator_command_variant_names(),
        "Coordinator",
    );
}

#[test]
fn managed_command_variants_have_one_exact_replay_policy_each() {
    assert_command_manifest_matches(
        MANAGED_COMMAND_REPLAY_POLICY_MANIFEST,
        managed_command_variant_names(),
        "managed",
    );
}

#[test]
fn store_command_variants_have_one_exact_replay_policy_each() {
    assert_command_manifest_matches(
        STORE_COMMAND_REPLAY_POLICY_MANIFEST,
        store_command_variant_names(),
        "Store",
    );
}

#[test]
fn asynchronous_root_intents_are_replay_protected_by_operation_id() {
    for variant in [
        "AdoptStore",
        "BootstrapStore",
        "ProvisionChild",
        "ProvisionComponent",
        "ProvisionPeer",
        "RefillCycles",
        "RemoveComponent",
        "RemoveRoot",
        "RemoveSubtree",
        "SynchronizeRegistry",
    ] {
        let entry = command_entry(ROOT_COMMAND_REPLAY_POLICY_MANIFEST, variant);
        assert!(
            matches!(
                entry.replay_policy,
                ReplayPolicy::ReplayProtected {
                    requires_operation_id: true,
                    ..
                }
            ),
            "Root command {variant} must own replay through its operation ID"
        );
    }
}

#[test]
fn query_like_root_commands_do_not_acquire_mutation_replay_policy() {
    for variant in ["InspectCanister", "PreviewCycleRefill"] {
        assert_eq!(
            command_entry(ROOT_COMMAND_REPLAY_POLICY_MANIFEST, variant).replay_policy,
            ReplayPolicy::QueryOrReadOnly
        );
    }
}

#[test]
fn root_capability_variant_delegates_to_its_nested_command_manifest() {
    let entry = command_entry(ROOT_COMMAND_REPLAY_POLICY_MANIFEST, "RespondCapability");
    assert_eq!(
        entry.replay_policy,
        ReplayPolicy::CommandDispatch {
            command_kind: replay_command_kind("root.capability_rpc.v1"),
            command_manifest: replay_command_manifest("root.capability.command_manifest.v1"),
        }
    );
}

#[test]
fn coordinator_long_running_intents_are_operation_replay_protected() {
    for variant in ["ProvisionComponents", "RemoveRoot"] {
        let entry = command_entry(COORDINATOR_COMMAND_REPLAY_POLICY_MANIFEST, variant);
        assert!(matches!(
            entry.replay_policy,
            ReplayPolicy::ReplayProtected {
                requires_operation_id: true,
                ..
            }
        ));
    }
}

#[test]
fn managed_auth_effects_preserve_their_existing_replay_contracts() {
    let install = command_entry(
        MANAGED_COMMAND_REPLAY_POLICY_MANIFEST,
        "InstallDelegationProof",
    );
    assert!(matches!(
        install.replay_policy,
        ReplayPolicy::IntentionallyNonIdempotent { .. }
    ));

    let prepare = command_entry(
        MANAGED_COMMAND_REPLAY_POLICY_MANIFEST,
        "PrepareDelegatedToken",
    );
    assert!(matches!(
        prepare.replay_policy,
        ReplayPolicy::ReplayProtected {
            requires_operation_id: true,
            ..
        }
    ));
    assert_eq!(
        prepare.cost_class,
        CostClass::IssuerCanisterSignaturePrepare
    );
    assert_eq!(
        prepare.quota_policy,
        Some(ISSUER_CANISTER_SIGNATURE_PREPARE_QUOTA_V1)
    );
}

#[test]
fn store_effect_variants_preserve_cost_and_convergence() {
    let reclaim = command_entry(
        STORE_COMMAND_REPLAY_POLICY_MANIFEST,
        "ReclaimDeletionCycles",
    );
    assert!(matches!(
        reclaim.replay_policy,
        ReplayPolicy::SnapshotConvergent { .. }
    ));
    assert_eq!(reclaim.cost_class, CostClass::ValueTransfer);
    assert_eq!(reclaim.quota_policy, Some(VALUE_TRANSFER_QUOTA_V1));
    assert_eq!(
        reclaim.cycle_reserve_policy,
        Some(VALUE_TRANSFER_RESERVE_V1)
    );

    for variant in ["SynchronizeState", "SynchronizeTopology"] {
        assert!(matches!(
            command_entry(STORE_COMMAND_REPLAY_POLICY_MANIFEST, variant).replay_policy,
            ReplayPolicy::SnapshotConvergent { .. }
        ));
    }
}

fn command_entry(
    manifest: &'static [CommandReplayPolicy],
    variant: &str,
) -> &'static CommandReplayPolicy {
    manifest
        .iter()
        .find(|entry| entry.variant == variant)
        .unwrap_or_else(|| panic!("missing command replay policy for {variant}"))
}

fn assert_command_manifest_matches(
    manifest: &[CommandReplayPolicy],
    expected: BTreeSet<&'static str>,
    role: &str,
) {
    let mut actual = BTreeSet::new();
    for entry in manifest {
        assert!(
            actual.insert(entry.variant),
            "duplicate {role} command replay policy for {}",
            entry.variant
        );
    }
    assert_eq!(actual, expected, "{role} command replay-policy drift");
}
