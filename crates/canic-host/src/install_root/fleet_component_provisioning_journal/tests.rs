//! Focused tests for host-owned Component provisioning installation recovery.

use super::*;
use crate::{
    fleet_catalog::{CommittedFleetCatalog, FleetCatalogEntryV1},
    fleet_install_plan::{
        FleetInstallPlan, PersistedFleetInstallPlan, PlannedCanisterCreationFunding,
        PlannedFleetCoordinator,
    },
    install_root::fleet_component_provisioning_plan::CompiledFleetComponentProvisioningPlan,
    test_support::temp_dir,
};
use std::path::Path;

use candid::Principal;
use canic_core::{
    dto::{
        component_provisioning::{
            FleetComponentProvisioningOperation, FleetComponentProvisioningPhase,
            FleetComponentProvisioningPlan, FleetComponentProvisioningPrepareRequest,
            FleetComponentProvisioningStatusResponse,
        },
        fleet_registry::FleetRegistryVersion,
    },
    ids::{
        AppId, CanonicalNetworkId, ComponentDeploymentConfigurationDigest, FleetBinding,
        FleetCoordinatorBinding, FleetId, FleetKey, FleetName, FleetRegistryAuthority,
        ReleaseBuildId, ReleaseBuildNonce, SubnetId,
    },
};

#[test]
fn terminal_catalog_is_gated_by_durable_runtime_activation_evidence() {
    let (plan, compiled, terminal) =
        terminal_transaction("fleet-component-provisioning-install-journal");

    assert_eq!(
        terminal.journal.phase,
        FleetComponentProvisioningInstallPhase::RuntimesActivated
    );
    assert!(matches!(
        record_fleet_catalog_published(&terminal, committed_catalog(&plan, 100)),
        Err(FleetComponentProvisioningInstallJournalError::InvalidTransition { .. })
    ));
    let publishing = begin_fleet_catalog_publication(&terminal, catalog_entry(&plan, 100))
        .expect("persist exact catalog row before publication");
    let published = record_fleet_catalog_published(&publishing, committed_catalog(&plan, 100))
        .expect("record exact catalog receipt");
    let complete = complete_fleet_component_provisioning_install(&published)
        .expect("complete host transaction");
    assert_eq!(
        complete.journal.phase,
        FleetComponentProvisioningInstallPhase::Complete
    );

    let replay =
        plan_fleet_component_provisioning_install(PlanFleetComponentProvisioningInstallRequest {
            fleet_install_plan: &plan,
            coordinator: principal(3),
            fleet_name: FleetName::try_from("main".to_string()).expect("Fleet name"),
            environment: "ic".to_string(),
            compiled,
        })
        .expect("load exact terminal replay");
    assert_eq!(replay.journal, complete.journal);
}

#[test]
fn catalog_publication_cannot_substitute_any_frozen_row_field() {
    let (plan, _compiled, terminal) =
        terminal_transaction("fleet-component-provisioning-conflicting-catalog-row");
    let mut intended = catalog_entry(&plan, 100);
    intended.environment = "local".to_string();
    assert!(matches!(
        begin_fleet_catalog_publication(&terminal, intended),
        Err(FleetComponentProvisioningInstallJournalError::InvalidDocument { .. })
    ));
}

#[test]
fn lost_advance_response_recovers_from_the_persisted_exact_request() {
    let root = temp_dir("fleet-component-provisioning-lost-response");
    let plan = install_plan(&root);
    let compiled = compiled_plan(&plan);
    let planned =
        plan_fleet_component_provisioning_install(PlanFleetComponentProvisioningInstallRequest {
            fleet_install_plan: &plan,
            coordinator: principal(3),
            fleet_name: FleetName::try_from("main".to_string()).expect("Fleet name"),
            environment: "ic".to_string(),
            compiled: compiled.clone(),
        })
        .expect("plan host transaction");
    let preparing = begin_component_provisioning_preparation(&planned).expect("begin preparation");
    let prepared = record_component_provisioning_observed(&preparing, planned_status(&compiled))
        .expect("record prepared status");
    let in_flight = begin_component_provisioning_advance(&prepared).expect("persist exact intent");

    let recovered =
        plan_fleet_component_provisioning_install(PlanFleetComponentProvisioningInstallRequest {
            fleet_install_plan: &plan,
            coordinator: principal(3),
            fleet_name: FleetName::try_from("main".to_string()).expect("Fleet name"),
            environment: "ic".to_string(),
            compiled: compiled.clone(),
        })
        .expect("recover uncertain advance");
    assert_eq!(recovered.journal, in_flight.journal);
    assert!(matches!(
        plan_fleet_component_provisioning_install(PlanFleetComponentProvisioningInstallRequest {
            fleet_install_plan: &plan,
            coordinator: principal(3),
            fleet_name: FleetName::try_from("main".to_string()).expect("Fleet name"),
            environment: "local".to_string(),
            compiled: compiled.clone(),
        }),
        Err(FleetComponentProvisioningInstallJournalError::ConflictingAuthority { .. })
    ));
    assert_eq!(
        recovered.journal.advance_request,
        Some(super::advance_request(
            recovered.journal.last_status.as_ref().expect("status")
        ))
    );
    assert_eq!(
        record_component_provisioning_advanced(&recovered, terminal_status(&compiled))
            .expect("reconcile observed terminal status")
            .journal
            .phase,
        FleetComponentProvisioningInstallPhase::RuntimesActivated
    );
}

#[test]
fn prepare_response_loss_reconciles_every_correlated_nonterminal_phase() {
    let phases = [
        FleetComponentProvisioningPhase::Planned,
        FleetComponentProvisioningPhase::AcceptingRoots,
        FleetComponentProvisioningPhase::RootsAccepted,
        FleetComponentProvisioningPhase::ProvisioningRoots,
        FleetComponentProvisioningPhase::ComponentsProvisioned,
        FleetComponentProvisioningPhase::ServiceTopologyPublished,
        FleetComponentProvisioningPhase::ConfirmingDirectories,
        FleetComponentProvisioningPhase::DirectoriesConfirmed,
        FleetComponentProvisioningPhase::ActivatingRuntimes,
    ];

    for (index, phase) in phases.into_iter().enumerate() {
        let name = format!("fleet-component-provisioning-first-status-{index}");
        let (plan, compiled, preparing) = preparing_transaction(&name);
        let recovered = plan_fleet_component_provisioning_install(
            PlanFleetComponentProvisioningInstallRequest {
                fleet_install_plan: &plan,
                coordinator: principal(3),
                fleet_name: FleetName::try_from("main".to_string()).expect("Fleet name"),
                environment: "ic".to_string(),
                compiled: compiled.clone(),
            },
        )
        .expect("recover retained preparation intent after response loss");
        assert_eq!(recovered.journal, preparing.journal);

        let observed = observed_status(&compiled, phase);
        let retained = record_component_provisioning_observed(&recovered, observed.clone())
            .expect("retain exact monotonic Coordinator status");
        assert_eq!(
            retained.journal.phase,
            FleetComponentProvisioningInstallPhase::Prepared
        );
        assert_eq!(retained.journal.last_status, Some(observed.clone()));

        let advancing = begin_component_provisioning_advance(&retained)
            .expect("persist advance derived from observed status");
        assert_eq!(
            advancing.journal.advance_request,
            Some(super::advance_request(&observed))
        );
    }
}

#[test]
fn first_terminal_status_requires_complete_evidence_and_skips_another_advance() {
    let (plan, compiled, preparing) =
        preparing_transaction("fleet-component-provisioning-first-terminal-status");
    let incomplete = status(
        &compiled,
        FleetComponentProvisioningPhase::RuntimesActivated,
    );
    assert!(matches!(
        record_component_provisioning_observed(&preparing, incomplete),
        Err(FleetComponentProvisioningInstallJournalError::InvalidDocument { .. })
    ));

    let terminal_status = terminal_status(&compiled);
    let terminal = record_component_provisioning_observed(&preparing, terminal_status.clone())
        .expect("retain complete terminal Coordinator evidence");
    assert_eq!(
        terminal.journal.phase,
        FleetComponentProvisioningInstallPhase::RuntimesActivated
    );
    assert_eq!(terminal.journal.last_status, Some(terminal_status));
    assert!(terminal.journal.advance_request.is_none());

    begin_fleet_catalog_publication(&terminal, catalog_entry(&plan, 100))
        .expect("continue directly to catalog publication");
}

#[test]
fn conflicting_coordinator_status_cannot_replace_frozen_authority() {
    let root = temp_dir("fleet-component-provisioning-conflict");
    let plan = install_plan(&root);
    let compiled = compiled_plan(&plan);
    let planned =
        plan_fleet_component_provisioning_install(PlanFleetComponentProvisioningInstallRequest {
            fleet_install_plan: &plan,
            coordinator: principal(3),
            fleet_name: FleetName::try_from("main".to_string()).expect("Fleet name"),
            environment: "ic".to_string(),
            compiled: compiled.clone(),
        })
        .expect("plan host transaction");
    let preparing = begin_component_provisioning_preparation(&planned).expect("begin preparation");
    let mut wrong = planned_status(&compiled);
    wrong.operation_id = [99; 32];
    assert!(matches!(
        record_component_provisioning_observed(&preparing, wrong),
        Err(FleetComponentProvisioningInstallJournalError::InvalidDocument { .. })
    ));

    let mut wrong_count = planned_status(&compiled);
    wrong_count.component_count = 1;
    assert!(matches!(
        record_component_provisioning_observed(&preparing, wrong_count),
        Err(FleetComponentProvisioningInstallJournalError::InvalidDocument { .. })
    ));
}

fn compiled_plan(plan: &PersistedFleetInstallPlan) -> CompiledFleetComponentProvisioningPlan {
    let fleet_registry = registry_version(&plan.plan.fleet, 1, [8; 32]);
    let prepare_request = FleetComponentProvisioningPrepareRequest {
        operation_id: [4; 32],
        plan: FleetComponentProvisioningPlan {
            fleet: plan.plan.fleet.clone(),
            fleet_registry,
            configuration_digest: ComponentDeploymentConfigurationDigest::from_bytes([7; 32]),
            operation: FleetComponentProvisioningOperation::FreshInstall,
            directory_confirmation_roots: Vec::new(),
            batches: Vec::new(),
        },
    };
    CompiledFleetComponentProvisioningPlan {
        prepare_request,
        plan_hash: [5; 32],
    }
}

fn terminal_transaction(
    name: &str,
) -> (
    PersistedFleetInstallPlan,
    CompiledFleetComponentProvisioningPlan,
    ResolvedFleetComponentProvisioningInstall,
) {
    let (plan, compiled, preparing) = preparing_transaction(name);
    let prepared = record_component_provisioning_observed(&preparing, planned_status(&compiled))
        .expect("record prepared Coordinator plan");
    let advancing =
        begin_component_provisioning_advance(&prepared).expect("persist advance intent");
    let terminal = record_component_provisioning_advanced(&advancing, terminal_status(&compiled))
        .expect("record terminal runtime evidence");
    (plan, compiled, terminal)
}

fn preparing_transaction(
    name: &str,
) -> (
    PersistedFleetInstallPlan,
    CompiledFleetComponentProvisioningPlan,
    ResolvedFleetComponentProvisioningInstall,
) {
    let root = temp_dir(name);
    let plan = install_plan(&root);
    let compiled = compiled_plan(&plan);
    let planned =
        plan_fleet_component_provisioning_install(PlanFleetComponentProvisioningInstallRequest {
            fleet_install_plan: &plan,
            coordinator: principal(3),
            fleet_name: FleetName::try_from("main".to_string()).expect("Fleet name"),
            environment: "ic".to_string(),
            compiled: compiled.clone(),
        })
        .expect("plan host transaction");
    let preparing =
        begin_component_provisioning_preparation(&planned).expect("persist preparation intent");
    (plan, compiled, preparing)
}

fn planned_status(
    compiled: &CompiledFleetComponentProvisioningPlan,
) -> FleetComponentProvisioningStatusResponse {
    status(compiled, FleetComponentProvisioningPhase::Planned)
}

fn terminal_status(
    compiled: &CompiledFleetComponentProvisioningPlan,
) -> FleetComponentProvisioningStatusResponse {
    observed_status(compiled, FleetComponentProvisioningPhase::RuntimesActivated)
}

fn observed_status(
    compiled: &CompiledFleetComponentProvisioningPlan,
    phase: FleetComponentProvisioningPhase,
) -> FleetComponentProvisioningStatusResponse {
    let mut observed = status(compiled, phase);
    if matches!(
        phase,
        FleetComponentProvisioningPhase::RootsAccepted
            | FleetComponentProvisioningPhase::ProvisioningRoots
            | FleetComponentProvisioningPhase::ComponentsProvisioned
            | FleetComponentProvisioningPhase::ServiceTopologyPublished
            | FleetComponentProvisioningPhase::ConfirmingDirectories
            | FleetComponentProvisioningPhase::DirectoriesConfirmed
            | FleetComponentProvisioningPhase::ActivatingRuntimes
            | FleetComponentProvisioningPhase::RuntimesActivated
    ) {
        observed.roots_accepted_at_ns = Some(2);
    }
    if matches!(
        phase,
        FleetComponentProvisioningPhase::ComponentsProvisioned
            | FleetComponentProvisioningPhase::ServiceTopologyPublished
            | FleetComponentProvisioningPhase::ConfirmingDirectories
            | FleetComponentProvisioningPhase::DirectoriesConfirmed
            | FleetComponentProvisioningPhase::ActivatingRuntimes
            | FleetComponentProvisioningPhase::RuntimesActivated
    ) {
        observed.components_provisioned_at_ns = Some(3);
    }
    if matches!(
        phase,
        FleetComponentProvisioningPhase::ServiceTopologyPublished
            | FleetComponentProvisioningPhase::ConfirmingDirectories
            | FleetComponentProvisioningPhase::DirectoriesConfirmed
            | FleetComponentProvisioningPhase::ActivatingRuntimes
            | FleetComponentProvisioningPhase::RuntimesActivated
    ) {
        observed.published_fleet_registry =
            Some(compiled.prepare_request.plan.fleet_registry.clone());
        observed.service_topology_published_at_ns = Some(4);
    }
    if matches!(
        phase,
        FleetComponentProvisioningPhase::DirectoriesConfirmed
            | FleetComponentProvisioningPhase::ActivatingRuntimes
            | FleetComponentProvisioningPhase::RuntimesActivated
    ) {
        observed.directories_confirmed_at_ns = Some(5);
    }
    if phase == FleetComponentProvisioningPhase::RuntimesActivated {
        observed.runtimes_activated_at_ns = Some(6);
    }
    observed
}

fn status(
    compiled: &CompiledFleetComponentProvisioningPlan,
    phase: FleetComponentProvisioningPhase,
) -> FleetComponentProvisioningStatusResponse {
    FleetComponentProvisioningStatusResponse {
        operation_id: compiled.prepare_request.operation_id,
        plan_hash: compiled.plan_hash,
        fleet_registry: compiled.prepare_request.plan.fleet_registry.clone(),
        configuration_digest: compiled.prepare_request.plan.configuration_digest,
        operation: FleetComponentProvisioningOperation::FreshInstall,
        phase,
        directory_confirmation_root_count: 0,
        root_batch_count: 0,
        accepted_root_count: 0,
        acceptance_in_flight_root: None,
        provisioned_root_count: 0,
        current_root: None,
        provisioning_in_flight_root: None,
        directory_confirmed_root_count: 0,
        current_synchronization: None,
        current_publication: None,
        publication_in_flight_root: None,
        runtime_activated_root_count: 0,
        current_activation: None,
        activation_in_flight_root: None,
        pending_root_failure: None,
        group_placement_count: 0,
        component_count: 0,
        planned_at_ns: 1,
        roots_accepted_at_ns: None,
        components_provisioned_at_ns: None,
        published_fleet_registry: None,
        service_topology_published_at_ns: None,
        directories_confirmed_at_ns: None,
        runtimes_activated_at_ns: None,
    }
}

fn install_plan(root: &Path) -> PersistedFleetInstallPlan {
    let fleet = fleet();
    PersistedFleetInstallPlan {
        plan: FleetInstallPlan {
            fleet: fleet.clone(),
            fresh_fleet_plan_digest: "ab".repeat(32),
            release_build_id: ReleaseBuildId::from_nonce(ReleaseBuildNonce::from_random_bytes(
                [6; 32],
            )),
            application_artifact_union_digest: [9; 32],
            admission: crate::test_support::fleet_admission_policy(fleet),
            coordinator: PlannedFleetCoordinator {
                coordinator_subnet: subnet(1),
                placement_cost: crate::test_support::placement_cost(subnet(1)),
                creation_funding: PlannedCanisterCreationFunding::Cycles { cycles: 1 },
                root_funding: Some(crate::test_support::coordinator_root_funding_policy()),
            },
            fleet_subnet_roots: Vec::new(),
        },
        digest: [10; 32],
        path: root.join("plan.json"),
        root_release_sets: Vec::new(),
    }
}

fn committed_catalog(plan: &PersistedFleetInstallPlan, time: u64) -> CommittedFleetCatalog {
    CommittedFleetCatalog {
        entry: catalog_entry(plan, time),
        catalog_hash: [11; 32],
        advanced: true,
    }
}

fn catalog_entry(plan: &PersistedFleetInstallPlan, time: u64) -> FleetCatalogEntryV1 {
    FleetCatalogEntryV1 {
        canonical_network_id: plan.plan.fleet.fleet.canonical_network_id,
        fleet_id: plan.plan.fleet.fleet.fleet_id,
        fleet_name: FleetName::try_from("main".to_string()).expect("Fleet name"),
        app: plan.plan.fleet.app.clone(),
        environment: "ic".to_string(),
        deployed_at_unix_secs: time,
        release_build_id: plan.plan.release_build_id,
        coordinator_principal: principal(3).to_text(),
    }
}

fn registry_version(fleet: &FleetBinding, revision: u64, hash: [u8; 32]) -> FleetRegistryVersion {
    FleetRegistryVersion {
        authority: FleetRegistryAuthority {
            binding: FleetCoordinatorBinding {
                fleet: fleet.clone(),
                coordinator_subnet: subnet(1),
                coordinator: principal(3),
            },
            epoch: 1,
        },
        revision,
        content_hash: hash,
    }
}

fn fleet() -> FleetBinding {
    FleetBinding {
        fleet: FleetKey {
            canonical_network_id: CanonicalNetworkId::ic_mainnet(),
            fleet_id: FleetId::from_generated_bytes([2; 32]),
        },
        app: AppId::from("demo"),
    }
}

fn principal(byte: u8) -> Principal {
    Principal::from_slice(&[byte; 29])
}

fn subnet(byte: u8) -> SubnetId {
    SubnetId::from_principal(principal(byte))
}
