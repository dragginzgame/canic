//! Module: replay_policy::tests::cost
//!
//! Responsibility: verify costed replay-policy entries declare guard labels.
//! Does not own: cost policy enforcement or manifest construction.
//! Boundary: test-only checks over manifest rows.

use super::*;
use std::collections::BTreeSet;

#[test]
fn costed_manifest_entries_declare_guards() {
    for entry in ENDPOINT_REPLAY_POLICY_MANIFEST {
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
fn costed_pool_admin_command_entries_declare_guards() {
    for entry in POOL_ADMIN_COMMAND_REPLAY_POLICY_MANIFEST {
        if entry.cost_class == CostClass::None {
            continue;
        }
        assert!(
            entry.quota_policy.is_some(),
            "costed pool admin command {} missing quota policy",
            entry.variant
        );
        assert!(
            entry.cycle_reserve_policy.is_some(),
            "costed pool admin command {} missing cycle-reserve policy",
            entry.variant
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
fn durable_publish_entries_are_wasm_store_publication_surfaces() {
    let expected = durable_publish_endpoint_names();
    let actual = ENDPOINT_REPLAY_POLICY_MANIFEST
        .iter()
        .filter(|entry| entry.cost_class == CostClass::DurablePublish)
        .map(|entry| entry.endpoint)
        .collect::<BTreeSet<_>>();

    assert_eq!(
        actual, expected,
        "durable-publish cost class must stay scoped to wasm-store publication surfaces"
    );

    for endpoint in expected {
        let entry = ENDPOINT_REPLAY_POLICY_MANIFEST
            .iter()
            .find(|entry| entry.endpoint == endpoint)
            .expect("durable publish endpoint entry");

        assert_eq!(
            entry.implementation_status,
            ReplayImplementationStatus::Implemented
        );
        assert_eq!(entry.endpoint_kind, EndpointKind::Update);
        assert!(
            matches!(
                entry.replay_policy,
                ReplayPolicy::MonotonicTransition { .. }
            ),
            "{endpoint} must stay classified as monotonic publication"
        );
        assert_eq!(entry.quota_policy, Some(DURABLE_PUBLISH_QUOTA_V1));
        assert_eq!(entry.cycle_reserve_policy, Some(DURABLE_PUBLISH_RESERVE_V1));
    }
}
