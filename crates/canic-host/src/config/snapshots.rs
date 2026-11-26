//! Scope native snapshot visibility to explicitly selected application children.
use super::{CanisterConfig, SelectedFleet};
use crate::{Error, Result, application, network::principal};
use canic_management_protocol::stored::{SnapshotReaders, valid_snapshot_readers};
use ic_platform_types::Principal;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::Path};

/// Additional readers of every retained native snapshot of selected applications.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SnapshotConfig {
    /// Optional subset of application roles. Omission selects all application roles.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub roles: Option<Vec<String>>,
    /// Additional authenticated principals, at most ten. Empty selects controllers only.
    #[serde(default)]
    pub download_principals: Vec<String>,
}

impl SnapshotConfig {
    fn readers(&self) -> Result<Vec<Principal>> {
        let mut readers = Vec::new();
        for value in &self.download_principals {
            let id = principal(value).map_err(|_| {
                Error::Invalid(format!(
                    "snapshots.download_principals contains {value:?}; expected a principal"
                ))
            })?;
            readers.push(id);
        }
        if !valid_snapshot_readers(&readers) {
            return Err(Error::Invalid(format!(
                "snapshots.download_principals = {:?}; expected zero to ten distinct authenticated principals",
                self.download_principals
            )));
        }
        Ok(readers)
    }

    pub(super) fn validate(&self) -> Result<()> {
        self.readers().map(|_| ())
    }

    pub(super) fn validate_roles(&self, slots: &BTreeMap<String, CanisterConfig>) -> Result<()> {
        if let Some(roles) = &self.roles
            && (roles.is_empty()
                || roles.iter().enumerate().any(|(index, role)| {
                    roles[..index].contains(role)
                        || !slots.iter().any(|(name, slot)| {
                            application_child(name, slot) && slot.role.as_ref() == Some(role)
                        })
                }))
        {
            return Err(Error::Invalid(format!(
                "snapshots.roles = {roles:?}; expected one or more distinct, declared application roles"
            )));
        }
        Ok(())
    }
}

fn application_child(name: &str, slot: &CanisterConfig) -> bool {
    slot.parent.is_some()
        && name != "store"
        && !matches!(slot.role.as_deref(), Some("root" | "wasm-store"))
}

impl SelectedFleet {
    /// Preview the exact reader replacement for a bound or installed permanent child.
    /// # Errors
    /// Refuses missing or ambiguous identities, infrastructure targets and invalid readers.
    pub fn snapshot_readers(&self, journal: &Path, child: &str) -> Result<SnapshotReaders> {
        let slot = self
            .config
            .canisters
            .get(child)
            .filter(|s| application_child(child, s))
            .ok_or_else(|| {
                Error::Invalid(format!(
                    "snapshot target {child:?} must be an application child"
                ))
            })?;
        let readers = match &self.config.snapshots {
            Some(policy)
                if policy.roles.as_ref().is_none_or(|roles| {
                    slot.role.as_ref().is_some_and(|role| roles.contains(role))
                }) =>
            {
                policy.readers()?
            }
            _ => Vec::new(),
        };
        let root = self
            .config
            .canisters
            .iter()
            .find(|(_, c)| c.role.as_deref() == Some("root") && c.parent.is_none())
            .ok_or_else(|| Error::Invalid("snapshot target requires a fleet Root".into()))?
            .0;
        Ok(SnapshotReaders {
            root: principal(&application::target(journal, self, root)?)?,
            child: principal(&application::target(journal, self, child)?)?,
            readers,
        })
    }
}
