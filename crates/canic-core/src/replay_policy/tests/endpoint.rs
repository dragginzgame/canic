//! Module: replay_policy::tests::endpoint
//!
//! Responsibility: verify direct endpoint replay classifications.
//! Does not own: role-command variant policy or workflow replay behavior.
//! Boundary: focused assertions over the maintained endpoint manifest.

use super::*;
use crate::protocol::{CANIC_COMMAND, CANIC_COORDINATOR_COMMAND, CANIC_ROOT_COMMAND, CANIC_STATUS};

#[test]
fn common_role_command_dispatch_is_variant_manifest_owned() {
    let command = ENDPOINT_REPLAY_POLICY_MANIFEST
        .iter()
        .find(|entry| entry.endpoint == CANIC_COMMAND)
        .expect("common role command entry");
    assert_eq!(command.endpoint_kind, EndpointKind::Update);
    assert_eq!(
        command.replay_policy,
        ReplayPolicy::CommandDispatch {
            command_kind: replay_command_kind("role.command.v1"),
            command_manifest: replay_command_manifest("role.command.variant_manifest.v1"),
        }
    );
    assert_eq!(command.cost_class, CostClass::None);
    assert_eq!(command.quota_policy, None);
    assert_eq!(command.cycle_reserve_policy, None);

    for endpoint in [CANIC_COORDINATOR_COMMAND, CANIC_ROOT_COMMAND] {
        let role_command = ENDPOINT_REPLAY_POLICY_MANIFEST
            .iter()
            .find(|entry| entry.endpoint == endpoint)
            .expect("role-specific command entry");
        assert_eq!(role_command.endpoint_kind, EndpointKind::Update);
        assert!(matches!(
            role_command.replay_policy,
            ReplayPolicy::CommandDispatch { .. }
        ));
    }

    let status = ENDPOINT_REPLAY_POLICY_MANIFEST
        .iter()
        .find(|entry| entry.endpoint == CANIC_STATUS)
        .expect("common role status entry");
    assert_eq!(status.endpoint_kind, EndpointKind::Query);
    assert_eq!(status.replay_policy, ReplayPolicy::QueryOrReadOnly);
}

#[test]
fn store_byte_lanes_keep_their_direct_replay_contracts() {
    let chunk = STORE_ENDPOINT_REPLAY_POLICY_MANIFEST
        .iter()
        .find(|entry| entry.endpoint == "canic_wasm_store_chunk")
        .expect("Store chunk lane");
    assert_eq!(chunk.endpoint_kind, EndpointKind::Update);
    assert_eq!(chunk.replay_policy, ReplayPolicy::QueryOrReadOnly);

    let publish = STORE_ENDPOINT_REPLAY_POLICY_MANIFEST
        .iter()
        .find(|entry| entry.endpoint == "canic_wasm_store_publish_chunk")
        .expect("Store publish-chunk lane");
    assert_eq!(publish.endpoint_kind, EndpointKind::Update);
    assert!(matches!(
        publish.replay_policy,
        ReplayPolicy::MonotonicTransition { command_kind }
            if command_kind.as_str() == "wasm_store.publish_chunk.v1"
    ));
}
