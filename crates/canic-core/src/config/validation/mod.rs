//! Module: config::validation
//!
//! Responsibility: validate complete Canic configuration models on host/test targets.
//! Does not own: config schema definitions, runtime config storage, or endpoint DTOs.
//! Boundary: bootstrap calls validation before config models are installed.

mod app;
mod auth;
mod tree_spec;

use crate::{
    config::schema::{
        CanisterKind, ConfigModel, ConfigSchemaError, MAX_FLEET_TREES, RoleDeclarationKind,
        Validate, validate_canister_role_name,
    },
    ids::CanisterRole,
};

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

        if self.tree_specs.is_empty() && self.tree_groups.is_empty() {
            if self.roles.contains_key(&CanisterRole::ROOT) {
                return Err(ConfigSchemaError::ValidationError(
                    "topology-less configs cannot declare role 'root'".into(),
                ));
            }
            return Ok(());
        }

        if self.tree_specs.is_empty() {
            return Err(ConfigSchemaError::ValidationError(
                "Tree Groups require at least one [tree_specs.<id>] declaration".into(),
            ));
        }
        if self.tree_groups.is_empty() {
            return Err(ConfigSchemaError::ValidationError(
                "Tree Specs require at least one [tree_groups.<id>] declaration".into(),
            ));
        }

        for (tree_spec_id, tree_spec) in &self.tree_specs {
            let roots = tree_spec
                .canisters
                .iter()
                .filter(|(_role, canister)| canister.kind == CanisterKind::Root)
                .map(|(role, _canister)| role.to_string())
                .collect::<Vec<_>>();
            if roots.len() != 1 {
                return Err(ConfigSchemaError::ValidationError(format!(
                    "Tree Spec '{tree_spec_id}' must contain exactly one root role (found {})",
                    roots.len(),
                )));
            }

            tree_spec.validate()?;
        }

        let mut maximum_trees = 0_u32;
        for (tree_group_id, tree_group) in &self.tree_groups {
            if !self.tree_specs.contains_key(&tree_group.tree_spec) {
                return Err(ConfigSchemaError::ValidationError(format!(
                    "Tree Group '{tree_group_id}' references unknown Tree Spec '{}'",
                    tree_group.tree_spec,
                )));
            }
            if tree_group.initial_trees == 0 {
                return Err(ConfigSchemaError::ValidationError(format!(
                    "Tree Group '{tree_group_id}' initial_trees must be > 0",
                )));
            }
            if tree_group.maximum_trees == 0 {
                return Err(ConfigSchemaError::ValidationError(format!(
                    "Tree Group '{tree_group_id}' maximum_trees must be > 0",
                )));
            }
            if tree_group.initial_trees > tree_group.maximum_trees {
                return Err(ConfigSchemaError::ValidationError(format!(
                    "Tree Group '{tree_group_id}' initial_trees must be <= maximum_trees",
                )));
            }

            maximum_trees = maximum_trees
                .checked_add(u32::from(tree_group.maximum_trees))
                .ok_or_else(|| {
                    ConfigSchemaError::ValidationError(
                        "Tree Group maximum_trees sum overflowed".into(),
                    )
                })?;
        }
        if maximum_trees > MAX_FLEET_TREES {
            return Err(ConfigSchemaError::ValidationError(format!(
                "Tree Group maximum_trees sum {maximum_trees} exceeds Fleet bound {MAX_FLEET_TREES}",
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

    if !config.tree_specs.is_empty() && !config.roles.contains_key(&CanisterRole::ROOT) {
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

    for (role, declaration) in &config.roles {
        if declaration.kind == RoleDeclarationKind::Root && !attached_roles.contains(role) {
            return Err(ConfigSchemaError::ValidationError(format!(
                "root role declaration '{role}' must be attached to topology",
            )));
        }
    }

    for tree_spec in config.tree_specs.values() {
        for (role, canister) in &tree_spec.canisters {
            let declaration = config.roles.get(role).ok_or_else(|| {
                ConfigSchemaError::ValidationError(format!(
                    "topology role '{role}' is not declared; add [roles.{role}]",
                ))
            })?;

            if canister.kind == CanisterKind::Root && declaration.kind != RoleDeclarationKind::Root
            {
                return Err(ConfigSchemaError::ValidationError(format!(
                    "topology role '{role}' has kind = \"root\" but [roles.{role}] is not kind = \"root\"",
                )));
            }
        }
    }

    Ok(())
}
