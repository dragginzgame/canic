//! Module: config::validation::subnet
//!
//! Responsibility: validate subnet topology, placement, and refill configuration.
//! Does not own: topology workflow, placement policy execution, or schema definitions.
//! Boundary: config validation calls this before runtime installation.

use crate::{
    config::schema::{
        CanisterConfig, CanisterKind, ConfigSchemaError, CyclesFundingPolicyConfig, NAME_MAX_BYTES,
        SubnetConfig, TopupPolicy, Validate,
    },
    config::validation::validate_canister_role,
    ids::CanisterRole,
};
use std::collections::BTreeMap;

impl Validate for SubnetConfig {
    fn validate(&self) -> Result<(), ConfigSchemaError> {
        if self.canisters.contains_key(&CanisterRole::WASM_STORE) {
            return Err(ConfigSchemaError::ValidationError(format!(
                "{} is implicit and must not be configured under subnets.<name>.canisters",
                CanisterRole::WASM_STORE
            )));
        }

        for (role, cfg) in &self.canisters {
            validate_canister_role(role, "canister")?;

            if cfg.randomness.enabled && cfg.randomness.reseed_interval_secs == 0 {
                return Err(ConfigSchemaError::ValidationError(format!(
                    "canister '{role}' randomness reseed_interval_secs must be > 0",
                )));
            }

            validate_kind(cfg, role)?;
            validate_cycles_funding(&cfg.cycles_funding, role)?;
            validate_topup(cfg, role)?;
            validate_scaling(cfg, role, &self.canisters)?;
            validate_sharding(cfg, role, &self.canisters)?;
            validate_directory(cfg, role, &self.canisters)?;
        }

        Ok(())
    }
}

fn validate_cycles_funding(
    policy: &CyclesFundingPolicyConfig,
    canister: &CanisterRole,
) -> Result<(), ConfigSchemaError> {
    let max_per_request = policy.max_per_request.to_u128();
    let max_per_child = policy.max_per_child.to_u128();

    if max_per_request == 0 {
        return Err(ConfigSchemaError::ValidationError(format!(
            "canister '{canister}' cycles_funding.max_per_request must be > 0",
        )));
    }

    if max_per_child == 0 {
        return Err(ConfigSchemaError::ValidationError(format!(
            "canister '{canister}' cycles_funding.max_per_child must be > 0",
        )));
    }

    if policy.cooldown_secs == 0 {
        return Err(ConfigSchemaError::ValidationError(format!(
            "canister '{canister}' cycles_funding.cooldown_secs must be > 0",
        )));
    }

    if max_per_request > max_per_child {
        return Err(ConfigSchemaError::ValidationError(format!(
            "canister '{canister}' cycles_funding.max_per_request must be <= cycles_funding.max_per_child",
        )));
    }

    Ok(())
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

    validate_icp_refill(topup, canister)?;

    Ok(())
}

fn validate_icp_refill(
    topup: &TopupPolicy,
    canister: &CanisterRole,
) -> Result<(), ConfigSchemaError> {
    let Some(icp_refill) = &topup.icp_refill else {
        return Ok(());
    };

    if !icp_refill.enabled {
        return Ok(());
    }

    if icp_refill.max_refill_e8s_per_call == 0 {
        return Err(ConfigSchemaError::ValidationError(format!(
            "canister '{canister}' topup.icp_refill.max_refill_e8s_per_call must be > 0",
        )));
    }

    if icp_refill.min_xdr_permyriad_per_icp == Some(0) {
        return Err(ConfigSchemaError::ValidationError(format!(
            "canister '{canister}' topup.icp_refill.min_xdr_permyriad_per_icp must be > 0 when set",
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
                || cfg.auth.delegated_token_issuer
                || cfg.auth.delegated_token_verifier
                || cfg.auth.role_attestation_cache
                || cfg.standards.icrc21
            {
                return Err(ConfigSchemaError::ValidationError(format!(
                    "canister '{canister}' kind = \"root\" cannot define scaling, sharding, directory, auth verifier/issuer/cache roles, or canister-local standards",
                )));
            }
        }

        CanisterKind::Service => {}

        CanisterKind::Singleton => {
            if cfg.scaling.is_some() || cfg.sharding.is_some() || cfg.directory.is_some() {
                return Err(ConfigSchemaError::ValidationError(format!(
                    "canister '{canister}' kind = \"singleton\" cannot define scaling, sharding, or directory",
                )));
            }
        }

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
