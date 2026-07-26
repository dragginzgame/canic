//! Module: config::validation::component_spec
//!
//! Responsibility: validate Component Spec topology, placement, and refill configuration.
//! Does not own: topology workflow, placement policy execution, or schema definitions.
//! Boundary: config validation calls this before runtime installation.

use crate::{
    config::schema::{
        ComponentChildConfig, ComponentChildKind, ComponentSpecConfig, ConfigSchemaError,
        CyclesFundingPolicyConfig, MAX_COMPONENT_CHILD_ROLES, MAX_COMPONENT_PROVISIONING_GRANTS,
        NAME_MAX_BYTES, TopupPolicy, Validate,
    },
    config::validation::validate_canister_role,
    ids::CanisterRole,
};
use std::collections::BTreeMap;

impl Validate for ComponentSpecConfig {
    fn validate(&self) -> Result<(), ConfigSchemaError> {
        validate_canister_role(&self.component_role, "Component role")?;
        if self.component_role.is_root() || self.component_role.is_wasm_store() {
            return Err(ConfigSchemaError::ValidationError(format!(
                "Component role '{}' is reserved infrastructure",
                self.component_role,
            )));
        }
        if self.maximum_instances == 0 {
            return Err(ConfigSchemaError::ValidationError(format!(
                "Component '{}' maximum_instances must be > 0",
                self.component_role,
            )));
        }
        if self.children.len() > MAX_COMPONENT_CHILD_ROLES {
            return Err(ConfigSchemaError::ValidationError(format!(
                "Component '{}' declares {} child roles, exceeding bound {MAX_COMPONENT_CHILD_ROLES}",
                self.component_role,
                self.children.len(),
            )));
        }
        if self.provisions.len() > MAX_COMPONENT_PROVISIONING_GRANTS {
            return Err(ConfigSchemaError::ValidationError(format!(
                "Component '{}' declares {} provisioning grants, exceeding bound {MAX_COMPONENT_PROVISIONING_GRANTS}",
                self.component_role,
                self.provisions.len(),
            )));
        }
        if !self.children.is_empty() && self.limits.maximum_children == 0 {
            return Err(ConfigSchemaError::ValidationError(format!(
                "Component '{}' limits.maximum_children must be > 0 when children are declared",
                self.component_role,
            )));
        }
        if self.limits.maximum_registry_bytes == 0 {
            return Err(ConfigSchemaError::ValidationError(format!(
                "Component '{}' limits.maximum_registry_bytes must be > 0",
                self.component_role,
            )));
        }
        if self.limits.cycles_funding.window_secs == 0 {
            return Err(ConfigSchemaError::ValidationError(format!(
                "Component '{}' limits.cycles_funding.window_secs must be > 0",
                self.component_role,
            )));
        }
        if self.limits.cycles_funding.maximum_cycles.to_u128() == 0 {
            return Err(ConfigSchemaError::ValidationError(format!(
                "Component '{}' limits.cycles_funding.maximum_cycles must be > 0",
                self.component_role,
            )));
        }

        validate_cycles_funding(&self.cycles_funding, &self.component_role)?;
        validate_topup(self.topup.as_ref(), &self.component_role)?;

        validate_component_children(self)?;
        validate_provisioning_grant_limits(self)?;

        validate_scaling(self, &self.component_role, &self.children)?;
        validate_sharding(self, &self.component_role, &self.children)?;
        validate_binding(self, &self.component_role, &self.children)?;

        Ok(())
    }
}

fn validate_component_children(config: &ComponentSpecConfig) -> Result<(), ConfigSchemaError> {
    let mut initial_children = 0_u32;
    for (role, child) in &config.children {
        validate_canister_role(role, "Component Child")?;
        if role == &config.component_role {
            return Err(ConfigSchemaError::ValidationError(format!(
                "Component '{}' cannot also be its own child",
                config.component_role,
            )));
        }
        if role.is_root() || role.is_wasm_store() {
            return Err(ConfigSchemaError::ValidationError(format!(
                "Component Child '{role}' is reserved infrastructure",
            )));
        }
        if child.maximum_instances == 0 {
            return Err(ConfigSchemaError::ValidationError(format!(
                "Component Child '{role}' maximum_instances must be > 0",
            )));
        }
        if child.initial_instances > child.maximum_instances {
            return Err(ConfigSchemaError::ValidationError(format!(
                "Component Child '{role}' initial_instances must be <= maximum_instances",
            )));
        }
        if child.kind == ComponentChildKind::Singleton && child.maximum_instances != 1 {
            return Err(ConfigSchemaError::ValidationError(format!(
                "singleton Component Child '{role}' maximum_instances must equal 1",
            )));
        }
        initial_children = initial_children
            .checked_add(child.initial_instances)
            .ok_or_else(|| {
                ConfigSchemaError::ValidationError(format!(
                    "Component '{}' initial child count overflowed",
                    config.component_role,
                ))
            })?;
        validate_cycles_funding(&child.cycles_funding, role)?;
        validate_topup(child.topup.as_ref(), role)?;
    }
    if initial_children > config.limits.maximum_children {
        return Err(ConfigSchemaError::ValidationError(format!(
            "Component '{}' initial child count {initial_children} exceeds limits.maximum_children {}",
            config.component_role, config.limits.maximum_children,
        )));
    }

    Ok(())
}

fn validate_provisioning_grant_limits(
    config: &ComponentSpecConfig,
) -> Result<(), ConfigSchemaError> {
    for (target, grant) in &config.provisions {
        if grant.maximum_instances_per_requester_per_root == 0 {
            return Err(ConfigSchemaError::ValidationError(format!(
                "Component provisioning grant to '{target}' maximum_instances_per_requester_per_root must be > 0",
            )));
        }
    }

    Ok(())
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

fn validate_topup(
    topup: Option<&TopupPolicy>,
    canister: &CanisterRole,
) -> Result<(), ConfigSchemaError> {
    let Some(topup) = topup else {
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

fn validate_sharding(
    cfg: &ComponentSpecConfig,
    role: &CanisterRole,
    children: &BTreeMap<CanisterRole, ComponentChildConfig>,
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

        if !children.contains_key(&pool.canister_role) {
            return Err(ConfigSchemaError::ValidationError(format!(
                "Component '{role}' sharding pool '{pool_name}' references undeclared direct child '{}'",
                pool.canister_role
            )));
        }

        let target = &children[&pool.canister_role];
        if target.kind != ComponentChildKind::Shard {
            return Err(ConfigSchemaError::ValidationError(format!(
                "Component '{role}' sharding pool '{pool_name}' references child '{}' which is not kind = \"shard\"",
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

        if pool.policy.max_shards > target.maximum_instances {
            return Err(ConfigSchemaError::ValidationError(format!(
                "Component '{role}' sharding pool '{pool_name}' max_shards exceeds child '{}' maximum_instances",
                pool.canister_role,
            )));
        }
    }

    Ok(())
}

fn validate_scaling(
    cfg: &ComponentSpecConfig,
    role: &CanisterRole,
    children: &BTreeMap<CanisterRole, ComponentChildConfig>,
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

        if !children.contains_key(&pool.canister_role) {
            return Err(ConfigSchemaError::ValidationError(format!(
                "Component '{role}' scaling pool '{pool_name}' references undeclared direct child '{}'",
                pool.canister_role
            )));
        }

        let target = &children[&pool.canister_role];
        if target.kind != ComponentChildKind::Replica {
            return Err(ConfigSchemaError::ValidationError(format!(
                "Component '{role}' scaling pool '{pool_name}' references child '{}' which is not kind = \"replica\"",
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

        if pool.policy.max_workers == 0 {
            return Err(ConfigSchemaError::ValidationError(format!(
                "Component '{role}' scaling pool '{pool_name}' max_workers must be > 0",
            )));
        }

        if pool.policy.max_workers > target.maximum_instances {
            return Err(ConfigSchemaError::ValidationError(format!(
                "Component '{role}' scaling pool '{pool_name}' max_workers exceeds child '{}' maximum_instances",
                pool.canister_role,
            )));
        }
    }

    Ok(())
}

fn validate_binding(
    cfg: &ComponentSpecConfig,
    role: &CanisterRole,
    children: &BTreeMap<CanisterRole, ComponentChildConfig>,
) -> Result<(), ConfigSchemaError> {
    let Some(binding) = &cfg.binding else {
        return Ok(());
    };

    for (pool_name, pool) in &binding.pools {
        if pool_name.len() > NAME_MAX_BYTES {
            return Err(ConfigSchemaError::ValidationError(format!(
                "canister '{role}' binding pool '{pool_name}' name exceeds {NAME_MAX_BYTES} bytes",
            )));
        }

        if pool.key_name.is_empty() {
            return Err(ConfigSchemaError::ValidationError(format!(
                "canister '{role}' binding pool '{pool_name}' must define a non-empty key_name",
            )));
        }

        if pool.key_name.len() > NAME_MAX_BYTES {
            return Err(ConfigSchemaError::ValidationError(format!(
                "canister '{role}' binding pool '{pool_name}' key_name '{}' exceeds {NAME_MAX_BYTES} bytes",
                pool.key_name
            )));
        }

        if !children.contains_key(&pool.canister_role) {
            return Err(ConfigSchemaError::ValidationError(format!(
                "Component '{role}' binding pool '{pool_name}' references undeclared direct child '{}'",
                pool.canister_role
            )));
        }

        let target = &children[&pool.canister_role];
        if target.kind != ComponentChildKind::Instance {
            return Err(ConfigSchemaError::ValidationError(format!(
                "Component '{role}' binding pool '{pool_name}' references child '{}' which is not kind = \"instance\"",
                pool.canister_role
            )));
        }
    }

    Ok(())
}
