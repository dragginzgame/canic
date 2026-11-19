//! Compile one fleet's selected roles and policy into exact child install bytes.
use super::{hex32, invalid, principal};
use crate::{CanisterConfig, FleetConfig, Result, supplier};
use canic::auth::install::{Install, LimitsPolicy};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Receiver install input. Root authorization for each receiver is a separate operation.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReceiverPolicy {
    /// Receiver storage lifetime. Omitted selects volatile.
    #[serde(default)]
    pub storage: canic::auth::install::Storage,
    /// Configured enrollment authority slot.
    pub root: String,
    /// Application canister roles selecting this policy.
    pub roles: Vec<String>,
    /// Trusted credential issuer.
    pub issuer: String,
    /// Accepted credential role (separate from canister role).
    pub role: String,
    /// Canonical network identifier as lowercase hexadecimal.
    pub network_id: String,
    /// Logical application audience as lowercase hexadecimal.
    pub application_id: String,
    /// Bounds for this fleet.
    pub limits: LimitsPolicy,
}

impl ReceiverPolicy {
    pub(crate) fn validate(&self, slots: &BTreeMap<String, CanisterConfig>) -> Result<()> {
        if !slots
            .get(&self.root)
            .is_some_and(|s| s.role.as_deref() == Some("root") && s.parent.is_none())
        {
            return Err(invalid(
                "receiver policy requires a declared top-level Root",
            ));
        }
        let mut seen = BTreeSet::new();
        if self.roles.is_empty()
            || self.roles.len() > 32
            || self.roles.iter().any(|role| {
                matches!(role.as_str(), "root" | "wasm-store")
                    || !seen.insert(role)
                    || !slots.values().any(|slot| slot.role.as_ref() == Some(role))
            })
        {
            return Err(invalid(
                "receiver policy needs distinct declared application roles",
            ));
        }
        self.role
            .parse::<canic::auth::enrollment::types::AuthRole>()
            .map_err(supplier)?;
        principal(&self.issuer)?;
        hex32(&self.network_id)?;
        hex32(&self.application_id)?;
        if self.limits.default_session_secs == 0
            || self.limits.default_session_secs > self.limits.max_session_secs
            || self.limits.max_session_secs > 1800
            || self.limits.max_proof_ttl_ns == 0
            || self.limits.max_sessions == Some(0)
        {
            return Err(invalid("invalid receiver policy limits"));
        }
        Ok(())
    }
}

impl FleetConfig {
    pub(crate) fn resolve_receiver_grants(&mut self) -> Result<()> {
        let Some(policy) = &self.receiver_policy else {
            return Ok(());
        };
        for grant in &mut self.auth {
            if !self
                .canisters
                .get(&grant.receiver)
                .and_then(|s| s.role.as_ref())
                .is_some_and(|role| policy.roles.contains(role))
            {
                continue;
            }
            for (value, expected) in [
                (&mut grant.root, &policy.root),
                (&mut grant.issuer, &policy.issuer),
                (&mut grant.role, &policy.role),
                (&mut grant.network_id, &policy.network_id),
                (&mut grant.application_id, &policy.application_id),
            ] {
                if value.is_empty() {
                    value.clone_from(expected);
                } else if value != expected {
                    return Err(invalid(
                        "receiver grant conflicts with fleet install policy",
                    ));
                }
            }
        }
        Ok(())
    }

    pub(crate) fn receiver_selected(&self, name: &str) -> bool {
        self.canisters
            .get(name)
            .and_then(|s| s.role.as_ref())
            .is_some_and(|role| {
                self.receiver_policy
                    .as_ref()
                    .is_some_and(|policy| policy.roles.contains(role))
            })
    }

    /// Compile a fresh-install payload for a selected component and resolved Root.
    /// The caller must retain these exact bytes with its install approval.
    /// # Errors
    /// Refuses unselected roles, invalid policy, invalid Root or oversized application data.
    pub fn receiver_install_args(
        &self,
        name: &str,
        root: &str,
        application: Vec<u8>,
    ) -> Result<Vec<u8>> {
        let policy = self
            .receiver_policy
            .as_ref()
            .filter(|_| self.receiver_selected(name))
            .ok_or_else(|| invalid("component has no selected receiver policy"))?;
        policy.validate(&self.canisters)?;
        if self.canisters[&policy.root]
            .id
            .as_deref()
            .is_some_and(|id| id != root)
        {
            return Err(invalid("receiver install differs from the configured Root"));
        }
        let root = principal(root)?;
        if root.as_slice().last() != Some(&1) {
            return Err(invalid("receiver needs a Root canister"));
        }
        Install {
            storage: policy.storage,
            root,
            issuer: principal(&policy.issuer)?,
            network: hex32(&policy.network_id)?,
            audience: hex32(&policy.application_id)?,
            role: policy.role.clone(),
            limits: policy.limits.clone(),
            application,
        }
        .encode()
        .map_err(supplier)
    }
}
