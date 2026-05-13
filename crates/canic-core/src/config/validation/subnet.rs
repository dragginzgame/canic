use crate::{
    config::schema::{
        CanisterConfig, CanisterKind, ConfigSchemaError, NAME_MAX_BYTES, SubnetConfig, Validate,
    },
    ids::CanisterRole,
};
use std::collections::BTreeMap;

fn validate_role_len(role: &CanisterRole, context: &str) -> Result<(), ConfigSchemaError> {
    if role.as_ref().len() > NAME_MAX_BYTES {
        return Err(ConfigSchemaError::ValidationError(format!(
            "{context} '{role}' exceeds {NAME_MAX_BYTES} bytes",
        )));
    }

    Ok(())
}

impl Validate for SubnetConfig {
    fn validate(&self) -> Result<(), ConfigSchemaError> {
        for role in &self.auto_create {
            validate_role_len(role, "auto-create canister")?;
            if !self.canisters.contains_key(role) {
                return Err(ConfigSchemaError::ValidationError(format!(
                    "auto-create canister '{role}' is not defined in subnet",
                )));
            }
        }

        for role in &self.subnet_index {
            validate_role_len(role, "subnet index canister")?;
            let cfg = self.canisters.get(role).ok_or_else(|| {
                ConfigSchemaError::ValidationError(format!(
                    "subnet index canister '{role}' is not defined in subnet",
                ))
            })?;

            if cfg.kind != CanisterKind::Singleton {
                return Err(ConfigSchemaError::ValidationError(format!(
                    "subnet index canister '{role}' must have kind = \"singleton\"",
                )));
            }
        }

        if self.canisters.contains_key(&CanisterRole::WASM_STORE) {
            return Err(ConfigSchemaError::ValidationError(format!(
                "{} is implicit and must not be configured under subnets.<name>.canisters",
                CanisterRole::WASM_STORE
            )));
        }

        for (role, cfg) in &self.canisters {
            validate_role_len(role, "canister")?;

            if cfg.randomness.enabled && cfg.randomness.reseed_interval_secs == 0 {
                return Err(ConfigSchemaError::ValidationError(format!(
                    "canister '{role}' randomness reseed_interval_secs must be > 0",
                )));
            }

            validate_kind(cfg, role)?;
            validate_topup(cfg, role)?;
            validate_scaling(cfg, role, &self.canisters)?;
            validate_sharding(cfg, role, &self.canisters)?;
            validate_directory(cfg, role, &self.canisters)?;
        }

        Ok(())
    }
}

fn validate_topup(cfg: &CanisterConfig, canister: &CanisterRole) -> Result<(), ConfigSchemaError> {
    let Some(topup) = &cfg.topup else {
        return Ok(());
    };

    let threshold = topup.threshold.to_u128();
    let amount = topup.amount.to_u128();

    if amount.saturating_mul(2) > threshold {
        return Err(ConfigSchemaError::ValidationError(format!(
            "canister '{canister}' topup.amount must be <= 50% of topup.threshold (got amount={amount}, threshold={threshold})",
        )));
    }

    Ok(())
}

fn validate_kind(cfg: &CanisterConfig, canister: &CanisterRole) -> Result<(), ConfigSchemaError> {
    match cfg.kind {
        CanisterKind::Root => {
            if cfg.scaling.is_some()
                || cfg.sharding.is_some()
                || cfg.directory.is_some()
                || cfg.auth.delegated_token_signer
                || cfg.auth.role_attestation_cache
                || cfg.standards.icrc21
            {
                return Err(ConfigSchemaError::ValidationError(format!(
                    "canister '{canister}' kind = \"root\" cannot define scaling, sharding, directory, auth signer/cache roles, or canister-local standards",
                )));
            }
        }

        CanisterKind::Singleton => {}

        CanisterKind::Replica | CanisterKind::Shard | CanisterKind::Instance => {
            if cfg.scaling.is_some() || cfg.sharding.is_some() || cfg.directory.is_some() {
                return Err(ConfigSchemaError::ValidationError(format!(
                    "canister '{canister}' kind = \"{}\" cannot define scaling, sharding, or directory",
                    cfg.kind,
                )));
            }
        }
    }

    Ok(())
}

fn validate_sharding(
    cfg: &CanisterConfig,
    role: &CanisterRole,
    all_roles: &BTreeMap<CanisterRole, CanisterConfig>,
) -> Result<(), ConfigSchemaError> {
    let Some(sharding) = &cfg.sharding else {
        return Ok(());
    };

    for (pool_name, pool) in &sharding.pools {
        if pool_name.len() > NAME_MAX_BYTES {
            return Err(ConfigSchemaError::ValidationError(format!(
                "canister '{role}' sharding pool '{pool_name}' name exceeds {NAME_MAX_BYTES} bytes",
            )));
        }

        if !all_roles.contains_key(&pool.canister_role) {
            return Err(ConfigSchemaError::ValidationError(format!(
                "canister '{role}' sharding pool '{pool_name}' references unknown canister role '{}'",
                pool.canister_role
            )));
        }

        let target = &all_roles[&pool.canister_role];
        if target.kind != CanisterKind::Shard {
            return Err(ConfigSchemaError::ValidationError(format!(
                "canister '{role}' sharding pool '{pool_name}' references canister '{}' which is not kind = \"shard\"",
                pool.canister_role
            )));
        }

        if pool.policy.capacity == 0 || pool.policy.max_shards == 0 {
            return Err(ConfigSchemaError::ValidationError(format!(
                "canister '{role}' sharding pool '{pool_name}' must have positive capacity and max_shards",
            )));
        }

        if pool.policy.initial_shards > pool.policy.max_shards {
            return Err(ConfigSchemaError::ValidationError(format!(
                "canister '{role}' sharding pool '{pool_name}' has initial_shards > max_shards",
            )));
        }
    }

    Ok(())
}

fn validate_scaling(
    cfg: &CanisterConfig,
    role: &CanisterRole,
    all_roles: &BTreeMap<CanisterRole, CanisterConfig>,
) -> Result<(), ConfigSchemaError> {
    let Some(scaling) = &cfg.scaling else {
        return Ok(());
    };

    for (pool_name, pool) in &scaling.pools {
        if pool_name.len() > NAME_MAX_BYTES {
            return Err(ConfigSchemaError::ValidationError(format!(
                "canister '{role}' scaling pool '{pool_name}' name exceeds {NAME_MAX_BYTES} bytes",
            )));
        }

        if !all_roles.contains_key(&pool.canister_role) {
            return Err(ConfigSchemaError::ValidationError(format!(
                "canister '{role}' scaling pool '{pool_name}' references unknown canister role '{}'",
                pool.canister_role
            )));
        }

        let target = &all_roles[&pool.canister_role];
        if target.kind != CanisterKind::Replica {
            return Err(ConfigSchemaError::ValidationError(format!(
                "canister '{role}' scaling pool '{pool_name}' references canister '{}' which is not kind = \"replica\"",
                pool.canister_role
            )));
        }

        if pool.policy.max_workers != 0 && pool.policy.max_workers < pool.policy.min_workers {
            return Err(ConfigSchemaError::ValidationError(format!(
                "canister '{role}' scaling pool '{pool_name}' has max_workers < min_workers",
            )));
        }

        if pool.policy.max_workers != 0 && pool.policy.max_workers < pool.policy.initial_workers {
            return Err(ConfigSchemaError::ValidationError(format!(
                "canister '{role}' scaling pool '{pool_name}' has max_workers < initial_workers",
            )));
        }
    }

    Ok(())
}

fn validate_directory(
    cfg: &CanisterConfig,
    role: &CanisterRole,
    all_roles: &BTreeMap<CanisterRole, CanisterConfig>,
) -> Result<(), ConfigSchemaError> {
    let Some(directory) = &cfg.directory else {
        return Ok(());
    };

    for (pool_name, pool) in &directory.pools {
        if pool_name.len() > NAME_MAX_BYTES {
            return Err(ConfigSchemaError::ValidationError(format!(
                "canister '{role}' directory pool '{pool_name}' name exceeds {NAME_MAX_BYTES} bytes",
            )));
        }

        if pool.key_name.is_empty() {
            return Err(ConfigSchemaError::ValidationError(format!(
                "canister '{role}' directory pool '{pool_name}' must define a non-empty key_name",
            )));
        }

        if pool.key_name.len() > NAME_MAX_BYTES {
            return Err(ConfigSchemaError::ValidationError(format!(
                "canister '{role}' directory pool '{pool_name}' key_name '{}' exceeds {NAME_MAX_BYTES} bytes",
                pool.key_name
            )));
        }

        if !all_roles.contains_key(&pool.canister_role) {
            return Err(ConfigSchemaError::ValidationError(format!(
                "canister '{role}' directory pool '{pool_name}' references unknown canister role '{}'",
                pool.canister_role
            )));
        }

        let target = &all_roles[&pool.canister_role];
        if target.kind != CanisterKind::Instance {
            return Err(ConfigSchemaError::ValidationError(format!(
                "canister '{role}' directory pool '{pool_name}' references canister '{}' which is not kind = \"instance\"",
                pool.canister_role
            )));
        }
    }

    Ok(())
}
