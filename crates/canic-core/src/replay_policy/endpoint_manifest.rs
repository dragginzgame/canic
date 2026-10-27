//! Module: replay_policy::endpoint_manifest
//!
//! Responsibility: record replay policy for Canic-owned endpoint surfaces.
//! Does not own: endpoint routing, workflow execution, or replay receipt storage.
//! Boundary: common role methods dispatch to variant manifests; other methods remain direct.

use crate::replay_policy::types::{
    CostClass, EndpointKind, EndpointReplayPolicy, ReplayCommandKindLabel,
    ReplayCycleReservePolicyLabel, ReplayImplementationStatus, ReplayPolicy,
    ReplayQuotaPolicyLabel,
};

/// Canonical replay-policy rows for ordinary, Coordinator and Root endpoint methods.
pub const ENDPOINT_REPLAY_POLICY_MANIFEST: &[EndpointReplayPolicy] = &[
    endpoint_policy(
        crate::protocol::pool_control::import::METHOD,
        ReplayPolicy::SnapshotConvergent {
            command_kind: command_kind("canister_pool.import.v1"),
        },
        CostClass::ManagementDeployment,
        Some(crate::replay_policy::quota::DEPLOYMENT_QUOTA_V1),
        Some(crate::replay_policy::quota::DEPLOYMENT_RESERVE_V1),
    ),
    endpoint_policy(
        crate::protocol::import_control::reserve::METHOD,
        ReplayPolicy::ReplayProtected {
            command_kind: command_kind("canister_pool.capacity_import.v1"),
            requires_operation_id: true,
        },
        CostClass::ManagementDeployment,
        Some(crate::replay_policy::quota::DEPLOYMENT_QUOTA_V1),
        Some(crate::replay_policy::quota::DEPLOYMENT_RESERVE_V1),
    ),
    endpoint_policy(
        crate::protocol::import_control::advance::METHOD,
        ReplayPolicy::ReplayProtected {
            command_kind: command_kind("canister_pool.capacity_import.v1"),
            requires_operation_id: true,
        },
        CostClass::ManagementDeployment,
        Some(crate::replay_policy::quota::DEPLOYMENT_QUOTA_V1),
        Some(crate::replay_policy::quota::DEPLOYMENT_RESERVE_V1),
    ),
    endpoint_policy(
        crate::protocol::import_control::settle::METHOD,
        ReplayPolicy::ReplayProtected {
            command_kind: command_kind("canister_pool.capacity_import.v1"),
            requires_operation_id: true,
        },
        CostClass::ManagementDeployment,
        Some(crate::replay_policy::quota::DEPLOYMENT_QUOTA_V1),
        Some(crate::replay_policy::quota::DEPLOYMENT_RESERVE_V1),
    ),
    endpoint_policy(
        crate::protocol::import_control::release::METHOD,
        ReplayPolicy::ReplayProtected {
            command_kind: command_kind("canister_pool.capacity_import.v1"),
            requires_operation_id: true,
        },
        CostClass::ManagementDeployment,
        Some(crate::replay_policy::quota::DEPLOYMENT_QUOTA_V1),
        Some(crate::replay_policy::quota::DEPLOYMENT_RESERVE_V1),
    ),
    endpoint_policy(
        crate::protocol::inspection_history::METHOD,
        ReplayPolicy::QueryOrReadOnly,
        CostClass::None,
        None,
        None,
    ),
    endpoint_policy(
        crate::protocol::batch_control::prepare::METHOD,
        ReplayPolicy::ReplayProtected {
            command_kind: command_kind("coordinator.provision_components.v1"),
            requires_operation_id: true,
        },
        CostClass::None,
        None,
        None,
    ),
    endpoint_policy(
        crate::protocol::batch_control::accept::METHOD,
        ReplayPolicy::ResponseIdempotent {
            command_kind: command_kind("component_provisioning.accept_root_batch.v1"),
        },
        CostClass::None,
        None,
        None,
    ),
    endpoint_policy(
        crate::protocol::batch_control::synchronize::METHOD,
        ReplayPolicy::ResponseIdempotent {
            command_kind: command_kind(
                "component_provisioning.synchronize_affected_directories.v1",
            ),
        },
        CostClass::None,
        None,
        None,
    ),
    endpoint_policy(
        crate::protocol::retirement_control::root_remove::METHOD,
        ReplayPolicy::ReplayProtected {
            command_kind: command_kind("root.remove_root.v1"),
            requires_operation_id: true,
        },
        CostClass::ValueTransfer,
        Some(crate::replay_policy::quota::VALUE_TRANSFER_QUOTA_V1),
        Some(crate::replay_policy::quota::VALUE_TRANSFER_RESERVE_V1),
    ),
    endpoint_policy(
        crate::protocol::retirement_control::remove::METHOD,
        ReplayPolicy::ReplayProtected {
            command_kind: command_kind("coordinator.remove_root.v1"),
            requires_operation_id: true,
        },
        CostClass::None,
        None,
        None,
    ),
    endpoint_policy(
        crate::protocol::retirement_control::retire::METHOD,
        ReplayPolicy::ReplayProtected {
            command_kind: command_kind("fleet.retirement.ledger.transfer.v1"),
            requires_operation_id: true,
        },
        CostClass::ValueTransfer,
        Some(crate::replay_policy::quota::VALUE_TRANSFER_QUOTA_V1),
        Some(crate::replay_policy::quota::VALUE_TRANSFER_RESERVE_V1),
    ),
    endpoint_policy(
        crate::protocol::component_control::prepare::METHOD,
        ReplayPolicy::ResponseIdempotent {
            command_kind: command_kind("component_registry.prepare_root.v1"),
        },
        CostClass::None,
        None,
        None,
    ),
    endpoint_policy(
        crate::protocol::component_control::provision::METHOD,
        ReplayPolicy::ReplayProtected {
            command_kind: command_kind("root.provision_component.v1"),
            requires_operation_id: true,
        },
        CostClass::ManagementDeployment,
        Some(crate::replay_policy::quota::DEPLOYMENT_QUOTA_V1),
        Some(crate::replay_policy::quota::DEPLOYMENT_RESERVE_V1),
    ),
    endpoint_policy(
        crate::protocol::pool_control::maintain::METHOD,
        ReplayPolicy::SnapshotConvergent {
            command_kind: command_kind("canister_pool.maintain.v1"),
        },
        CostClass::ManagementDeployment,
        Some(crate::replay_policy::quota::DEPLOYMENT_QUOTA_V1),
        Some(crate::replay_policy::quota::DEPLOYMENT_RESERVE_V1),
    ),
    endpoint_policy(
        crate::protocol::registry_control::synchronize::METHOD,
        ReplayPolicy::ReplayProtected {
            command_kind: command_kind("fleet_registry.synchronize_root.v1"),
            requires_operation_id: true,
        },
        CostClass::None,
        None,
        None,
    ),
    endpoint_policy(
        crate::protocol::root_store_control::adopt::METHOD,
        ReplayPolicy::ReplayProtected {
            command_kind: command_kind("root.adopt_store.v1"),
            requires_operation_id: true,
        },
        CostClass::None,
        None,
        None,
    ),
    endpoint_policy(
        crate::protocol::root_store_control::bootstrap::METHOD,
        ReplayPolicy::ReplayProtected {
            command_kind: command_kind("root.bootstrap_store.v1"),
            requires_operation_id: true,
        },
        CostClass::ManagementDeployment,
        Some(crate::replay_policy::quota::DEPLOYMENT_QUOTA_V1),
        Some(crate::replay_policy::quota::DEPLOYMENT_RESERVE_V1),
    ),
    endpoint_policy(
        crate::protocol::root_store_control::prepare::METHOD,
        ReplayPolicy::MonotonicTransition {
            command_kind: command_kind("root.prepare_store_fixture.v1"),
        },
        CostClass::None,
        None,
        None,
    ),
    endpoint_policy(
        crate::protocol::root_store_control::publish::METHOD,
        ReplayPolicy::MonotonicTransition {
            command_kind: command_kind("wasm_store.admin.v1"),
        },
        CostClass::DurablePublish,
        Some(crate::replay_policy::quota::DURABLE_PUBLISH_QUOTA_V1),
        Some(crate::replay_policy::quota::DURABLE_PUBLISH_RESERVE_V1),
    ),
    endpoint_policy(
        crate::protocol::root_deletion::complete::METHOD,
        ReplayPolicy::ResponseIdempotent {
            command_kind: command_kind("fleet_registry.complete_root_deletion.v1"),
        },
        CostClass::None,
        None,
        None,
    ),
    endpoint_policy(
        crate::protocol::root_deletion::prepare::METHOD,
        ReplayPolicy::ResponseIdempotent {
            command_kind: command_kind("fleet_registry.begin_root_deletion_execution.v1"),
        },
        CostClass::None,
        None,
        None,
    ),
    endpoint_policy(
        crate::protocol::pool_control::handoff::METHOD,
        ReplayPolicy::SnapshotConvergent {
            command_kind: command_kind("canister_pool.handoff.v1"),
        },
        CostClass::None,
        None,
        None,
    ),
    endpoint_policy(
        crate::protocol::registry_control::join::METHOD,
        ReplayPolicy::ResponseIdempotent {
            command_kind: command_kind("fleet_registry.root_join.v1"),
        },
        CostClass::None,
        None,
        None,
    ),
    endpoint_policy(
        crate::protocol::registry_control::activate::METHOD,
        ReplayPolicy::ResponseIdempotent {
            command_kind: command_kind("fleet_registry.activate.v1"),
        },
        CostClass::None,
        None,
        None,
    ),
    endpoint_policy(
        crate::protocol::coordinator_acknowledgements::acknowledge::METHOD,
        ReplayPolicy::ResponseIdempotent {
            command_kind: command_kind("fleet_registry.acknowledge_root.v1"),
        },
        CostClass::None,
        None,
        None,
    ),
    endpoint_policy(
        crate::protocol::cycle_refill::preview::METHOD,
        ReplayPolicy::QueryOrReadOnly,
        CostClass::None,
        None,
        None,
    ),
    endpoint_policy(
        crate::protocol::cycle_refill::execute::METHOD,
        ReplayPolicy::ReplayProtected {
            command_kind: command_kind("icp.refill.v1"),
            requires_operation_id: true,
        },
        CostClass::ValueTransfer,
        Some(crate::replay_policy::quota::VALUE_TRANSFER_QUOTA_V1),
        Some(crate::replay_policy::quota::VALUE_TRANSFER_RESERVE_V1),
    ),
    endpoint_policy(
        crate::protocol::component_control::child::METHOD,
        ReplayPolicy::ReplayProtected {
            command_kind: command_kind("root.provision_child.v1"),
            requires_operation_id: true,
        },
        CostClass::ManagementDeployment,
        Some(crate::replay_policy::quota::DEPLOYMENT_QUOTA_V1),
        Some(crate::replay_policy::quota::DEPLOYMENT_RESERVE_V1),
    ),
    endpoint_policy(
        crate::protocol::component_control::peer::METHOD,
        ReplayPolicy::ReplayProtected {
            command_kind: command_kind("root.provision_peer.v1"),
            requires_operation_id: true,
        },
        CostClass::ManagementDeployment,
        Some(crate::replay_policy::quota::DEPLOYMENT_QUOTA_V1),
        Some(crate::replay_policy::quota::DEPLOYMENT_RESERVE_V1),
    ),
    endpoint_policy(
        crate::protocol::component_control::remove::METHOD,
        ReplayPolicy::ReplayProtected {
            command_kind: command_kind("root.remove_component.v1"),
            requires_operation_id: true,
        },
        CostClass::ManagementDeployment,
        Some(crate::replay_policy::quota::DEPLOYMENT_QUOTA_V1),
        Some(crate::replay_policy::quota::DEPLOYMENT_RESERVE_V1),
    ),
    endpoint_policy(
        crate::protocol::component_control::subtree::METHOD,
        ReplayPolicy::ReplayProtected {
            command_kind: command_kind("root.remove_subtree.v1"),
            requires_operation_id: true,
        },
        CostClass::ManagementDeployment,
        Some(crate::replay_policy::quota::DEPLOYMENT_QUOTA_V1),
        Some(crate::replay_policy::quota::DEPLOYMENT_RESERVE_V1),
    ),
    endpoint_policy(
        crate::protocol::funding_control::root_accept::METHOD,
        ReplayPolicy::ReplayProtected {
            command_kind: command_kind("fleet_root_funding.accept.v1"),
            requires_operation_id: true,
        },
        CostClass::None,
        None,
        None,
    ),
    endpoint_policy(
        crate::protocol::funding_control::root_prepare::METHOD,
        ReplayPolicy::ReplayProtected {
            command_kind: command_kind("fleet_funding_policy_rotation.prepare_root.v1"),
            requires_operation_id: true,
        },
        CostClass::None,
        None,
        None,
    ),
    endpoint_policy(
        crate::protocol::funding_control::root_activate::METHOD,
        ReplayPolicy::ReplayProtected {
            command_kind: command_kind("fleet_funding_policy_rotation.activate_root.v1"),
            requires_operation_id: true,
        },
        CostClass::None,
        None,
        None,
    ),
    endpoint_policy(
        crate::protocol::funding_control::request::METHOD,
        ReplayPolicy::ReplayProtected {
            command_kind: command_kind("fleet_root_funding.request.v1"),
            requires_operation_id: true,
        },
        CostClass::ValueTransfer,
        Some(crate::replay_policy::quota::VALUE_TRANSFER_QUOTA_V1),
        Some(crate::replay_policy::quota::VALUE_TRANSFER_RESERVE_V1),
    ),
    endpoint_policy(
        crate::protocol::funding_control::begin::METHOD,
        ReplayPolicy::ReplayProtected {
            command_kind: command_kind("fleet_funding_policy_rotation.begin.v1"),
            requires_operation_id: true,
        },
        CostClass::None,
        None,
        None,
    ),
    endpoint_policy(
        crate::protocol::funding_control::stage::METHOD,
        ReplayPolicy::ReplayProtected {
            command_kind: command_kind("fleet_funding_policy_rotation.stage_root.v1"),
            requires_operation_id: true,
        },
        CostClass::None,
        None,
        None,
    ),
    endpoint_policy(
        crate::protocol::funding_control::apply::METHOD,
        ReplayPolicy::ReplayProtected {
            command_kind: command_kind("fleet_funding_policy_rotation.apply.v1"),
            requires_operation_id: true,
        },
        CostClass::None,
        None,
        None,
    ),
    endpoint_policy(
        crate::protocol::funding_control::set_enabled::METHOD,
        ReplayPolicy::ResponseIdempotent {
            command_kind: command_kind("fleet_root_funding.set_enabled.v1"),
        },
        CostClass::None,
        None,
        None,
    ),
    endpoint_policy(
        crate::protocol::root_activation::resume::METHOD,
        ReplayPolicy::ResponseIdempotent {
            command_kind: command_kind("fleet_activation.resume.v1"),
        },
        CostClass::None,
        None,
        None,
    ),
    endpoint_policy(
        crate::protocol::root_activation::prepare::METHOD,
        ReplayPolicy::ResponseIdempotent {
            command_kind: command_kind("fleet_activation.prepare.v1"),
        },
        CostClass::None,
        None,
        None,
    ),
    endpoint_policy(
        crate::protocol::store_control::grant::METHOD,
        ReplayPolicy::MonotonicTransition {
            command_kind: command_kind("wasm_store.set_fixture_grant.v1"),
        },
        CostClass::None,
        None,
        None,
    ),
    endpoint_policy(
        crate::protocol::store_control::reclaim::METHOD,
        ReplayPolicy::SnapshotConvergent {
            command_kind: command_kind("wasm_store.reclaim_deletion_cycles.v1"),
        },
        CostClass::ValueTransfer,
        Some(crate::replay_policy::quota::VALUE_TRANSFER_QUOTA_V1),
        Some(crate::replay_policy::quota::VALUE_TRANSFER_RESERVE_V1),
    ),
    endpoint_policy(
        crate::protocol::store_activation::activate::METHOD,
        ReplayPolicy::ResponseIdempotent {
            command_kind: command_kind("fleet_activation.activate.v1"),
        },
        CostClass::None,
        None,
        None,
    ),
    endpoint_policy(
        crate::protocol::store_activation::prepare::METHOD,
        ReplayPolicy::MonotonicTransition {
            command_kind: command_kind("fleet_activation.prepare_credential_generation.v1"),
        },
        CostClass::None,
        None,
        None,
    ),
    endpoint_policy(
        crate::protocol::store_gc::METHOD,
        ReplayPolicy::MonotonicTransition {
            command_kind: command_kind("wasm_store.gc.v1"),
        },
        CostClass::None,
        None,
        None,
    ),
    endpoint_policy(
        crate::protocol::store_publication::fixture_prepare::METHOD,
        ReplayPolicy::MonotonicTransition {
            command_kind: command_kind("wasm_store.prepare_fixture.v1"),
        },
        CostClass::None,
        None,
        None,
    ),
    endpoint_policy(
        crate::protocol::store_publication::inspect::METHOD,
        ReplayPolicy::QueryOrReadOnly,
        CostClass::None,
        None,
        None,
    ),
    endpoint_policy(
        crate::protocol::store_publication::prepare::METHOD,
        ReplayPolicy::MonotonicTransition {
            command_kind: command_kind("wasm_store.prepare.v1"),
        },
        CostClass::None,
        None,
        None,
    ),
    endpoint_policy(
        crate::protocol::store_publication::stage::METHOD,
        ReplayPolicy::MonotonicTransition {
            command_kind: command_kind("wasm_store.stage_manifest.v1"),
        },
        CostClass::None,
        None,
        None,
    ),
    endpoint_policy(
        crate::protocol::component_initialization::METHOD,
        ReplayPolicy::ResponseIdempotent {
            command_kind: command_kind("root.bind_component_initialization.v1"),
        },
        CostClass::None,
        None,
        None,
    ),
    endpoint_policy(
        crate::protocol::runtime_configuration::METHOD,
        ReplayPolicy::ResponseIdempotent {
            command_kind: command_kind("component_runtime.configure.v1"),
        },
        CostClass::None,
        None,
        None,
    ),
    endpoint_policy(
        crate::protocol::cascade::state::METHOD,
        ReplayPolicy::SnapshotConvergent {
            command_kind: command_kind("cascade.sync_state.v1"),
        },
        CostClass::None,
        None,
        None,
    ),
    endpoint_policy(
        crate::protocol::cascade::topology::METHOD,
        ReplayPolicy::SnapshotConvergent {
            command_kind: command_kind("cascade.sync_topology.v1"),
        },
        CostClass::None,
        None,
        None,
    ),
    endpoint_policy(
        crate::protocol::fleet_state_control::funding::METHOD,
        ReplayPolicy::ResponseIdempotent {
            command_kind: command_kind("fleet.set_cycles_funding.v1"),
        },
        CostClass::None,
        None,
        None,
    ),
    endpoint_policy(
        crate::protocol::fleet_state_control::status::METHOD,
        ReplayPolicy::ResponseIdempotent {
            command_kind: command_kind("fleet.set_status.v1"),
        },
        CostClass::None,
        None,
        None,
    ),
    endpoint_policy(
        crate::protocol::pool_retry::refill::METHOD,
        ReplayPolicy::SnapshotConvergent {
            command_kind: command_kind("canister_pool.retry_refill.v1"),
        },
        CostClass::ValueTransfer,
        Some(crate::replay_policy::quota::VALUE_TRANSFER_QUOTA_V1),
        Some(crate::replay_policy::quota::VALUE_TRANSFER_RESERVE_V1),
    ),
    endpoint_policy(
        crate::protocol::pool_retry::reset::METHOD,
        ReplayPolicy::SnapshotConvergent {
            command_kind: command_kind("canister_pool.retry_reset.v1"),
        },
        CostClass::ManagementDeployment,
        Some(crate::replay_policy::quota::DEPLOYMENT_QUOTA_V1),
        Some(crate::replay_policy::quota::DEPLOYMENT_RESERVE_V1),
    ),
    endpoint_policy(
        crate::protocol::issuer_proof::INSTALL,
        ReplayPolicy::IntentionallyNonIdempotent {
            command_kind: command_kind("auth.install_active_delegation_proof.v1"),
            reason: "controller maintenance replaces issuer-local active proof metadata",
        },
        CostClass::None,
        None,
        None,
    ),
    endpoint_policy(
        crate::protocol::root_capability::acknowledge::METHOD,
        ReplayPolicy::ResponseIdempotent {
            command_kind: command_kind("root.acknowledge_placement_receipt"),
        },
        CostClass::None,
        None,
        None,
    ),
    endpoint_policy(
        crate::protocol::root_capability::allocate::METHOD,
        ReplayPolicy::ReplayProtected {
            command_kind: command_kind("root.allocate_placement_child"),
            requires_operation_id: true,
        },
        CostClass::ManagementDeployment,
        Some(crate::replay_policy::quota::DEPLOYMENT_QUOTA_V1),
        Some(crate::replay_policy::quota::DEPLOYMENT_RESERVE_V1),
    ),
    endpoint_policy(
        crate::protocol::root_capability::create::METHOD,
        ReplayPolicy::ReplayProtected {
            command_kind: command_kind("root.provision"),
            requires_operation_id: true,
        },
        CostClass::ManagementDeployment,
        Some(crate::replay_policy::quota::DEPLOYMENT_QUOTA_V1),
        Some(crate::replay_policy::quota::DEPLOYMENT_RESERVE_V1),
    ),
    endpoint_policy(
        crate::protocol::root_capability::recycle::METHOD,
        ReplayPolicy::ReplayProtected {
            command_kind: command_kind("root.recycle_canister.v1"),
            requires_operation_id: true,
        },
        CostClass::ManagementDeployment,
        Some(crate::replay_policy::quota::DEPLOYMENT_QUOTA_V1),
        Some(crate::replay_policy::quota::DEPLOYMENT_RESERVE_V1),
    ),
    endpoint_policy(
        crate::protocol::root_capability::cycles::METHOD,
        ReplayPolicy::ReplayProtected {
            command_kind: command_kind("root.request_cycles.v1"),
            requires_operation_id: true,
        },
        CostClass::ValueTransfer,
        Some(crate::replay_policy::quota::VALUE_TRANSFER_QUOTA_V1),
        Some(crate::replay_policy::quota::VALUE_TRANSFER_RESERVE_V1),
    ),
    endpoint_policy(
        crate::protocol::cycles_capability::METHOD,
        ReplayPolicy::ReplayProtected {
            command_kind: command_kind("root.request_cycles.v1"),
            requires_operation_id: true,
        },
        CostClass::ValueTransfer,
        Some(crate::replay_policy::quota::VALUE_TRANSFER_QUOTA_V1),
        Some(crate::replay_policy::quota::VALUE_TRANSFER_RESERVE_V1),
    ),
    query_read_only(crate::protocol::admission_control::root_status::METHOD),
    query_read_only(crate::protocol::admission_control::status::METHOD),
    endpoint_policy(
        crate::protocol::admission_control::mutate::METHOD,
        ReplayPolicy::ReplayProtected {
            command_kind: command_kind("fleet_admission.mutate.v1"),
            requires_operation_id: true,
        },
        CostClass::None,
        None,
        None,
    ),
    query_read_only(crate::protocol::funding_read::coordinator_status::METHOD),
    query_read_only("icrc10_supported_standards"),
    query_read_only("icrc21_canister_call_consent_message"),
    endpoint_policy(
        crate::protocol::inspection::METHOD,
        ReplayPolicy::QueryOrReadOnly,
        CostClass::None,
        None,
        None,
    ),
    query_read_only(crate::protocol::funding_read::root_status::METHOD),
    query_read_only(crate::protocol::admission_control::operation::METHOD),
    query_read_only(crate::protocol::funding_read::rotation::METHOD),
    query_read_only(crate::protocol::retirement_control::status::METHOD),
    query_read_only(crate::protocol::retirement_control::removal_status::METHOD),
    query_read_only(crate::protocol::component_read::child_allocation::METHOD),
    query_read_only(crate::protocol::component_read::allocation::METHOD),
    query_read_only(crate::protocol::subtree_read::METHOD),
    query_read_only(crate::protocol::component_removal_read::METHOD),
    query_read_only(crate::protocol::component_read::provisioning::METHOD),
    query_read_only(crate::protocol::root_store_control::adoption::METHOD),
    query_read_only(crate::protocol::root_activation::status::METHOD),
    query_read_only(crate::protocol::registry_control::synchronization_status::METHOD),
    query_read_only(crate::protocol::cycle_refill::status::METHOD),
    query_read_only(crate::protocol::root_store_control::bootstrap_status::METHOD),
    query_read_only(crate::protocol::funding_read::release::METHOD),
    endpoint_policy(
        crate::protocol::protected_state::snapshot_prepare::METHOD,
        ReplayPolicy::ResponseIdempotent {
            command_kind: command_kind("authority_restore.prepare_snapshot.v1"),
        },
        CostClass::None,
        None,
        None,
    ),
    endpoint_policy(
        crate::protocol::protected_state::snapshot_resume::METHOD,
        ReplayPolicy::ResponseIdempotent {
            command_kind: command_kind("authority_restore.resume_snapshot.v1"),
        },
        CostClass::None,
        None,
        None,
    ),
    query_read_only(crate::protocol::protected_state::authority_restore::METHOD),
    query_read_only(crate::protocol::protected_state::fleet_state::METHOD),
    query_read_only("canic_root_checkpoint_status"),
    update_monotonic_transition(
        "canic_root_checkpoint_begin",
        command_kind("root.checkpoint.begin.v1"),
    ),
    update_monotonic_transition(
        "canic_root_checkpoint_advance",
        command_kind("root.checkpoint.advance.v1"),
    ),
    update_monotonic_transition(
        "canic_root_checkpoint_release",
        command_kind("root.checkpoint.release.v1"),
    ),
    endpoint_policy(
        crate::protocol::caller_authority::PREPARE,
        ReplayPolicy::ReplayProtected {
            command_kind: command_kind("caller_authority.publication.v1"),
            requires_operation_id: true,
        },
        CostClass::None,
        None,
        None,
    ),
    endpoint_policy(
        crate::protocol::caller_authority::COMMIT,
        ReplayPolicy::ReplayProtected {
            command_kind: command_kind("caller_authority.publication.v1"),
            requires_operation_id: true,
        },
        CostClass::None,
        None,
        None,
    ),
    endpoint_policy(
        crate::protocol::caller_authority::COMPLETE,
        ReplayPolicy::ReplayProtected {
            command_kind: command_kind("caller_authority.publication.v1"),
            requires_operation_id: true,
        },
        CostClass::None,
        None,
        None,
    ),
    endpoint_policy(
        crate::protocol::caller_authority::RELEASE,
        ReplayPolicy::ResponseIdempotent {
            command_kind: command_kind("application_startup.release.v1"),
        },
        CostClass::None,
        None,
        None,
    ),
    update_read_only(crate::protocol::observability_relay::child_funding::METHOD),
    update_read_only(crate::protocol::observability_relay::cycle_balance::METHOD),
    update_read_only(crate::protocol::observability_relay::cycle_history::METHOD),
    update_read_only(crate::protocol::observability_relay::cycle_topups::METHOD),
    update_read_only(crate::protocol::observability_relay::memory_allocations::METHOD),
    update_read_only(crate::protocol::observability_relay::metrics::METHOD),
    update_read_only(crate::protocol::root_observation::child_funding::METHOD),
    update_read_only(crate::protocol::root_observation::cycle_balance::METHOD),
    update_read_only(crate::protocol::root_observation::cycle_history::METHOD),
    update_read_only(crate::protocol::root_observation::cycle_topups::METHOD),
    update_read_only(crate::protocol::root_observation::memory_allocations::METHOD),
    update_read_only(crate::protocol::root_observation::metrics::METHOD),
    query_read_only(crate::protocol::inspection_reserve::METHOD),
    query_read_only(crate::protocol::coordinator_acknowledgements::METHOD),
    query_read_only(crate::protocol::release_evidence::intents::METHOD),
    query_read_only(crate::protocol::release_evidence::receipts::METHOD),
    query_read_only(crate::protocol::registry_discovery::manifest::METHOD),
    query_read_only(crate::protocol::registry_discovery::version::METHOD),
    update_read_only(crate::protocol::CANIC_ROOT_MEMBERSHIP),
    query_read_only(crate::protocol::CANIC_ROOT_FIXTURE_STATUS),
    query_read_only(crate::protocol::pool_import_read::status::METHOD),
    query_read_only(crate::protocol::pool_import_read::context::METHOD),
    query_read_only(crate::protocol::pool_census::METHOD),
    query_read_only(crate::protocol::pool_inventory::METHOD),
    query_read_only(crate::protocol::provisioning_census::METHOD),
    query_read_only(crate::protocol::root_store_reporting::overview::METHOD),
    query_read_only(crate::protocol::root_config::METHOD),
    query_read_only(crate::protocol::runtime_reporting::METHOD),
    query_read_only(crate::protocol::CANIC_PUBLIC_HEALTH),
    query_read_only(crate::protocol::CANIC_PUBLIC_METRICS),
    query_read_only(crate::protocol::CANIC_PUBLIC_HISTORY),
    query_read_only(crate::protocol::component_directory::HEAD),
    query_read_only(crate::protocol::component_directory::PAGE),
    query_read_only(crate::protocol::component_directory::MEMBER_PAGE),
    query_read_only(crate::protocol::CANIC_PUBLIC_OVERVIEW),
    query_read_only(crate::protocol::CANIC_PUBLIC_CHILDREN),
    query_read_only(crate::protocol::cycle_balance::METHOD),
    query_read_only(crate::protocol::cycle_history::METHOD),
    query_read_only(crate::protocol::cycle_topups::METHOD),
    query_read_only(crate::protocol::caller_authority_status::METHOD),
    query_read_only(crate::protocol::child_funding::METHOD),
    query_read_only(crate::protocol::protected_readiness::METHOD),
    query_read_only(crate::protocol::protected_runtime::METHOD),
    query_read_only(crate::protocol::protected_binding::METHOD),
    query_read_only(crate::protocol::protected_health::METHOD),
    query_read_only(crate::protocol::protected_logs::METHOD),
    query_read_only(crate::protocol::protected_metrics::METHOD),
    query_read_only(crate::protocol::protected_memory_allocations::METHOD),
    query_read_only(crate::protocol::issuer_proof::status::METHOD),
    endpoint_policy(
        crate::protocol::admission_control::target_prepare::METHOD,
        ReplayPolicy::ReplayProtected {
            command_kind: command_kind("fleet_admission.prepare_target.v1"),
            requires_operation_id: true,
        },
        CostClass::None,
        None,
        None,
    ),
    endpoint_policy(
        crate::protocol::admission_control::target_activate::METHOD,
        ReplayPolicy::ReplayProtected {
            command_kind: command_kind("fleet_admission.activate_target.v1"),
            requires_operation_id: true,
        },
        CostClass::None,
        None,
        None,
    ),
    endpoint_policy(
        crate::protocol::admission_control::target_open::METHOD,
        ReplayPolicy::ReplayProtected {
            command_kind: command_kind("fleet_admission.open_target.v1"),
            requires_operation_id: true,
        },
        CostClass::None,
        None,
        None,
    ),
    endpoint_policy(
        crate::protocol::admission_control::root_prepare::METHOD,
        ReplayPolicy::ReplayProtected {
            command_kind: command_kind("fleet_admission.prepare_root.v1"),
            requires_operation_id: true,
        },
        CostClass::None,
        None,
        None,
    ),
    endpoint_policy(
        crate::protocol::admission_control::root_activate::METHOD,
        ReplayPolicy::ReplayProtected {
            command_kind: command_kind("fleet_admission.activate_root.v1"),
            requires_operation_id: true,
        },
        CostClass::None,
        None,
        None,
    ),
    endpoint_policy(
        crate::protocol::admission_control::root_open::METHOD,
        ReplayPolicy::ReplayProtected {
            command_kind: command_kind("fleet_admission.open_root.v1"),
            requires_operation_id: true,
        },
        CostClass::None,
        None,
        None,
    ),
    query_read_only(crate::protocol::CANIC_ADMISSION_STATUS),
    endpoint_policy(
        crate::protocol::issuer_control::configure::METHOD,
        ReplayPolicy::SnapshotConvergent {
            command_kind: command_kind("auth.configure_root_issuer.v1"),
        },
        CostClass::None,
        None,
        None,
    ),
    endpoint_policy(
        crate::protocol::issuer_control::proof::METHOD,
        ReplayPolicy::SnapshotConvergent {
            command_kind: command_kind("auth.get_or_create_chain_key_delegation_proof.v1"),
        },
        CostClass::RootChainKeySigning,
        Some(crate::replay_policy::quota::ROOT_CHAIN_KEY_SIGNING_QUOTA_V1),
        None,
    ),
    endpoint_policy(
        crate::protocol::issuer_control::public_key::METHOD,
        ReplayPolicy::QueryOrReadOnly,
        CostClass::None,
        None,
        None,
    ),
    query_read_only(crate::protocol::issuer_renewal::METHOD),
    query_read_only(crate::protocol::role_attestation::get::METHOD),
    endpoint_policy(
        crate::protocol::role_attestation::prepare::METHOD,
        ReplayPolicy::ReplayProtected {
            command_kind: command_kind("auth.prepare_role_attestation.v1"),
            requires_operation_id: true,
        },
        CostClass::RootCanisterSignaturePrepare,
        Some(crate::replay_policy::quota::ROOT_CANISTER_SIGNATURE_PREPARE_QUOTA_V1),
        None,
    ),
    query_read_only(crate::protocol::root_removal_read::METHOD),
    query_read_only(crate::protocol::component_read::coordinator_provisioning::METHOD),
    query_read_only(crate::protocol::registry_discovery::snapshot::METHOD),
    query_read_only(crate::protocol::registry_discovery::root_inventory::METHOD),
    query_read_only(crate::protocol::store_catalog::fixture::METHOD),
    query_read_only(crate::protocol::store_catalog::grant::METHOD),
    query_read_only(crate::protocol::store_catalog::template::METHOD),
    query_read_only(crate::protocol::store_inventory::catalog::METHOD),
    query_read_only(crate::protocol::store_inventory::storage::METHOD),
];

/// Exact replay-policy rows for the Store's template and fixture data lanes.
pub const STORE_ENDPOINT_REPLAY_POLICY_MANIFEST: &[EndpointReplayPolicy] = &[
    query_read_only(crate::protocol::store_status::authority::METHOD),
    query_read_only(crate::protocol::store_status::activation::METHOD),
    query_read_only(crate::protocol::store_status::gc::METHOD),
    update_read_only(crate::protocol::CANIC_WASM_STORE_CHUNK),
    update_read_only(crate::protocol::CANIC_WASM_STORE_FIXTURE_CHUNK),
    update_monotonic_transition(
        crate::protocol::CANIC_WASM_STORE_PUBLISH_FIXTURE,
        command_kind("wasm_store.publish_fixture.v1"),
    ),
    update_monotonic_transition(
        crate::protocol::CANIC_WASM_STORE_PUBLISH_CHUNK,
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
