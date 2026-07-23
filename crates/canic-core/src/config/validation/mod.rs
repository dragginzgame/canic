//! Module: config::validation
//!
//! Responsibility: validate complete Canic configuration models on host/test targets.
//! Does not own: config schema definitions, runtime config storage, or endpoint DTOs.
//! Boundary: bootstrap calls validation before config models are installed.

mod app;
mod auth;
mod subnet;

use crate::{
    config::schema::{
        CanisterKind, ConfigModel, ConfigSchemaError, NAME_MAX_BYTES, RoleDeclarationKind,
        Validate, validate_canister_role_name,
    },
    ids::{CanisterRole, SubnetSlotId},
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

fn validate_subnet_slot_id_len(
    slot: &SubnetSlotId,
    context: &str,
) -> Result<(), ConfigSchemaError> {
    if slot.as_ref().len() > NAME_MAX_BYTES {
        return Err(ConfigSchemaError::ValidationError(format!(
            "{context} '{slot}' exceeds {NAME_MAX_BYTES} bytes",
        )));
    }
    Ok(())
}

impl Validate for ConfigModel {
    fn validate(&self) -> Result<(), ConfigSchemaError> {
        // Validation order is intentional to surface the most meaningful
        // errors first and avoid cascaded failures.
        for subnet_slot in self.subnets.keys() {
            validate_subnet_slot_id_len(subnet_slot, "Subnet Slot")?;
        }

        self.log.validate()?;
        self.auth.validate()?;
        self.app.validate()?;

        validate_role_declarations(self)?;

        if self.subnets.is_empty() {
            if self.roles.contains_key(&CanisterRole::ROOT) {
                return Err(ConfigSchemaError::ValidationError(
                    "topology-less configs cannot declare role 'root'".into(),
                ));
            }
            if !self.services.fleet.roles.is_empty() {
                return Err(ConfigSchemaError::ValidationError(
                    "topology-less configs cannot define services.fleet.roles entries".into(),
                ));
            }
            return Ok(());
        }

        let default_slot = SubnetSlotId::DEFAULT;
        let default_subnet = self.subnets.get(&default_slot).ok_or_else(|| {
            ConfigSchemaError::ValidationError("default Subnet Slot not found".into())
        })?;

        let root_role = CanisterRole::ROOT;
        let root_cfg = default_subnet.canisters.get(&root_role).ok_or_else(|| {
            ConfigSchemaError::ValidationError(
                "root canister not defined in default Subnet Slot".into(),
            )
        })?;

        if root_cfg.kind != CanisterKind::Root {
            return Err(ConfigSchemaError::ValidationError(
                "root canister must have kind = \"root\"".into(),
            ));
        }

        for canister_role in &self.services.fleet.roles {
            validate_canister_role(canister_role, "Fleet service role")?;

            let canister_cfg = default_subnet.canisters.get(canister_role).ok_or_else(|| {
                ConfigSchemaError::ValidationError(format!(
                    "Fleet service role '{canister_role}' is not in default Subnet Slot",
                ))
            })?;

            if canister_cfg.kind != CanisterKind::Service {
                return Err(ConfigSchemaError::ValidationError(format!(
                    "Fleet service role '{canister_role}' must have kind = \"service\"",
                )));
            }
        }

        let mut root_roles = Vec::new();
        for (subnet_slot, subnet) in &self.subnets {
            for (canister_role, canister_cfg) in &subnet.canisters {
                if canister_cfg.kind == CanisterKind::Root {
                    root_roles.push(format!("{subnet_slot}:{canister_role}"));
                }
            }
        }

        if root_roles.len() > 1 {
            return Err(ConfigSchemaError::ValidationError(format!(
                "root kind must be unique globally (found {})",
                root_roles.join(", "),
            )));
        }

        for subnet in self.subnets.values() {
            subnet.validate()?;
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

    if !config.subnets.is_empty() && !config.roles.contains_key(&CanisterRole::ROOT) {
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

    for subnet in config.subnets.values() {
        for (role, canister) in &subnet.canisters {
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
