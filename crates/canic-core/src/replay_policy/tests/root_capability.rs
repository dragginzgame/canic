use super::*;
use std::collections::BTreeSet;

#[test]
fn root_capability_command_variants_have_replay_policy_entries() {
    let variants = root_capability_command_variant_names();
    let manifest = ROOT_CAPABILITY_COMMAND_REPLAY_POLICY_MANIFEST
        .iter()
        .map(|entry| entry.variant)
        .collect::<BTreeSet<_>>();

    assert_eq!(manifest, variants);
}

#[test]
fn root_capability_endpoint_is_manifested_as_command_dispatch() {
    let entry = ENDPOINT_REPLAY_POLICY_MANIFEST
        .iter()
        .find(|entry| entry.endpoint == "canic_response_capability_v1")
        .expect("root capability endpoint policy entry");

    assert_eq!(
        entry.implementation_status,
        ReplayImplementationStatus::Implemented
    );
    assert_eq!(
        entry.replay_policy,
        ReplayPolicy::CommandDispatch {
            command_kind: "root.capability_rpc.v1",
            command_manifest: "root.capability.command_manifest.v1",
        }
    );
    assert_eq!(entry.cost_class, CostClass::ManagementDeployment);
    assert_eq!(entry.quota_policy, Some(DEPLOYMENT_QUOTA_V1));
    assert_eq!(entry.cycle_reserve_policy, Some(DEPLOYMENT_RESERVE_V1));
}

#[test]
fn root_capability_command_blockers_are_explicit() {
    let blockers = ROOT_CAPABILITY_COMMAND_REPLAY_POLICY_MANIFEST
        .iter()
        .filter(|entry| entry.implementation_status == ReplayImplementationStatus::ReleaseBlocker)
        .map(|entry| entry.variant)
        .collect::<BTreeSet<_>>();

    assert!(blockers.is_empty(), "unexpected blockers: {blockers:?}");
}

#[test]
fn root_capability_implemented_commands_are_replay_protected() {
    for (variant, command_kind, cost_class) in [
        (
            "ProvisionCanister",
            "root.provision.v1",
            CostClass::ManagementDeployment,
        ),
        (
            "UpgradeCanister",
            "root.upgrade.v1",
            CostClass::ManagementDeployment,
        ),
        (
            "RecycleCanister",
            "root.recycle_canister.v1",
            CostClass::ManagementDeployment,
        ),
        (
            "RequestCycles",
            "root.request_cycles.v1",
            CostClass::ValueTransfer,
        ),
    ] {
        let entry = ROOT_CAPABILITY_COMMAND_REPLAY_POLICY_MANIFEST
            .iter()
            .find(|entry| entry.variant == variant)
            .expect("root capability command policy entry");

        assert_eq!(
            entry.implementation_status,
            ReplayImplementationStatus::Implemented
        );
        assert_eq!(
            entry.replay_policy,
            ReplayPolicy::ReplayProtected {
                command_kind,
                requires_operation_id: true,
            }
        );
        assert_eq!(entry.cost_class, cost_class);
    }
}
