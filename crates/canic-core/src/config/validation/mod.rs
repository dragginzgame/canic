//! Module: config::validation
//!
//! Responsibility: validate complete Canic configuration models on host/test targets.
//! Does not own: config schema definitions, runtime config storage, or endpoint DTOs.
//! Boundary: bootstrap calls validation before config models are installed.

mod app;
mod auth;
mod component_spec;

use crate::{
    config::schema::{
        ConfigModel, ConfigSchemaError, MAX_FLEET_COMPONENT_INSTANCES, RoleDeclarationKind,
        Validate, validate_canister_role_name,
    },
    ids::CanisterRole,
};
use std::collections::{BTreeMap, BTreeSet};

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

        if self.component_specs.is_empty() {
            return Ok(());
        }

        let mut maximum_component_instances = 0_u32;
        let mut component_owners = BTreeMap::<CanisterRole, String>::new();
        let mut child_roles = BTreeSet::<CanisterRole>::new();
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
            child_roles.extend(component_spec.children.keys().cloned());
        }
        if let Some(role) = child_roles
            .iter()
            .find(|role| component_owners.contains_key(*role))
        {
            return Err(ConfigSchemaError::ValidationError(format!(
                "role '{role}' cannot be both a Component and a Component Child",
            )));
        }
        if maximum_component_instances > MAX_FLEET_COMPONENT_INSTANCES {
            return Err(ConfigSchemaError::ValidationError(format!(
                "Component Spec maximum_instances sum {maximum_component_instances} exceeds Fleet bound {MAX_FLEET_COMPONENT_INSTANCES}",
            )));
        }

        validate_topology_roles_are_declared(self)?;

        Ok(())
    }
}

fn validate_role_declarations(config: &ConfigModel) -> Result<(), ConfigSchemaError> {
    if config.roles.is_empty() {
        return Err(ConfigSchemaError::ValidationError(
            "role declarations are required; add [roles.<role>] entries".into(),
        ));
    }

    for (role, declaration) in &config.roles {
        validate_canister_role(role, "role declaration")?;

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

        if declaration.package.trim().is_empty() {
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
