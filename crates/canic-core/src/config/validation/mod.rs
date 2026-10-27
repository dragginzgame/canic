//! Module: config::validation
//!
//! Responsibility: validate complete Canic configuration models on host/test targets.
//! Does not own: config schema definitions, runtime config storage, or endpoint DTOs.
//! Boundary: bootstrap calls validation before config models are installed.

mod app;
mod auth;
mod component_spec;

use crate::config::{
    provisioning_grants::{ProvisioningGrantGraph, ProvisioningGrantGraphError},
    schema::{
        ConfigModel, ConfigSchemaError, MAX_FLEET_COMPONENT_INSTANCES, RoleDeclarationKind,
        Validate, validate_canister_role_name,
    },
};
use canic_contracts::ids::CanisterRole;
use std::collections::BTreeMap;

fn validate_canister_role(
    role: &CanisterRole,
    context: &'static str,
) -> Result<(), ConfigSchemaError> {
    validate_canister_role_name(role.as_str()).map_err(|issue| {
        ConfigSchemaError::InvalidCanisterRoleName {
            context,
            role: role.to_string(),
            issue,
        }
    })
}

impl Validate for ConfigModel {
    fn validate(&self) -> Result<(), ConfigSchemaError> {
        // Validation order is intentional to surface the most meaningful
        // errors first and avoid cascaded failures.
        self.log.validate()?;
        self.auth.validate()?;
        self.app.validate()?;

        validate_role_declarations(self)?;
        validate_attestation_epoch_roles(self)?;

        for (role, declaration) in &self.roles {
            if let Some(policy) = &declaration.caller_authority {
                if !self
                    .component_specs
                    .values()
                    .any(|spec| spec.get_canister(role).is_some())
                {
                    return Err(
                        crate::config::caller_authority::CallerPolicyError::UnsupportedReceiver
                            .into(),
                    );
                }
                crate::config::caller_authority::validate_sources(self, policy)?;
            }
        }

        if self.component_specs.is_empty() {
            return Ok(());
        }

        let mut maximum_component_instances = 0_u32;
        let mut component_owners = BTreeMap::<CanisterRole, String>::new();
        for (component_spec_id, component_spec) in &self.component_specs {
            component_spec.validate()?;

            maximum_component_instances = maximum_component_instances
                .checked_add(component_spec.maximum_instances)
                .ok_or_else(|| {
                    ConfigSchemaError::ValidationError(
                        "Component Spec maximum_instances sum overflowed".into(),
                    )
                })?;

            if let Some(existing) = component_owners.insert(
                component_spec.component_role.clone(),
                component_spec_id.to_string(),
            ) {
                return Err(ConfigSchemaError::ValidationError(format!(
                    "Component role '{}' occurs in both Component Specs '{existing}' and '{component_spec_id}'",
                    component_spec.component_role,
                )));
            }
        }
        if maximum_component_instances > MAX_FLEET_COMPONENT_INSTANCES {
            return Err(ConfigSchemaError::ValidationError(format!(
                "Component Spec maximum_instances sum {maximum_component_instances} exceeds Fleet bound {MAX_FLEET_COMPONENT_INSTANCES}",
            )));
        }

        validate_component_provisioning_grants(self)?;
        validate_topology_roles_are_declared(self)?;

        Ok(())
    }
}

fn validate_attestation_epoch_roles(config: &ConfigModel) -> Result<(), ConfigSchemaError> {
    for role in config
        .auth
        .role_attestation
        .min_accepted_epoch_by_role
        .keys()
    {
        let role = CanisterRole::owned(role.clone());
        let infrastructure = role.is_root() || role.is_fleet_coordinator() || role.is_wasm_store();
        if !infrastructure && !config.roles.contains_key(&role) {
            return Err(ConfigSchemaError::ValidationError(format!(
                "auth.role_attestation.min_accepted_epoch_by_role references undeclared role '{role}'"
            )));
        }
    }
    Ok(())
}

fn validate_component_provisioning_grants(config: &ConfigModel) -> Result<(), ConfigSchemaError> {
    let mut graph = ProvisioningGrantGraph::new(config.component_specs.keys());
    for (requester, source) in &config.component_specs {
        for target in source.provisions.keys() {
            if requester == target {
                return Err(ConfigSchemaError::ValidationError(format!(
                    "Component Spec '{requester}' cannot provision itself",
                )));
            }
            graph.add_grant(requester, target).map_err(|error| {
                let message = match error {
                    ProvisioningGrantGraphError::UnknownTarget => format!(
                        "Component Spec '{requester}' provisioning grant references unknown target '{target}'",
                    ),
                    ProvisioningGrantGraphError::IncomingCountOverflow =>
                        "Component provisioning grant count overflowed".into(),
                };
                ConfigSchemaError::ValidationError(message)
            })?;
        }
    }
    if let Some(component_spec) = graph.cycle_blocked_spec() {
        return Err(ConfigSchemaError::ValidationError(format!(
            "Component provisioning grants contain a cycle involving Spec '{component_spec}'",
        )));
    }
    Ok(())
}

fn validate_role_declarations(config: &ConfigModel) -> Result<(), ConfigSchemaError> {
    if config.roles.is_empty() {
        return Err(ConfigSchemaError::ValidationError(
            "role declarations are required; add [roles.<role>] entries".into(),
        ));
    }

    for (role, declaration) in &config.roles {
        validate_canister_role(role, "role declaration")?;

        if role.is_fleet_coordinator() {
            return Err(ConfigSchemaError::ValidationError(
                "role declaration 'fleet_coordinator' is reserved Canic infrastructure".into(),
            ));
        }

        if role.is_root() && declaration.kind != RoleDeclarationKind::Root {
            return Err(ConfigSchemaError::ValidationError(
                "role declaration 'root' must have kind = \"root\"".into(),
            ));
        }

        if !role.is_root() && declaration.kind == RoleDeclarationKind::Root {
            return Err(ConfigSchemaError::ValidationError(format!(
                "role declaration '{role}' cannot have kind = \"root\"",
            )));
        }

        if declaration.kind == RoleDeclarationKind::Root && declaration.fleet_admission {
            return Err(ConfigSchemaError::ValidationError(format!(
                "role declaration '{role}' cannot enable fleet_admission because Roots distribute admission but are not participants",
            )));
        }

        if declaration.kind == RoleDeclarationKind::Root {
            if declaration.package.is_some() {
                return Err(ConfigSchemaError::ValidationError(
                    "Root uses the canonical Canic package; omit roles.root.package".into(),
                ));
            }
        } else if declaration
            .package
            .as_deref()
            .is_none_or(|package| package.trim().is_empty())
        {
            return Err(ConfigSchemaError::ValidationError(format!(
                "role declaration '{role}' package must not be empty",
            )));
        }
    }

    if !config.component_specs.is_empty() && !config.roles.contains_key(&CanisterRole::ROOT) {
        return Err(ConfigSchemaError::ValidationError(
            "root role declaration missing; add [roles.root] kind = \"root\"".into(),
        ));
    }

    Ok(())
}

fn validate_topology_roles_are_declared(config: &ConfigModel) -> Result<(), ConfigSchemaError> {
    let attached_roles = config.attached_roles();

    for role in &attached_roles {
        if !config.roles.contains_key(role) {
            let display = config.app_role_ref(role).to_string();
            return Err(ConfigSchemaError::ValidationError(format!(
                "topology role '{display}' is not declared; add [roles.{role}]",
            )));
        }
    }

    for role in attached_roles {
        let declaration = &config.roles[&role];
        if declaration.kind != RoleDeclarationKind::Canister {
            return Err(ConfigSchemaError::ValidationError(format!(
                "Component topology role '{role}' must use [roles.{role}] kind = \"canister\"",
            )));
        }
    }

    Ok(())
}
