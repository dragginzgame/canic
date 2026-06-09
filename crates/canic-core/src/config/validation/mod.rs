mod app;
mod auth;
mod fleet;
mod subnet;

use crate::{
    config::schema::{
        CanisterKind, ConfigModel, ConfigSchemaError, NAME_MAX_BYTES, RoleDeclarationKind, Validate,
    },
    ids::{CanisterRole, SubnetRole},
};

fn validate_canister_role_len(role: &CanisterRole, context: &str) -> Result<(), ConfigSchemaError> {
    if role.as_ref().len() > NAME_MAX_BYTES {
        return Err(ConfigSchemaError::ValidationError(format!(
            "{context} '{role}' exceeds {NAME_MAX_BYTES} bytes",
        )));
    }
    Ok(())
}

fn validate_subnet_role_len(role: &SubnetRole, context: &str) -> Result<(), ConfigSchemaError> {
    if role.as_ref().len() > NAME_MAX_BYTES {
        return Err(ConfigSchemaError::ValidationError(format!(
            "{context} '{role}' exceeds {NAME_MAX_BYTES} bytes",
        )));
    }
    Ok(())
}

impl Validate for ConfigModel {
    fn validate(&self) -> Result<(), ConfigSchemaError> {
        // Validation order is intentional to surface the most meaningful
        // errors first and avoid cascaded failures.
        for subnet_role in self.subnets.keys() {
            validate_subnet_role_len(subnet_role, "subnet")?;
        }

        self.log.validate()?;
        self.auth.validate()?;
        self.app.validate()?;
        let fleet = self.fleet.as_ref().ok_or_else(|| {
            ConfigSchemaError::ValidationError(
                "fleet config is required; add [fleet] name = \"<fleet>\"".into(),
            )
        })?;
        fleet.validate()?;
        if fleet.name.is_none() {
            return Err(ConfigSchemaError::ValidationError(
                "fleet name is required; add [fleet] name = \"<fleet>\"".into(),
            ));
        }

        validate_role_declarations(self)?;

        if self.subnets.is_empty() {
            if self.roles.contains_key(&CanisterRole::ROOT) {
                return Err(ConfigSchemaError::ValidationError(
                    "topology-less configs cannot declare role 'root'".into(),
                ));
            }
            if !self.app_index.is_empty() {
                return Err(ConfigSchemaError::ValidationError(
                    "topology-less configs cannot define app_index entries".into(),
                ));
            }
            return Ok(());
        }

        let prime = SubnetRole::PRIME;
        let prime_subnet = self
            .subnets
            .get(&prime)
            .ok_or_else(|| ConfigSchemaError::ValidationError("prime subnet not found".into()))?;

        let root_role = CanisterRole::ROOT;
        let root_cfg = prime_subnet.canisters.get(&root_role).ok_or_else(|| {
            ConfigSchemaError::ValidationError("root canister not defined in prime subnet".into())
        })?;

        if root_cfg.kind != CanisterKind::Root {
            return Err(ConfigSchemaError::ValidationError(
                "root canister must have kind = \"root\"".into(),
            ));
        }

        for canister_role in &self.app_index {
            validate_canister_role_len(canister_role, "app index canister")?;

            let canister_cfg = prime_subnet.canisters.get(canister_role).ok_or_else(|| {
                ConfigSchemaError::ValidationError(format!(
                    "app index canister '{canister_role}' is not in prime subnet",
                ))
            })?;

            if canister_cfg.kind != CanisterKind::Service {
                return Err(ConfigSchemaError::ValidationError(format!(
                    "app index canister '{canister_role}' must have kind = \"service\"",
                )));
            }
        }

        let mut root_roles = Vec::new();
        for (subnet_role, subnet) in &self.subnets {
            for (canister_role, canister_cfg) in &subnet.canisters {
                if canister_cfg.kind == CanisterKind::Root {
                    root_roles.push(format!("{subnet_role}:{canister_role}"));
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
        validate_canister_role_len(role, "role declaration")?;

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
            let display = config
                .fleet_role_ref(role)
                .map_or_else(|| role.to_string(), |role_ref| role_ref.to_string());
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
