use crate::{
    domain::value::Principal,
    ids::{CanisterRole, ComponentSpecId},
    model::env::ValidatedEnv,
};
use thiserror::Error as ThisError;

///
/// EnvInput
///

#[derive(Clone, Debug)]
pub struct EnvInput {
    pub fleet_root_pid: Option<Principal>,
    pub component_spec: Option<ComponentSpecId>,
    pub subnet_pid: Option<Principal>,
    pub root_pid: Option<Principal>,
    pub canister_role: Option<CanisterRole>,
    pub parent_pid: Option<Principal>,
}

///
/// EnvPolicyError
///

#[derive(Debug, ThisError)]
pub enum EnvPolicyError {
    #[error("missing required env fields: {0}")]
    MissingEnvFields(String),
}

pub fn validate_or_default(raw_env: EnvInput) -> Result<ValidatedEnv, EnvPolicyError> {
    let mut missing = Vec::new();
    if raw_env.fleet_root_pid.is_none() {
        missing.push("fleet_root_pid");
    }
    if raw_env.subnet_pid.is_none() {
        missing.push("subnet_pid");
    }
    if raw_env.root_pid.is_none() {
        missing.push("root_pid");
    }
    if raw_env.canister_role.is_none() {
        missing.push("canister_role");
    }
    if raw_env.parent_pid.is_none() {
        missing.push("parent_pid");
    }
    if raw_env
        .canister_role
        .as_ref()
        .is_some_and(|role| !role.is_root() && !role.is_wasm_store())
        && raw_env.component_spec.is_none()
    {
        missing.push("component_spec");
    }

    if !missing.is_empty() {
        return Err(EnvPolicyError::MissingEnvFields(missing.join(", ")));
    }

    let fleet_root_pid = raw_env
        .fleet_root_pid
        .ok_or_else(|| EnvPolicyError::MissingEnvFields("fleet_root_pid".to_string()))?;
    let component_spec = raw_env.component_spec;
    let subnet_pid = raw_env
        .subnet_pid
        .ok_or_else(|| EnvPolicyError::MissingEnvFields("subnet_pid".to_string()))?;
    let root_pid = raw_env
        .root_pid
        .ok_or_else(|| EnvPolicyError::MissingEnvFields("root_pid".to_string()))?;
    let canister_role = raw_env
        .canister_role
        .ok_or_else(|| EnvPolicyError::MissingEnvFields("canister_role".to_string()))?;
    let parent_pid = raw_env
        .parent_pid
        .ok_or_else(|| EnvPolicyError::MissingEnvFields("parent_pid".to_string()))?;

    Ok(ValidatedEnv {
        fleet_root_pid,
        component_spec,
        subnet_pid,
        root_pid,
        canister_role,
        parent_pid,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(role: CanisterRole, component_spec: Option<ComponentSpecId>) -> EnvInput {
        let principal = Principal::from_slice(&[1; 29]);
        EnvInput {
            fleet_root_pid: Some(principal),
            component_spec,
            subnet_pid: Some(principal),
            root_pid: Some(principal),
            canister_role: Some(role),
            parent_pid: Some(principal),
        }
    }

    #[test]
    fn infrastructure_does_not_require_a_component_spec() {
        for role in [CanisterRole::ROOT, CanisterRole::WASM_STORE] {
            let validated =
                validate_or_default(input(role, None)).expect("infrastructure env is valid");
            assert!(validated.component_spec.is_none());
        }
    }

    #[test]
    fn component_roles_still_require_a_component_spec() {
        validate_or_default(input(CanisterRole::from("hub"), None))
            .expect_err("Component role without Component Spec must reject");
    }
}
