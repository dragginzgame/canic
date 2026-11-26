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
            return Err(invalid(&format!(
                "receiver_policy.root = {:?} must name a declared top-level Root",
                self.root
            )));
        }
        let mut seen = BTreeSet::new();
        crate::config::positive_count("receiver_policy.roles count", self.roles.len(), 32)?;
        for role in &self.roles {
            if matches!(role.as_str(), "root" | "wasm-store")
                || !slots.values().any(|slot| slot.role.as_ref() == Some(role))
            {
                return Err(invalid(&format!(
                    "receiver_policy.roles entry {role:?} must name a declared application role"
                )));
            }
            if !seen.insert(role) {
                return Err(invalid(&format!(
                    "receiver_policy.roles contains duplicate role {role:?}"
                )));
            }
        }
        self.role
            .parse::<canic::auth::enrollment::types::AuthRole>()
            .map_err(supplier)?;
        principal(&self.issuer)?;
        hex32(&self.network_id)?;
        hex32(&self.application_id)?;
        let limits = &self.limits;
        crate::config::positive_count(
            "receiver_policy.limits.max_session_secs",
            limits.max_session_secs,
            1800,
        )?;
        crate::config::positive_count(
            "receiver_policy.limits.default_session_secs",
            limits.default_session_secs,
            limits.max_session_secs,
        )?;
        if limits.max_proof_ttl_ns == 0 {
            return Err(invalid(
                "receiver_policy.limits.max_proof_ttl_ns = 0; expected a positive duration in nanoseconds",
            ));
        }
        if limits.max_admitted_logins == Some(0) {
            return Err(invalid(
                "receiver_policy.limits.max_admitted_logins = 0; expected at least one known login",
            ));
        }
        if limits.max_sessions == Some(0) {
            return Err(invalid(
                "receiver_policy.limits.max_sessions = 0; expected at least one session",
            ));
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

    pub(crate) fn route_operator_role(&self, name: &str) -> Option<&str> {
        self.shard_pools
            .iter()
            .any(|pool| {
                self.canisters
                    .get(&pool.component)
                    .and_then(|c| c.parent.as_deref())
                    == Some(name)
            })
            .then(|| self.canisters.get(name)?.role.as_deref())
            .flatten()
    }

    pub(crate) fn funding_operator_role(&self, name: &str) -> Option<&str> {
        self.top_up_children
            .iter()
            .any(|edge| {
                self.canisters
                    .get(edge.child())
                    .and_then(|c| c.parent.as_deref())
                    == Some(name)
            })
            .then(|| self.canisters.get(name)?.role.as_deref())
            .flatten()
    }

    /// Bind route and payment administration to the operator named by this fresh installation.
    /// Returns the application payload directly for a component without a shard pool.
    /// # Errors
    /// Refuses an invalid operator or a service declaration exceeding the install budget.
    pub fn bind_hub_operator(
        &self,
        name: &str,
        operator: &str,
        application: Vec<u8>,
    ) -> Result<Vec<u8>> {
        let Some(role) = self
            .route_operator_role(name)
            .or_else(|| self.funding_operator_role(name))
        else {
            return Ok(application);
        };
        canic::topology::directory::ServiceInstall {
            hub_operator: self
                .route_operator_role(name)
                .map(|_| crate::network::principal(operator))
                .transpose()?,
            funding_operator: self
                .funding_operator_role(name)
                .map(|_| crate::network::principal(operator))
                .transpose()?,
            name: name.into(),
            role: role.into(),
            application,
        }
        .encode()
        .map_err(invalid)
    }

    /// Bind the DB incarnation and verified issuer independently inside the Auth install.
    /// # Errors
    /// Refuses an unselected service, invalid nonce, invalid Root or Auth policy.
    pub fn db_service_install_args(
        &self,
        name: &str,
        root: &str,
        nonce: &[u8],
        issuer_key: [u8; 32],
        operator: &str,
    ) -> Result<Vec<u8>> {
        use canic::topology::directory::{DbService, ServiceInstall};
        if !self.role_receivers.iter().any(|service| service == name) {
            return Err(invalid("DB service must be selected for role verification"));
        }
        let role = self
            .canisters
            .get(name)
            .and_then(|c| c.role.clone())
            .ok_or_else(|| invalid("missing service role"))?;
        let nonce = nonce
            .try_into()
            .map_err(|_| invalid("DB service needs a 16-byte installation nonce"))?;
        let application = DbService { nonce, issuer_key }.encode().map_err(invalid)?;
        let service = ServiceInstall {
            funding_operator: self
                .funding_operator_role(name)
                .map(|_| crate::network::principal(operator))
                .transpose()?,
            hub_operator: self
                .route_operator_role(name)
                .map(|_| crate::network::principal(operator))
                .transpose()?,
            name: name.into(),
            role,
            application,
        }
        .encode()
        .map_err(invalid)?;
        self.receiver_install_args(name, root, service)
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
