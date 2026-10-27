//! Individual protected Root calls for the host Component workflow.
//!
//! Selection reuses terminal Fleet inventory and its exact protocol sidecars.

use crate::{
    canister_protocol::{call_wire, query_wire_cli},
    component_operation::{
        ComponentOperationError,
        model::{ComponentAuthorityRecord, ComponentPlanRecord},
        ops::ComponentTransport,
        view::{ComponentObservation, ComponentProgressObservation},
    },
    fleet_ensure::{
        ops::{EnsurePaths, read_state},
        resolve_current_fleet,
    },
    icp::IcpCli,
    protocol_binding::resolve_release_registry_wire_schema,
};
use canic_contracts::{
    diagnostics::codes::STATE_UNAVAILABLE,
    dto::{
        component_registry::RootComponentAllocationRequest, fleet_registry::FleetSubnetRootStatus,
        pool::CanisterPoolStatusRequest, role::OperationReceipt,
        root::RootComponentOperationStatus,
    },
    ids::{ComponentSpecId, FleetSubnetRootBinding},
    protocol,
};
use ic_platform::principal::ExternalPrincipal as Principal;
use sha2_host::{Digest, Sha256};
use std::path::{Path, PathBuf};

/// ICP transport rooted in the same selected workspace and environment as its review.

pub struct IcpComponentTransport {
    root: PathBuf,
    icp: IcpCli,
}

impl IcpComponentTransport {
    /// Retain the operator-selected ICP context; this constructor performs no calls.
    #[must_use]
    pub fn new(root: &Path, icp: IcpCli) -> Self {
        Self {
            root: root.to_path_buf(),
            icp: icp.with_cwd(root),
        }
    }

    /// Resolve immutable authority from the current terminal Fleet and selected identity.
    pub fn authority(
        &self,
        environment: &str,
        fleet: &str,
        root_name: &str,
        spec: &ComponentSpecId,
    ) -> Result<ComponentAuthorityRecord, ComponentOperationError> {
        if self.icp.environment() != Some(environment) {
            return Err(ComponentOperationError::Authority {
                field: "ICP environment",
            });
        }
        let current = self.current_fleet(environment, fleet)?;
        let registry = current.initial_active_registry(fleet)?;
        let desired = current
            .plan
            .reviewed_desired
            .as_deref()
            .ok_or(ComponentOperationError::Integrity)?
            .desired();
        let configured = desired
            .canisters
            .iter()
            .find(|entry| entry.name == root_name)
            .ok_or(ComponentOperationError::Missing)?;
        let state = read_state(&EnsurePaths::under(&self.root, environment, fleet), fleet)?;
        let principal = state
            .principals
            .get(root_name)
            .ok_or(ComponentOperationError::Missing)?;
        let root = registry
            .fleet_subnet_roots
            .iter()
            .find(|entry| entry.fleet_subnet_root.to_text() == *principal)
            .ok_or(ComponentOperationError::Missing)?;
        if root.status != FleetSubnetRootStatus::Active {
            return Err(ComponentOperationError::Authority {
                field: "active Root",
            });
        }
        let selected = registry
            .component_specs
            .iter()
            .find(|entry| entry.component_spec == *spec)
            .ok_or(ComponentOperationError::Spec)?;
        if !root.component_admissions.iter().any(|entry| {
            entry.component_spec == *spec
                && entry.spec_hash == selected.spec_hash
                && entry.maximum_root_instances > 0
        }) {
            return Err(ComponentOperationError::Spec);
        }
        let entry = current
            .registry
            .entries
            .iter()
            .find(|entry| entry.pid == *principal)
            .ok_or(ComponentOperationError::Missing)?;
        let protocol = resolve_release_registry_wire_schema(
            &self.root,
            root.active_release_set.release_build_id,
            entry,
        )?;
        let mut controllers = configured.controllers.clone();
        for name in &configured.controller_canisters {
            controllers.push(
                state
                    .principals
                    .get(name)
                    .ok_or(ComponentOperationError::Missing)?
                    .clone(),
            );
        }
        controllers.sort();
        controllers.dedup();
        let operator = self.require_operator_controller(&controllers)?;
        Ok(ComponentAuthorityRecord {
            environment: environment.to_string(),
            fleet: fleet.to_string(),
            root_name: root_name.to_string(),
            source_plan_sha256: current.plan.plan_sha256.clone(),
            binding: FleetSubnetRootBinding {
                authority: registry.authority.clone(),
                placement_subnet: root.placement_subnet,
                fleet_subnet_root: root.fleet_subnet_root,
                component_admissions: root.component_admissions.clone(),
                component_topology_digest: root.component_topology_digest,
                limits: root.limits.clone(),
                funding: root.funding.clone(),
            },
            release_set: root.active_release_set,
            root_module_sha256: entry
                .module_hash
                .clone()
                .ok_or(ComponentOperationError::Integrity)?,
            root_wire_schema_sha256: protocol.schema_sha256,
            root_controllers: controllers,
            registry_sha256: Sha256::digest(serde_json::to_vec(registry)?).into(),
            operator,
            component_spec: spec.clone(),
            spec_hash: selected.spec_hash,
            role: selected.component_role.clone(),
        })
    }

    fn require_operator_controller(
        &self,
        controllers: &[String],
    ) -> Result<Principal, ComponentOperationError> {
        let operator = Principal::from_text(self.icp.identity_principal_text()?)
            .map_err(|_| ComponentOperationError::Authority { field: "operator" })?;
        if !controllers.contains(&operator.to_text()) {
            return Err(ComponentOperationError::Authority {
                field: "operator controller",
            });
        }
        Ok(operator)
    }

    fn current_fleet(
        &self,
        environment: &str,
        fleet: &str,
    ) -> Result<crate::fleet_ensure::CurrentFleetResolution, ComponentOperationError> {
        let current = resolve_current_fleet(&self.root, environment, fleet)?;
        if crate::fleet_ensure::policy::expected_plan_sha256(&current.plan)
            != current.plan.plan_sha256
        {
            return Err(ComponentOperationError::Integrity);
        }
        let registry = current.initial_active_registry(fleet)?;
        let network =
            crate::network::resolve_canonical_network_id_from_root(&self.root, environment)?;
        if registry.authority.binding.fleet.fleet.canonical_network_id != network {
            return Err(ComponentOperationError::Authority {
                field: "selected network",
            });
        }
        Ok(current)
    }

    fn binding(
        &self,
        plan: &ComponentPlanRecord,
        principal: Principal,
    ) -> Result<crate::release_set::BuiltWireSchema, ComponentOperationError> {
        let current = resolve_current_fleet(
            &self.root,
            &plan.authority.environment,
            &plan.authority.fleet,
        )?;
        let entry = current
            .registry
            .entries
            .iter()
            .find(|entry| entry.pid == principal.to_text())
            .ok_or(ComponentOperationError::Missing)?;
        let schema = resolve_release_registry_wire_schema(
            &self.root,
            plan.authority.release_set.release_build_id,
            entry,
        )?;
        if principal == plan.authority.binding.fleet_subnet_root
            && schema.schema_sha256 != plan.authority.root_wire_schema_sha256
        {
            return Err(ComponentOperationError::Authority {
                field: "Root Wire schema",
            });
        }
        Ok(schema)
    }

    fn query(
        &self,
        plan: &ComponentPlanRecord,
    ) -> Result<RootComponentOperationStatus, ComponentOperationError> {
        let principal = plan.authority.binding.fleet_subnet_root;
        let binding = self.binding(plan, principal)?;
        crate::artifact_io::require_wire_method(
            &binding.schema,
            protocol::component_read::allocation::METHOD,
            protocol::component_read::allocation::METHOD_ID,
            protocol::wire_declaration::MethodKind::Query,
            protocol::component_read::REQUEST_LIMITS,
            protocol::component_read::RESPONSE_LIMITS,
        )?;
        let status: canic_contracts::dto::component_read::ComponentAllocation = query_wire_cli(
            &self.icp,
            principal,
            protocol::component_read::allocation::METHOD,
            protocol::component_read::allocation::METHOD_ID,
            &plan.operation_id,
            protocol::component_read::REQUEST_LIMITS,
            protocol::component_read::RESPONSE_LIMITS,
        )?;
        Ok(status.into())
    }
}

impl ComponentTransport for IcpComponentTransport {
    fn observe(
        &mut self,
        plan: &ComponentPlanRecord,
    ) -> Result<ComponentObservation, ComponentOperationError> {
        let expected = &plan.authority;
        let authority = self.authority(
            &expected.environment,
            &expected.fleet,
            &expected.root_name,
            &expected.component_spec,
        )?;
        crate::component_operation::policy::validate_authority(expected, &authority)?;
        let principal = plan.authority.binding.fleet_subnet_root;
        let _binding = self.binding(plan, principal)?;
        let root = crate::canister_protocol::root_authority::query(&self.icp, principal)?;
        let matching_binding = root.binding == authority.binding;
        let matching_release = root.initial_release_set == authority.release_set;
        let matching_module = canic_core::cdk::utils::hash::hex_bytes(root.expected_module_hash)
            == authority.root_module_sha256;
        if !(matching_binding && matching_release && matching_module) {
            return Err(ComponentOperationError::Authority {
                field: "live Root binding",
            });
        }
        let coordinator = authority.binding.authority.binding.coordinator;
        // Validate the selected participant before using live registry evidence.
        self.binding(plan, coordinator)?;
        let registry = crate::registry_discovery::snapshot(&self.icp, coordinator)?;
        let registry_sha256: [u8; 32] = Sha256::digest(serde_json::to_vec(&registry)?).into();
        if registry_sha256 != authority.registry_sha256 {
            return Err(ComponentOperationError::Authority {
                field: "live Registry",
            });
        }
        let report = self
            .icp
            .canister_status_report(&authority.binding.fleet_subnet_root.to_text())?;
        let mut controllers = report
            .settings
            .ok_or(ComponentOperationError::Integrity)?
            .controllers;
        controllers.sort();
        let module = report
            .module_hash
            .map(|hash| hash.trim_start_matches("0x").to_ascii_lowercase());
        if report.id != authority.binding.fleet_subnet_root.to_text()
            || controllers != authority.root_controllers
            || module.as_ref() != Some(&authority.root_module_sha256)
        {
            return Err(ComponentOperationError::Authority {
                field: "Root management",
            });
        }
        let pool = crate::canister_protocol::pool_inventory::query(
            &self.icp,
            authority.binding.fleet_subnet_root,
            &CanisterPoolStatusRequest {
                start_after: None,
                limit: 1,
            },
        )?;
        Ok(ComponentObservation {
            authority,
            ready_assets: pool.ready,
        })
    }

    fn progress(
        &mut self,
        plan: &ComponentPlanRecord,
    ) -> Result<ComponentProgressObservation, ComponentOperationError> {
        let response = self.query(plan);
        let response = match response {
            Err(ComponentOperationError::Protocol(error))
                if error.is_rejected_with(STATE_UNAVAILABLE) =>
            {
                return Ok(ComponentProgressObservation { progress: None });
            }
            other => other?,
        };
        crate::component_operation::ops::project_progress(plan, response)
    }

    fn submit(&mut self, plan: &ComponentPlanRecord) -> Result<(), ComponentOperationError> {
        let principal = plan.authority.binding.fleet_subnet_root;
        let binding = self.binding(plan, principal)?;
        crate::artifact_io::require_wire_method(
            &binding.schema,
            protocol::component_control::provision::METHOD,
            protocol::component_control::provision::METHOD_ID,
            protocol::wire_declaration::MethodKind::Update,
            protocol::component_control::REQUEST_LIMITS,
            protocol::component_control::RESPONSE_LIMITS,
        )?;
        let receipt: OperationReceipt = call_wire(
            &self.icp,
            principal,
            protocol::component_control::provision::METHOD,
            protocol::component_control::provision::METHOD_ID,
            &RootComponentAllocationRequest {
                operation_id: plan.operation_id,
                component_spec: plan.authority.component_spec.clone(),
            },
            protocol::component_control::REQUEST_LIMITS,
            protocol::component_control::RESPONSE_LIMITS,
        )?;
        if receipt.operation_id != plan.operation_id {
            return Err(ComponentOperationError::Progress);
        }
        Ok(())
    }
}
