//! Module: replay_policy::endpoint_manifest
//!
//! Responsibility: record replay policy for Canic-owned endpoint surfaces.
//! Does not own: endpoint routing, workflow execution, or replay receipt storage.
//! Boundary: common role methods dispatch to variant manifests; other methods remain direct.

use crate::replay_policy::types::{
    CostClass, EndpointKind, EndpointReplayPolicy, ReplayCommandKindLabel,
    ReplayCommandManifestLabel, ReplayCycleReservePolicyLabel, ReplayImplementationStatus,
    ReplayPolicy, ReplayQuotaPolicyLabel,
};

/// Canonical replay-policy rows for Canic endpoint methods.
pub const ENDPOINT_REPLAY_POLICY_MANIFEST: &[EndpointReplayPolicy] = &[
    update_command_dispatch(
        "canic_command",
        command_kind("role.command.v1"),
        command_manifest("role.command.variant_manifest.v1"),
        ReplayImplementationStatus::Implemented,
        CostClass::None,
        None,
        None,
    ),
    query_read_only("canic_status"),
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
