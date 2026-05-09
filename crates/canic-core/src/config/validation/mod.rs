mod app;
mod auth;
mod fleet;
mod subnet;

use crate::{
    config::schema::{CanisterKind, ConfigModel, ConfigSchemaError, NAME_MAX_BYTES, Validate},
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
        if let Some(fleet) = &self.fleet {
            fleet.validate()?;
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

            if canister_cfg.kind != CanisterKind::Singleton {
                return Err(ConfigSchemaError::ValidationError(format!(
                    "app index canister '{canister_role}' must have kind = \"singleton\"",
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

        Ok(())
    }
}
