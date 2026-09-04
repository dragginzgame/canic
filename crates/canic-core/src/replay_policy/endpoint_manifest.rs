//! Module: replay_policy::endpoint_manifest
//!
//! Responsibility: record replay policy for Canic-owned endpoint surfaces.
//! Does not own: endpoint routing, workflow execution, or replay receipt storage.
//! Boundary: common role methods dispatch to variant manifests; other methods remain direct.

use crate::{
    protocol::{
        CANIC_COMMAND, CANIC_COORDINATOR_COMMAND, CANIC_COORDINATOR_STATUS, CANIC_ROOT_COMMAND,
        CANIC_ROOT_STATUS, CANIC_STATUS, CANIC_WASM_STORE_COMMAND, CANIC_WASM_STORE_STATUS,
    },
    replay_policy::types::{
        CostClass, EndpointKind, EndpointReplayPolicy, ReplayCommandKindLabel,
        ReplayCommandManifestLabel, ReplayCycleReservePolicyLabel, ReplayImplementationStatus,
        ReplayPolicy, ReplayQuotaPolicyLabel,
    },
};

/// Canonical replay-policy rows for ordinary, Coordinator and Root endpoint methods.
pub const ENDPOINT_REPLAY_POLICY_MANIFEST: &[EndpointReplayPolicy] = &[
    update_command_dispatch(
        CANIC_COMMAND,
        command_kind("role.command.v1"),
        command_manifest("role.command.variant_manifest.v1"),
        ReplayImplementationStatus::Implemented,
        CostClass::None,
        None,
        None,
    ),
    update_command_dispatch(
        CANIC_COORDINATOR_COMMAND,
        command_kind("role.command.v1"),
        command_manifest("role.command.variant_manifest.v1"),
        ReplayImplementationStatus::Implemented,
        CostClass::None,
        None,
        None,
    ),
    query_read_only(CANIC_COORDINATOR_STATUS),
    update_command_dispatch(
        CANIC_ROOT_COMMAND,
        command_kind("role.command.v1"),
        command_manifest("role.command.variant_manifest.v1"),
        ReplayImplementationStatus::Implemented,
        CostClass::None,
        None,
        None,
    ),
    query_read_only(CANIC_ROOT_STATUS),
    query_read_only(CANIC_STATUS),
];

/// Exact replay-policy rows for the Store role, including its two data lanes.
pub const STORE_ENDPOINT_REPLAY_POLICY_MANIFEST: &[EndpointReplayPolicy] = &[
    update_command_dispatch(
        CANIC_WASM_STORE_COMMAND,
        command_kind("role.command.v1"),
        command_manifest("role.command.variant_manifest.v1"),
        ReplayImplementationStatus::Implemented,
        CostClass::None,
        None,
        None,
    ),
    query_read_only(CANIC_WASM_STORE_STATUS),
    update_read_only("canic_wasm_store_chunk"),
    update_monotonic_transition(
        "canic_wasm_store_publish_chunk",
        command_kind("wasm_store.publish_chunk.v1"),
    ),
];

/// Returns the canonical endpoint replay-policy manifest.
#[must_use]
pub const fn endpoint_replay_policy_manifest() -> &'static [EndpointReplayPolicy] {
    ENDPOINT_REPLAY_POLICY_MANIFEST
}

/// Returns the exact Store endpoint replay-policy manifest.
#[must_use]
pub const fn store_endpoint_replay_policy_manifest() -> &'static [EndpointReplayPolicy] {
    STORE_ENDPOINT_REPLAY_POLICY_MANIFEST
}

const fn command_kind(label: &'static str) -> ReplayCommandKindLabel {
    ReplayCommandKindLabel::new(label)
}

const fn command_manifest(label: &'static str) -> ReplayCommandManifestLabel {
    ReplayCommandManifestLabel::new(label)
}

const fn query_read_only(endpoint: &'static str) -> EndpointReplayPolicy {
    EndpointReplayPolicy {
        endpoint,
        endpoint_kind: EndpointKind::Query,
        replay_policy: ReplayPolicy::QueryOrReadOnly,
        implementation_status: ReplayImplementationStatus::Implemented,
        cost_class: CostClass::None,
        quota_policy: None,
        cycle_reserve_policy: None,
    }
}

const fn update_monotonic_transition(
    endpoint: &'static str,
    command_kind: ReplayCommandKindLabel,
) -> EndpointReplayPolicy {
    endpoint_policy(
        endpoint,
        ReplayPolicy::MonotonicTransition { command_kind },
        CostClass::None,
        None,
        None,
    )
}

const fn update_read_only(endpoint: &'static str) -> EndpointReplayPolicy {
    endpoint_policy(
        endpoint,
        ReplayPolicy::QueryOrReadOnly,
        CostClass::None,
        None,
        None,
    )
}

const fn update_command_dispatch(
    endpoint: &'static str,
    command_kind: ReplayCommandKindLabel,
    command_manifest: ReplayCommandManifestLabel,
    implementation_status: ReplayImplementationStatus,
    cost_class: CostClass,
    quota_policy: Option<ReplayQuotaPolicyLabel>,
    cycle_reserve_policy: Option<ReplayCycleReservePolicyLabel>,
) -> EndpointReplayPolicy {
    EndpointReplayPolicy {
        endpoint,
        endpoint_kind: EndpointKind::Update,
        replay_policy: ReplayPolicy::CommandDispatch {
            command_kind,
            command_manifest,
        },
        implementation_status,
        cost_class,
        quota_policy,
        cycle_reserve_policy,
    }
}

const fn endpoint_policy(
    endpoint: &'static str,
    replay_policy: ReplayPolicy,
    cost_class: CostClass,
    quota_policy: Option<ReplayQuotaPolicyLabel>,
    cycle_reserve_policy: Option<ReplayCycleReservePolicyLabel>,
) -> EndpointReplayPolicy {
    EndpointReplayPolicy {
        endpoint,
        endpoint_kind: EndpointKind::Update,
        replay_policy,
        implementation_status: ReplayImplementationStatus::Implemented,
        cost_class,
        quota_policy,
        cycle_reserve_policy,
    }
}
