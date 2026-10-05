//! Module: config::caller_authority
//!
//! Responsibility: define and compile bounded receiver-local caller permissions.
//! Does not own: membership, publication progress, or endpoint admission.
//! Boundary: host validation resolves exact Component Spec/role pairs before embedding policy.

#[cfg(test)]
mod tests;

use crate::ids::{CanisterRole, ComponentSpecId};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use thiserror::Error;

/// Maximum distinct permission names compiled into one receiver artifact.
pub const MAX_CALLER_PERMISSIONS: usize = 128;
/// Maximum exact source selectors in one compiled receiver policy.
pub const MAX_CALLER_SELECTORS: usize = 4_096;
/// Physical entry ceiling; deployments must also reserve their complete actual census.
pub const MAX_CALLER_ENTRIES: u32 = 65_536;
/// Physical encoded-byte ceiling for a receiver's durable projection.
pub const MAX_CALLER_BYTES: u32 = 64 * 1_024 * 1_024;
/// Fixed receiver control-header reservation included in the declared byte capacity.
pub const CALLER_HEADER_BYTES: u32 = 8_192;

/// Compile-time exact-name check over build-generated permission metadata.
#[must_use]
pub const fn permission_is_declared(declarations: &str, permission: &str) -> bool {
    let declarations = declarations.as_bytes();
    let permission = permission.as_bytes();
    if permission.is_empty() {
        return false;
    }
    let mut start = 0;
    while start < declarations.len() {
        let mut end = start;
        while end < declarations.len() && declarations[end] != b',' {
            end += 1;
        }
        if end - start == permission.len() {
            let mut offset = 0;
            while offset < permission.len() && declarations[start + offset] == permission[offset] {
                offset += 1;
            }
            if offset == permission.len() {
                return true;
            }
        }
        start = end + 1;
    }
    false
}

///
/// CallerAuthorityConfig
///
/// Bounded human-authored receiver permission declaration.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CallerAuthorityConfig {
    pub maximum_entries: u32,
    pub maximum_bytes: u32,
    pub permissions: BTreeMap<String, CallerPermission>,
}

///
/// CallerPermission
///
/// One named endpoint operation class with exact source pairs and one explicit scope.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CallerPermission {
    #[serde(default)]
    pub direction: CallerPermissionDirection,
    pub scope: CallerScope,
    pub sources: Vec<CallerSourceSelector>,
}

///
/// CallerPermissionDirection
///
/// Distinguish transport-caller admission from selecting a destination for a proxy call.
///

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CallerPermissionDirection {
    #[default]
    Caller,
    Target,
}

///
/// CallerScope
///
/// Root-local permission boundary; neither alternative grants cross-Root authority.
///

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CallerScope {
    SameComponent,
    SameRoot,
}

///
/// CallerSourceSelector
///
/// An indivisible source selector, preserving the declared Spec/role pairing.
///

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CallerSourceSelector {
    pub component_spec: ComponentSpecId,
    pub role: CanisterRole,
}

///
/// CompiledCallerPolicy
///
/// Immutable role-bound policy embedded by build tooling, with canonical content identity.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompiledCallerPolicy {
    pub role: CanisterRole,
    pub configuration: Option<CallerAuthorityConfig>,
    pub digest: [u8; 32],
}

///
/// CallerPolicyError
///
/// Typed build-time policy refusal; malformed declarations never produce artifacts.
///

#[derive(Clone, Debug, Eq, Error, PartialEq)]
pub enum CallerPolicyError {
    #[error("caller-authority capacity is zero or exceeds its physical bound")]
    Capacity,

    #[error("caller-authority source selector is duplicated")]
    DuplicateSelector,

    #[error("caller-authority permission must select at least one source")]
    EmptySources,

    #[error("caller-authority permission name is invalid")]
    InvalidPermissionName,

    #[error("caller-authority permission count exceeds its bound")]
    PermissionCapacity,

    #[error("caller-authority selector count exceeds its bound")]
    SelectorCapacity,

    #[error("caller-authority selector does not name a declared Component Spec/role pair")]
    UnknownSource,

    #[error("caller-authority receiver must be a managed Component or child role")]
    UnsupportedReceiver,
}

impl CompiledCallerPolicy {
    /// Normalize selector order and bind every permission field to the receiver role.
    /// Source resolution belongs to complete-model host validation.
    pub fn compile(
        role: CanisterRole,
        mut configuration: Option<CallerAuthorityConfig>,
    ) -> Result<Self, CallerPolicyError> {
        if let Some(config) = &mut configuration {
            validate_shape(config)?;
            for permission in config.permissions.values_mut() {
                permission.sources.sort();
            }
        }
        let digest = policy_digest(&role, configuration.as_ref());
        Ok(Self {
            role,
            configuration,
            digest,
        })
    }

    /// Refuse altered generated policy before installing runtime authority.
    #[must_use]
    pub fn is_canonical(&self) -> bool {
        Self::compile(self.role.clone(), self.configuration.clone())
            .is_ok_and(|canonical| canonical == *self)
    }
}

#[cfg(any(not(target_arch = "wasm32"), test))]
pub(in crate::config) fn validate_sources(
    model: &crate::config::ConfigModel,
    config: &CallerAuthorityConfig,
) -> Result<(), CallerPolicyError> {
    validate_shape(config)?;
    for source in config
        .permissions
        .values()
        .flat_map(|permission| &permission.sources)
    {
        let spec = model
            .component_specs
            .get(&source.component_spec)
            .ok_or(CallerPolicyError::UnknownSource)?;
        if !model.roles.contains_key(&source.role) || spec.get_canister(&source.role).is_none() {
            return Err(CallerPolicyError::UnknownSource);
        }
    }
    Ok(())
}

fn validate_shape(config: &CallerAuthorityConfig) -> Result<(), CallerPolicyError> {
    if !(1..=MAX_CALLER_ENTRIES).contains(&config.maximum_entries)
        || !(CALLER_HEADER_BYTES * 2..=MAX_CALLER_BYTES).contains(&config.maximum_bytes)
    {
        return Err(CallerPolicyError::Capacity);
    }
    if config.permissions.len() > MAX_CALLER_PERMISSIONS {
        return Err(CallerPolicyError::PermissionCapacity);
    }
    let mut selector_count = 0_usize;
    for (name, permission) in &config.permissions {
        if name.len() > crate::config::schema::NAME_MAX_BYTES
            || !crate::shared_support::is_ascii_snake_case(name)
        {
            return Err(CallerPolicyError::InvalidPermissionName);
        }
        if permission.sources.is_empty() {
            return Err(CallerPolicyError::EmptySources);
        }
        selector_count = selector_count
            .checked_add(permission.sources.len())
            .ok_or(CallerPolicyError::SelectorCapacity)?;
        if selector_count > MAX_CALLER_SELECTORS {
            return Err(CallerPolicyError::SelectorCapacity);
        }
        let mut selectors = permission.sources.iter().collect::<Vec<_>>();
        selectors.sort();
        if selectors.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err(CallerPolicyError::DuplicateSelector);
        }
    }
    Ok(())
}

fn policy_digest(role: &CanisterRole, config: Option<&CallerAuthorityConfig>) -> [u8; 32] {
    let mut hash = Sha256::new();
    hash.update(b"canic.caller-policy.v1\0");
    hash_string(&mut hash, role.as_str());
    if let Some(config) = config {
        hash.update([1]);
        hash.update(config.maximum_entries.to_be_bytes());
        hash.update(config.maximum_bytes.to_be_bytes());
        hash.update((config.permissions.len() as u64).to_be_bytes());
        for (name, permission) in &config.permissions {
            hash_string(&mut hash, name);
            hash.update([match permission.direction {
                CallerPermissionDirection::Caller => 0,
                CallerPermissionDirection::Target => 1,
            }]);
            hash.update([match permission.scope {
                CallerScope::SameComponent => 0,
                CallerScope::SameRoot => 1,
            }]);
            hash.update((permission.sources.len() as u64).to_be_bytes());
            for source in &permission.sources {
                hash_string(&mut hash, source.component_spec.as_str());
                hash_string(&mut hash, source.role.as_str());
            }
        }
    } else {
        hash.update([0]);
    }
    hash.finalize().into()
}

fn hash_string(hash: &mut Sha256, value: &str) {
    hash.update((value.len() as u64).to_be_bytes());
    hash.update(value.as_bytes());
}
