//! Module: config::validation::component_spec
//!
//! Responsibility: validate Component Spec catalogs, spawn grants, placement, and refill policy.
//! Does not own: topology workflow, placement policy execution, or schema definitions.
//! Boundary: config validation calls this before runtime installation.

use crate::{
    config::schema::{
        CanisterAuthConfig, CanisterConfig, ComponentChildKind, ComponentSpecConfig,
        ConfigSchemaError, CyclesFundingPolicyConfig, MAX_COMPONENT_CHILD_ROLES,
        MAX_COMPONENT_PROVISIONING_GRANTS, MAX_COMPONENT_SPAWN_GRANTS, NAME_MAX_BYTES, TopupPolicy,
        Validate,
    },
    config::validation::validate_canister_role,
    ids::CanisterRole,
    model::auth::application_authorization::{
        ApplicationScope, MAX_LOCAL_APPLICATION_SESSION_TTL_NS, MAX_VERIFIED_APPLICATION_SCOPES,
    },
};
use std::collections::BTreeMap;

impl Validate for ComponentSpecConfig {
    fn validate(&self) -> Result<(), ConfigSchemaError> {
        validate_canister_role(&self.component_role, "Component role")?;
        if self.component_role.is_fleet_coordinator()
            || self.component_role.is_root()
            || self.component_role.is_wasm_store()
        {
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
        if !self.children.is_empty() && self.limits.maximum_descendants == 0 {
            return Err(ConfigSchemaError::ValidationError(format!(
                "Component '{}' limits.maximum_descendants must be > 0 when child roles are declared",
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
        validate_auth(&self.auth, &self.component_role)?;

        validate_component_children(self)?;
        validate_spawn_grants(self)?;
        validate_provisioning_grant_limits(self)?;

        validate_placement_policies(
            self,
            &self.component_role,
            &self.component_canister_config(),
        )?;
        for (role, child) in &self.children {
            validate_placement_policies(self, role, &child.canister_config())?;
        }

        Ok(())
    }
}

fn validate_component_children(config: &ComponentSpecConfig) -> Result<(), ConfigSchemaError> {
    for (role, child) in &config.children {
        validate_canister_role(role, "Component child role")?;
        if role == &config.component_role {
            return Err(ConfigSchemaError::ValidationError(format!(
                "Component '{}' cannot also be its own child",
                config.component_role,
            )));
        }
        if role.is_fleet_coordinator() || role.is_root() || role.is_wasm_store() {
            return Err(ConfigSchemaError::ValidationError(format!(
                "Component Child '{role}' is reserved infrastructure",
            )));
        }
        validate_cycles_funding(&child.cycles_funding, role)?;
        validate_topup(child.topup.as_ref(), role)?;
        validate_auth(&child.auth, role)?;
    }

    Ok(())
}

fn validate_auth(
    auth: &CanisterAuthConfig,
    canister: &CanisterRole,
) -> Result<(), ConfigSchemaError> {
    let Some(local) = &auth.local_application_authorization else {
        return Ok(());
    };

    if !auth.delegated_token_verifier {
        return Err(ConfigSchemaError::ValidationError(format!(
            "canister '{canister}' auth.local_application_authorization requires auth.delegated_token_verifier = true",
        )));
    }
    if local.allowed_scopes.is_empty() {
        return Err(ConfigSchemaError::ValidationError(format!(
            "canister '{canister}' auth.local_application_authorization.allowed_scopes must not be empty",
        )));
    }
    if local.allowed_scopes.len() > MAX_VERIFIED_APPLICATION_SCOPES {
        return Err(ConfigSchemaError::ValidationError(format!(
            "canister '{canister}' auth.local_application_authorization.allowed_scopes exceeds {MAX_VERIFIED_APPLICATION_SCOPES} entries",
        )));
    }
    for (index, scope) in local.allowed_scopes.iter().enumerate() {
        ApplicationScope::parse(scope.clone()).map_err(|error| {
            ConfigSchemaError::ValidationError(format!(
                "canister '{canister}' auth.local_application_authorization.allowed_scopes[{index}] is invalid: {error}",
            ))
        })?;
        if index > 0 && local.allowed_scopes[index - 1] >= *scope {
            return Err(ConfigSchemaError::ValidationError(format!(
                "canister '{canister}' auth.local_application_authorization.allowed_scopes must be strictly sorted and unique",
            )));
        }
    }

    let hard_maximum_secs = MAX_LOCAL_APPLICATION_SESSION_TTL_NS / 1_000_000_000;
    if local.default_session_ttl_secs == 0 || local.maximum_session_ttl_secs == 0 {
        return Err(ConfigSchemaError::ValidationError(format!(
            "canister '{canister}' auth.local_application_authorization session TTLs must be greater than zero",
        )));
    }
    if local.default_session_ttl_secs > local.maximum_session_ttl_secs {
        return Err(ConfigSchemaError::ValidationError(format!(
            "canister '{canister}' auth.local_application_authorization.default_session_ttl_secs must not exceed maximum_session_ttl_secs",
        )));
    }
    if local.maximum_session_ttl_secs > hard_maximum_secs {
        return Err(ConfigSchemaError::ValidationError(format!(
            "canister '{canister}' auth.local_application_authorization.maximum_session_ttl_secs must not exceed {hard_maximum_secs}",
        )));
    }

    Ok(())
}

fn validate_spawn_grants(config: &ComponentSpecConfig) -> Result<(), ConfigSchemaError> {
    let grant_count = config
        .spawn_grants
        .values()
        .try_fold(0_usize, |count, grants| count.checked_add(grants.len()));
    let Some(grant_count) = grant_count else {
        return Err(ConfigSchemaError::ValidationError(format!(
            "Component '{}' spawn grant count overflowed",
            config.component_role,
        )));
    };
    if grant_count > MAX_COMPONENT_SPAWN_GRANTS {
        return Err(ConfigSchemaError::ValidationError(format!(
            "Component '{}' declares {grant_count} spawn grants, exceeding bound {MAX_COMPONENT_SPAWN_GRANTS}",
            config.component_role,
        )));
    }

    let mut incoming = BTreeMap::<&CanisterRole, usize>::new();
    for (parent_role, grants) in &config.spawn_grants {
        if parent_role != &config.component_role && !config.children.contains_key(parent_role) {
            return Err(ConfigSchemaError::ValidationError(format!(
                "Component '{}' spawn grants reference undeclared parent role '{parent_role}'",
                config.component_role,
            )));
        }

        for (child_role, grant) in grants {
            let Some(child) = config.children.get(child_role) else {
                return Err(ConfigSchemaError::ValidationError(format!(
                    "Component '{}' spawn grant '{parent_role}' -> '{child_role}' references an undeclared child role",
                    config.component_role,
                )));
            };
            if grant.maximum_instances_per_parent == 0 {
                return Err(ConfigSchemaError::ValidationError(format!(
                    "Component '{}' spawn grant '{parent_role}' -> '{child_role}' maximum_instances_per_parent must be > 0",
                    config.component_role,
                )));
            }
            if child.kind == ComponentChildKind::Singleton
                && grant.maximum_instances_per_parent != 1
            {
                return Err(ConfigSchemaError::ValidationError(format!(
                    "singleton Component Child '{child_role}' spawn grants must set maximum_instances_per_parent = 1",
                )));
            }
            *incoming.entry(child_role).or_default() += 1;
        }
    }

    for child_role in config.children.keys() {
        if !incoming.contains_key(child_role) {
            return Err(ConfigSchemaError::ValidationError(format!(
                "Component '{}' child role '{child_role}' has no incoming spawn grant",
                config.component_role,
            )));
        }
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

fn validate_placement_policies(
    cfg: &ComponentSpecConfig,
    role: &CanisterRole,
    canister: &CanisterConfig,
) -> Result<(), ConfigSchemaError> {
    validate_scaling(cfg, role, canister)?;
    validate_sharding(cfg, role, canister)?;
    validate_index(cfg, role, canister)
}

fn spawn_limit(
    cfg: &ComponentSpecConfig,
    parent_role: &CanisterRole,
    child_role: &CanisterRole,
) -> Result<u32, ConfigSchemaError> {
    cfg.spawn_grants
        .get(parent_role)
        .and_then(|grants| grants.get(child_role))
        .map(|grant| grant.maximum_instances_per_parent)
        .ok_or_else(|| {
            ConfigSchemaError::ValidationError(format!(
                "Component '{}' placement policy for '{parent_role}' targets '{child_role}' without a matching spawn grant",
                cfg.component_role,
            ))
        })
}

fn validate_sharding(
    cfg: &ComponentSpecConfig,
    role: &CanisterRole,
    canister: &CanisterConfig,
) -> Result<(), ConfigSchemaError> {
    let Some(sharding) = &canister.sharding else {
        return Ok(());
    };

    for (pool_name, pool) in &sharding.pools {
        if pool_name.len() > NAME_MAX_BYTES {
            return Err(ConfigSchemaError::ValidationError(format!(
                "canister '{role}' sharding pool '{pool_name}' name exceeds {NAME_MAX_BYTES} bytes",
            )));
        }

        if !cfg.children.contains_key(&pool.canister_role) {
            return Err(ConfigSchemaError::ValidationError(format!(
                "Component '{role}' sharding pool '{pool_name}' references undeclared child role '{}'",
                pool.canister_role
            )));
        }

        let target = &cfg.children[&pool.canister_role];
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

        let maximum_instances_per_parent = spawn_limit(cfg, role, &pool.canister_role)?;
        if pool.policy.max_shards > maximum_instances_per_parent {
            return Err(ConfigSchemaError::ValidationError(format!(
                "Component '{role}' sharding pool '{pool_name}' max_shards exceeds spawn grant to '{}' maximum_instances_per_parent",
                pool.canister_role,
            )));
        }
    }

    Ok(())
}

fn validate_scaling(
    cfg: &ComponentSpecConfig,
    role: &CanisterRole,
    canister: &CanisterConfig,
) -> Result<(), ConfigSchemaError> {
    let Some(scaling) = &canister.scaling else {
        return Ok(());
    };

    for (pool_name, pool) in &scaling.pools {
        if pool_name.len() > NAME_MAX_BYTES {
            return Err(ConfigSchemaError::ValidationError(format!(
                "canister '{role}' scaling pool '{pool_name}' name exceeds {NAME_MAX_BYTES} bytes",
            )));
        }

        if !cfg.children.contains_key(&pool.canister_role) {
            return Err(ConfigSchemaError::ValidationError(format!(
                "Component '{role}' scaling pool '{pool_name}' references undeclared child role '{}'",
                pool.canister_role
            )));
        }

        let target = &cfg.children[&pool.canister_role];
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

        let maximum_instances_per_parent = spawn_limit(cfg, role, &pool.canister_role)?;
        if pool.policy.max_workers > maximum_instances_per_parent {
            return Err(ConfigSchemaError::ValidationError(format!(
                "Component '{role}' scaling pool '{pool_name}' max_workers exceeds spawn grant to '{}' maximum_instances_per_parent",
                pool.canister_role,
            )));
        }
    }

    Ok(())
}

fn validate_index(
    cfg: &ComponentSpecConfig,
    role: &CanisterRole,
    canister: &CanisterConfig,
) -> Result<(), ConfigSchemaError> {
    let Some(index) = &canister.index else {
        return Ok(());
    };

    for (pool_name, pool) in &index.pools {
        if pool_name.len() > NAME_MAX_BYTES {
            return Err(ConfigSchemaError::ValidationError(format!(
                "canister '{role}' index pool '{pool_name}' name exceeds {NAME_MAX_BYTES} bytes",
            )));
        }

        if pool.key_name.is_empty() {
            return Err(ConfigSchemaError::ValidationError(format!(
                "canister '{role}' index pool '{pool_name}' must define a non-empty key_name",
            )));
        }

        if pool.key_name.len() > NAME_MAX_BYTES {
            return Err(ConfigSchemaError::ValidationError(format!(
                "canister '{role}' index pool '{pool_name}' key_name '{}' exceeds {NAME_MAX_BYTES} bytes",
                pool.key_name
            )));
        }

        if !cfg.children.contains_key(&pool.canister_role) {
            return Err(ConfigSchemaError::ValidationError(format!(
                "Component '{role}' index pool '{pool_name}' references undeclared child role '{}'",
                pool.canister_role
            )));
        }

        let target = &cfg.children[&pool.canister_role];
        if target.kind != ComponentChildKind::Instance {
            return Err(ConfigSchemaError::ValidationError(format!(
                "Component '{role}' index pool '{pool_name}' references child '{}' which is not kind = \"instance\"",
                pool.canister_role
            )));
        }
        let _ = spawn_limit(cfg, role, &pool.canister_role)?;
    }

    Ok(())
}
