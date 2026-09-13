//! Host journal ordering, review rejection and monotonic observation regressions.
//!
//! These native transport probes do not claim IC lifecycle execution evidence.

use super::{
    ComponentOperationError,
    model::*,
    ops::{self, ComponentTransport},
    policy,
    view::*,
    workflow,
};
use crate::test_support::{fleet_subnet_root_funding_authority, temp_dir};
use candid::Principal;
use canic_core::{
    cdk::types::Cycles,
    ids::{
        AppId, CanonicalNetworkId, ComponentBinding, ComponentInstanceId, ComponentSpecAdmission,
        ComponentTopologyDigest, CyclesFundingBudget, FleetBinding, FleetCoordinatorBinding,
        FleetId, FleetKey, FleetRegistryAuthority, FleetSubnetCanisterPoolConfig,
        FleetSubnetRootBinding, FleetSubnetRootLimits, FleetSubnetRootReleaseSet, ReleaseBuildId,
        ReleaseSetDigest, SubnetId,
    },
};
use std::{io, path::PathBuf};

fn authority() -> ComponentAuthorityRecord {
    let operator = Principal::from_slice(&[1]);
    let component_spec: canic_core::ids::ComponentSpecId = "core".parse().unwrap();
    ComponentAuthorityRecord {
        environment: "local".into(),
        fleet: "demo".into(),
        root_name: "root".into(),
        source_plan_sha256: "11".repeat(32),
        binding: FleetSubnetRootBinding {
            authority: FleetRegistryAuthority {
                binding: FleetCoordinatorBinding {
                    fleet: FleetBinding {
                        fleet: FleetKey {
                            canonical_network_id: CanonicalNetworkId::ic_mainnet(),
                            fleet_id: FleetId::from_generated_bytes([1; 32]),
                        },
                        app: AppId::from("demo"),
                    },
                    coordinator_subnet: SubnetId::from_principal(Principal::from_slice(&[2])),
                    coordinator: Principal::from_slice(&[3]),
                },
                epoch: 1,
            },
            placement_subnet: SubnetId::from_principal(Principal::from_slice(&[4])),
            fleet_subnet_root: Principal::from_slice(&[5]),
            component_admissions: vec![ComponentSpecAdmission {
                component_spec: component_spec.clone(),
                spec_hash: [3; 32],
                maximum_root_instances: 2,
            }],
            component_topology_digest: ComponentTopologyDigest::from_bytes([2; 32]),
            limits: FleetSubnetRootLimits {
                maximum_component_instances: 2,
                maximum_registry_bytes: 65_536,
                maximum_wasm_store_bytes: 1_048_576,
                maximum_group_placements: 2,
                canister_pool: FleetSubnetCanisterPoolConfig {
                    minimum_size: 1,
                    maximum_size: 4,
                    canister_cycles: Cycles::new(1_000_000_000_000),
                    creation_execution_margin: Cycles::new(1_000_000_000),
                },
                cycles_funding: CyclesFundingBudget {
                    window_secs: 3600,
                    maximum_cycles: Cycles::new(10_000_000_000_000),
                },
            },
            funding: fleet_subnet_root_funding_authority(),
        },
        release_set: FleetSubnetRootReleaseSet {
            release_build_id: ReleaseBuildId::from_nonce(
                canic_core::ids::ReleaseBuildNonce::from_random_bytes([4; 32]),
            ),
            manifest_digest: ReleaseSetDigest::from_bytes([5; 32]),
        },
        root_module_sha256: "66".repeat(32),
        root_candid_sha256: [7; 32],
        root_controllers: vec![operator.to_text()],
        registry_sha256: [8; 32],
        operator,
        component_spec,
        spec_hash: [3; 32],
        role: "core".into(),
    }
}

fn progress(authority: &ComponentAuthorityRecord) -> ComponentProgressRecord {
    let component = ComponentInstanceId::from_root_allocation(
        authority.binding.authority.binding.fleet.fleet,
        authority.binding.authority.epoch,
        authority.binding.fleet_subnet_root,
        1,
    );
    ComponentProgressRecord {
        allocation_sequence: 1,
        component,
        phase: ComponentPhase::Committed,
        complete: true,
        binding: Some(ComponentBinding {
            authority: authority.binding.authority.clone(),
            component,
            component_spec: authority.component_spec.clone(),
            spec_hash: authority.spec_hash,
            role: authority.role.clone(),
            placement_subnet: authority.binding.placement_subnet,
            fleet_subnet_root: authority.binding.fleet_subnet_root,
            canister_id: Principal::from_slice(&[6]),
        }),
    }
}

struct TransportProbe {
    directory: PathBuf,
    observation: ComponentObservation,
    progress: Option<ComponentProgressRecord>,
    submissions: Vec<[u8; 32]>,
    fail_submit: bool,
    fail_read: bool,
}

impl TransportProbe {
    fn new() -> Self {
        Self {
            directory: temp_dir("canic-component-operation"),
            observation: ComponentObservation {
                authority: authority(),
                ready_assets: 1,
            },
            progress: None,
            submissions: vec![],
            fail_submit: false,
            fail_read: false,
        }
    }
    fn plan(&mut self) -> ComponentOperationRecord {
        workflow::plan(&self.directory.clone(), "core", authority(), self).unwrap()
    }
    fn apply(&mut self, review: &str) -> Result<ComponentOperationRecord, ComponentOperationError> {
        workflow::apply(
            &self.directory.clone(),
            "local",
            "demo",
            "core",
            review,
            self,
        )
    }
}

impl Drop for TransportProbe {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.directory);
    }
}

impl ComponentTransport for TransportProbe {
    fn observe(
        &mut self,
        _: &ComponentPlanRecord,
    ) -> Result<ComponentObservation, ComponentOperationError> {
        if self.fail_read {
            return Err(io::Error::from(io::ErrorKind::PermissionDenied).into());
        }
        Ok(self.observation.clone())
    }
    fn progress(
        &mut self,
        _: &ComponentPlanRecord,
    ) -> Result<ComponentProgressObservation, ComponentOperationError> {
        Ok(ComponentProgressObservation {
            progress: self.progress.clone(),
        })
    }
    fn submit(&mut self, plan: &ComponentPlanRecord) -> Result<(), ComponentOperationError> {
        let path = ops::record_path(&self.directory, "local", "demo", "core")?;
        let intent = ops::read(&path)?.unwrap();
        assert_eq!(intent.plan, *plan);
        assert_eq!(
            intent.submission_attempts,
            self.submissions.len() as u64 + 1
        );
        self.submissions.push(plan.operation_id);
        if self.fail_submit {
            return Err(io::Error::from(io::ErrorKind::ConnectionReset).into());
        }
        Ok(())
    }
}

#[test]
fn lost_response_keeps_intent_and_reconciles_without_a_second_submission() {
    let mut transport = TransportProbe::new();
    let plan = transport.plan();
    assert_eq!(transport.plan(), plan);
    assert!(transport.submissions.is_empty());
    transport.fail_submit = true;
    assert!(matches!(
        transport.apply(&plan.plan.review_sha256),
        Err(ComponentOperationError::Io(_))
    ));
    transport.progress = Some(progress(&authority()));
    transport.observation.ready_assets = 0;
    let completed = transport.apply(&plan.plan.review_sha256).unwrap();
    assert!(completed.progress.as_ref().unwrap().complete);
    transport.fail_read = true;
    assert_eq!(
        transport.apply(&plan.plan.review_sha256).unwrap(),
        completed
    );
    assert_eq!(transport.submissions, vec![plan.plan.operation_id]);
}

#[test]
fn absent_remote_intent_resubmits_only_the_same_retained_identity() {
    let mut transport = TransportProbe::new();
    let plan = transport.plan();
    transport.fail_submit = true;
    assert!(transport.apply(&plan.plan.review_sha256).is_err());
    transport.fail_submit = false;
    transport.apply(&plan.plan.review_sha256).unwrap();
    assert_eq!(transport.submissions, vec![plan.plan.operation_id; 2]);
}

#[test]
fn capacity_denial_and_review_conflict_do_not_reserve_a_submission() {
    let mut transport = TransportProbe::new();
    let plan = transport.plan();
    assert!(matches!(
        transport.apply("wrong"),
        Err(ComponentOperationError::Review)
    ));
    transport.observation.ready_assets = 0;
    assert!(matches!(
        transport.apply(&plan.plan.review_sha256),
        Err(ComponentOperationError::Capacity)
    ));
    transport.observation.ready_assets = 1;
    transport.fail_read = true;
    assert!(matches!(
        transport.apply(&plan.plan.review_sha256),
        Err(ComponentOperationError::Io(_))
    ));
    assert!(transport.submissions.is_empty());
    let path = ops::record_path(&transport.directory, "local", "demo", "core").unwrap();
    assert_eq!(ops::read(&path).unwrap().unwrap(), plan);
}

#[test]
fn release_spec_and_controller_drift_reject_the_existing_review() {
    let original = authority();
    let mut variants = vec![original; 4];
    variants[0].source_plan_sha256 = "22".repeat(32);
    variants[1].spec_hash = [44; 32];
    variants[2].operator = Principal::from_slice(&[77]);
    variants[3]
        .root_controllers
        .push(Principal::from_slice(&[78]).to_text());
    for changed in variants {
        let mut transport = TransportProbe::new();
        let plan = transport.plan();
        transport.observation.authority = changed;
        assert!(matches!(
            transport.apply(&plan.plan.review_sha256),
            Err(ComponentOperationError::Authority { .. })
        ));
        assert!(transport.submissions.is_empty());
    }
}

#[test]
fn intermediate_progress_is_monotonic_and_bound_to_allocation_and_canister() {
    let authority = authority();
    let complete = progress(&authority);
    let mut intermediate = complete.clone();
    intermediate.complete = false;
    intermediate.phase = ComponentPhase::Installed;
    assert!(policy::validate_progress(&authority, Some(&intermediate), &complete).is_ok());
    assert!(matches!(
        policy::validate_progress(&authority, Some(&complete), &intermediate),
        Err(ComponentOperationError::Progress)
    ));
    let mut changes = vec![complete; 4];
    changes[0].allocation_sequence += 1;
    changes[1].binding = None;
    changes[2].binding.as_mut().unwrap().fleet_subnet_root = Principal::anonymous();
    changes[3].binding.as_mut().unwrap().canister_id = Principal::from_slice(&[7]);
    for changed in changes {
        assert!(matches!(
            policy::validate_progress(&authority, Some(&intermediate), &changed),
            Err(ComponentOperationError::Progress)
        ));
    }
}

#[test]
fn status_never_submits_and_unrelated_remote_progress_is_rejected() {
    let mut transport = TransportProbe::new();
    let plan = transport.plan();
    let root = transport.directory.clone();
    assert_eq!(
        workflow::status(&root, "local", "demo", "core", &mut transport).unwrap(),
        plan
    );
    transport.progress = Some(progress(&authority()));
    assert!(matches!(
        transport.apply(&plan.plan.review_sha256),
        Err(ComponentOperationError::Progress)
    ));
    assert!(transport.submissions.is_empty());
}

#[test]
fn modified_document_and_path_escape_are_rejected() {
    let mut transport = TransportProbe::new();
    let plan = transport.plan();
    let path = ops::record_path(&transport.directory, "local", "demo", "core").unwrap();
    let mut changed = plan;
    changed.submission_attempts = 100;
    std::fs::write(&path, serde_json::to_vec(&changed).unwrap()).unwrap();
    assert!(matches!(
        ops::read(&path),
        Err(ComponentOperationError::Integrity)
    ));
    for name in ["../core", "", "/tmp/core", "a.b"] {
        assert!(matches!(
            ops::record_path(&transport.directory, "local", "demo", name),
            Err(ComponentOperationError::InvalidLabel(_))
        ));
    }
}

#[test]
fn retained_incomplete_allocation_is_resumed_even_when_the_pool_is_empty() {
    let mut transport = TransportProbe::new();
    let plan = transport.plan();
    transport.apply(&plan.plan.review_sha256).unwrap();
    let mut pending = progress(&authority());
    pending.phase = ComponentPhase::Installed;
    pending.complete = false;
    transport.progress = Some(pending.clone());
    transport.observation.ready_assets = 0;
    let resumed = transport.apply(&plan.plan.review_sha256).unwrap();
    assert_eq!(resumed.progress, Some(pending));
    assert_eq!(transport.submissions, vec![plan.plan.operation_id; 2]);
    let root = transport.directory.clone();
    workflow::status(&root, "local", "demo", "core", &mut transport).unwrap();
    assert_eq!(transport.submissions.len(), 2);
}

#[test]
fn remote_progress_must_match_the_exact_request_caller_and_release() {
    use canic_control_plane::dto::root::RootComponentOperationStatus;
    use canic_core::dto::component_registry::{
        ComponentProvisioningOrigin, RootComponentAllocationPhase, RootComponentAllocationResponse,
    };
    let record = ops::new_record("core", authority()).unwrap();
    let binding = &record.plan.authority;
    let allocation = RootComponentAllocationResponse {
        operation_id: record.plan.operation_id,
        allocation_sequence: 1,
        component: progress(binding).component,
        component_spec: binding.component_spec.clone(),
        spec_hash: binding.spec_hash,
        role: binding.role.clone(),
        provisioning_origin: ComponentProvisioningOrigin::FleetAdministrator {
            caller: binding.operator,
        },
        release_set: binding.release_set,
        phase: RootComponentAllocationPhase::Reserved,
        creation: None,
        installation: None,
    };
    let status = RootComponentOperationStatus {
        allocation,
        complete: false,
    };
    assert!(ops::project_progress(&record.plan, status.clone()).is_ok());
    let mut conflicts = vec![status; 4];
    conflicts[0].allocation.operation_id = [0; 32];
    conflicts[1].allocation.spec_hash = [9; 32];
    conflicts[2].allocation.release_set.manifest_digest = ReleaseSetDigest::from_bytes([9; 32]);
    conflicts[3].allocation.provisioning_origin = ComponentProvisioningOrigin::FleetAdministrator {
        caller: Principal::anonymous(),
    };
    for conflict in conflicts {
        assert!(matches!(
            ops::project_progress(&record.plan, conflict),
            Err(ComponentOperationError::Progress)
        ));
    }
}

#[test]
fn transport_rejects_a_different_environment_before_discovery_or_calls() {
    let transport = ops::transport::IcpComponentTransport::new(
        std::path::Path::new("/must-not-be-opened"),
        crate::icp::IcpCli::new("must-not-run", Some("academic".to_string())),
    );
    assert!(matches!(
        transport.authority("local", "demo", "root", &"core".parse().unwrap()),
        Err(ComponentOperationError::Authority {
            field: "ICP environment"
        })
    ));
}
