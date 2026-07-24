use crate::{
    domain::value::Principal,
    ids::{CanisterRole, SubnetSlotId},
    model::env::ValidatedEnv,
};
use thiserror::Error as ThisError;

///
/// EnvInput
///

#[derive(Clone, Debug)]
pub struct EnvInput {
    pub fleet_root_pid: Option<Principal>,
    pub subnet_slot: Option<SubnetSlotId>,
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
    if raw_env.subnet_slot.is_none() {
        missing.push("subnet_slot");
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

    if !missing.is_empty() {
        return Err(EnvPolicyError::MissingEnvFields(missing.join(", ")));
    }

    let fleet_root_pid = raw_env
        .fleet_root_pid
        .ok_or_else(|| EnvPolicyError::MissingEnvFields("fleet_root_pid".to_string()))?;
    let subnet_slot = raw_env
        .subnet_slot
        .ok_or_else(|| EnvPolicyError::MissingEnvFields("subnet_slot".to_string()))?;
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
        subnet_slot,
        subnet_pid,
        root_pid,
        canister_role,
        parent_pid,
    })
}
