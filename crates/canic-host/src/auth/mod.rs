//! Optional explicit Auth approvals; the existing journal owns signed attempts.
//! These records prove configuration replies, never receiver enrollment or readiness.
mod execution;
#[cfg(test)]
mod tests;

use crate::{
    Error, Plan, PlanAction, Result, SelectedFleet, UpgradeTarget, artifact, plan, store, supplier,
};
use canic::auth::{enrollment as wire, protocol::model};
pub(crate) use execution::{apply, status};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Sole controller expected on the enrolled receiver, resolved under the immutable plan.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReceiverController {
    /// Root controls the receiver (the original v5 contract).
    #[default]
    Root,
    /// The operator named by the plan controls the receiver independently.
    Operator,
}
impl ReceiverController {
    fn is_root(&self) -> bool {
        *self == Self::Root
    }
}

/// Optional fleet-owned approval input; names select existing app canister definitions.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AuthConfig {
    /// Explicit authority slot, not inferred from a role or tree edge.
    pub root: String,
    /// Reviewed Root Wasm when supplied by this fleet without an app artifact.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub root_wasm: Option<PathBuf>,
    /// Controller binding used when the operator approves this fleet's grant.
    #[serde(default, skip_serializing_if = "ReceiverController::is_root")]
    pub receiver_controller: ReceiverController,
    /// Exact receiver slot.
    pub receiver: String,
    /// Application-approved issuer principal.
    pub issuer: String,
    /// Ed25519 public key as 64 lowercase hexadecimal digits.
    pub key: String,
    /// Canonical network identity; not the Host transport/trust digest.
    pub network_id: String,
    /// Logical application identity; independent of display names and fleet instances.
    pub application_id: String,
    /// Explicit application role, not a canister role.
    pub role: String,
    /// Canonically ordered unique application scopes.
    pub scopes: Vec<String>,
    /// Inclusive authority start in Unix nanoseconds.
    pub not_before_ns: u64,
    /// Exclusive authority deadline in Unix nanoseconds.
    pub expires_at_ns: u64,
}

fn invalid(message: &str) -> Error {
    Error::Invalid(message.into())
}
fn hex32(value: &str) -> Result<[u8; 32]> {
    if value.len() != 64 || value != value.to_ascii_lowercase() {
        return Err(invalid(
            "Auth identities and keys require 64 lowercase hexadecimal digits",
        ));
    }
    crate::unhex(value)?
        .try_into()
        .map_err(|_| invalid("Auth identity must be 32 bytes"))
}
fn principal(value: &str) -> Result<wire::types::Principal> {
    let id = crate::network::principal(value)?;
    if id.to_text() != value || id.as_slice().is_empty() || value == "2vxsx-fae" {
        return Err(invalid("Auth requires canonical non-anonymous principals"));
    }
    Ok(wire::types::Principal::from_slice(id.as_slice()))
}
impl AuthConfig {
    pub(crate) fn grant(&self, fleet: &crate::FleetConfig) -> Result<wire::Grant> {
        if self.root == self.receiver {
            return Err(invalid("Auth authority and receiver must be distinct"));
        }
        let _ = self.canister(fleet, &self.root)?;
        let receiver = self.canister(fleet, &self.receiver)?;
        let grant = wire::Grant {
            receiver: principal(receiver)?,
            issuer: principal(&self.issuer)?,
            key: hex32(&self.key)?,
            audience: model::AudienceId {
                canonical_network_id: hex32(&self.network_id)?,
                fleet_id: hex32(&self.application_id)?,
            },
            grant: model::DelegatedRoleGrant {
                target: ic_wire::Text::new(self.role.clone()).map_err(supplier)?,
                scopes: ic_wire::List::new(
                    self.scopes
                        .iter()
                        .map(|scope| ic_wire::Text::new(scope.clone()))
                        .collect::<std::result::Result<_, _>>()
                        .map_err(supplier)?,
                )
                .map_err(supplier)?,
            },
            not_before_ns: self.not_before_ns,
            expires_at_ns: self.expires_at_ns,
        };
        validate_grant(&grant)?;
        Ok(grant)
    }
    fn canister<'a>(&self, fleet: &'a crate::FleetConfig, slot: &str) -> Result<&'a str> {
        fleet
            .canisters
            .get(slot)
            .and_then(|c| c.id.as_deref())
            .ok_or_else(|| invalid("Auth requires explicitly bound Root and receiver slots"))
    }
}
fn validate_grant(grant: &wire::Grant) -> Result<()> {
    principal(&grant.receiver.to_text())?;
    principal(&grant.issuer.to_text())?;
    // Grammar belongs to the passive supplier types; key verification remains at Root.
    let _: wire::types::AuthRole = grant.grant.target.as_str().parse().map_err(supplier)?;
    let scopes = grant.grant.scopes.as_slice();
    for scope in scopes {
        let _: wire::types::ScopeLabel = scope.as_str().parse().map_err(supplier)?;
    }
    if scopes.is_empty()
        || scopes.windows(2).any(|s| s[0].as_str() >= s[1].as_str())
        || grant.not_before_ns >= grant.expires_at_ns
    {
        return Err(invalid(
            "Auth needs ordered unique scopes and a finite grant window",
        ));
    }
    Ok(())
}

/// One exact configuration or terminal revocation. Contains no runtime engine.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AuthApproval {
    /// Exact authority executable; retained artifact bytes bind this hash too.
    pub root: UpgradeTarget,
    /// Exact receiver executable.
    pub receiver: UpgradeTarget,
    /// Complete immutable grant, including original validity window.
    pub grant: wire::Grant,
    /// Terminal revocation of this exact grant; false configures a fresh Root.
    pub revoke: bool,
    /// Omitted for historical Root-owned approvals, preserving their exact bytes.
    #[serde(default, skip_serializing_if = "ReceiverController::is_root")]
    pub receiver_controller: ReceiverController,
}
impl AuthApproval {
    /// Resolve the sole receiver controller using the operator retained in the same plan.
    #[must_use]
    pub fn expected_receiver_controller<'a>(&'a self, operator: &'a str) -> &'a str {
        match self.receiver_controller {
            ReceiverController::Root => &self.root.canister,
            ReceiverController::Operator => operator,
        }
    }
}

/// Historical successful Root response, not a current readiness observation.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct AuthReceipt {
    /// Original approved operation.
    pub operation: String,
    /// Whether the exact call revoked its grant.
    pub revoked: bool,
}

pub(crate) fn validate(plan: &Plan, approval: &AuthApproval) -> Result<()> {
    for target in [&approval.root, &approval.receiver] {
        principal(&target.canister)?;
        hex32(&target.module)?;
    }
    if approval.root.canister == approval.receiver.canister
        || approval.receiver.canister != approval.grant.receiver.to_text()
        || approval.root.module != plan.artifact.sha256
    {
        return Err(invalid(
            "Auth approval differs from its exact code or receiver binding",
        ));
    }
    validate_grant(&approval.grant)
}

/// Retain the selected fleet's exact grant; sends nothing and holds no signing keys.
/// # Errors
/// Refuses absent bindings/artifacts, conflicting approvals or invalid grant bounds.
pub fn create_auth_plan(
    journal: &Path,
    config_path: &Path,
    selected: &SelectedFleet,
    controller: &str,
) -> Result<(String, Plan)> {
    selected.validate()?;
    let config = selected
        .config
        .auth
        .as_ref()
        .ok_or_else(|| invalid("selected fleet has no Auth declaration"))?;
    let grant = config.grant(&selected.config)?;
    let supplied = config
        .root_wasm
        .as_ref()
        .map(|path| config_path.with_file_name(path));
    let root_bytes = store::read(
        &selected.wasm_path(config_path, &config.root, supplied.as_deref())?,
        artifact::MAX_ARTIFACT_BYTES,
    )?;
    let root_artifact = artifact::admit_bounded(&root_bytes, artifact::MAX_ARTIFACT_BYTES)?;
    let receiver_bytes = store::read(
        &selected.wasm_path(config_path, &config.receiver, None)?,
        artifact::MAX_ARTIFACT_BYTES,
    )?;
    let receiver_artifact = artifact::admit_bounded(&receiver_bytes, artifact::MAX_ARTIFACT_BYTES)?;
    let now = crate::now()?;
    let proposed = Plan {
        version: 5,
        network: selected.config.network.clone(),
        controller: controller.into(),
        action: PlanAction::Auth(Box::new(AuthApproval {
            root: UpgradeTarget {
                canister: config.canister(&selected.config, &config.root)?.into(),
                module: root_artifact.sha256.clone(),
            },
            receiver: UpgradeTarget {
                canister: grant.receiver.to_text(),
                module: receiver_artifact.sha256,
            },
            grant,
            revoke: false,
            receiver_controller: config.receiver_controller,
        })),
        artifact: root_artifact,
        deployment: Some(crate::DeploymentName {
            app: selected.app.clone(),
            fleet: selected.fleet.clone(),
            canister: config.root.clone(),
        }),
        created_at_ns: now,
        expires_at_ns: now + 3_600_000_000_000,
    };
    let held = store::Store::open(journal)?;
    retain(&held, proposed, &root_bytes)
}

/// Plan terminal revocation from retained configuration, independent of edited config.
/// Requires a retained successful configuration reply before admitting revocation.
/// # Errors
/// Refuses a different fleet, source action or competing exact approval.
pub fn create_auth_revocation(
    journal: &Path,
    operation: &str,
    selected: &SelectedFleet,
) -> Result<(String, Plan)> {
    selected.validate()?;
    let held = store::Store::open(journal)?;
    let (mut plan, bytes) = plan::load(&held.operation(operation)?, operation)?;
    if plan.network != selected.config.network
        || !plan
            .deployment
            .as_ref()
            .is_some_and(|d| d.app == selected.app && d.fleet == selected.fleet)
    {
        return Err(invalid("Auth approval does not belong to selected fleet"));
    }
    let PlanAction::Auth(approval) = &mut plan.action else {
        return Err(invalid("Auth configuration operation required"));
    };
    if approval.revoke {
        return Err(invalid("select the original Auth configuration operation"));
    }
    if execution::receipt(&held.operation(operation)?, &plan)?.is_none() {
        return Err(Error::Blocked(
            "configuration has no retained successful reply; reconcile its original attempt first"
                .into(),
        ));
    }
    let PlanAction::Auth(approval) = &mut plan.action else {
        unreachable!("checked above")
    };
    approval.revoke = true;
    plan.created_at_ns = crate::now()?;
    plan.expires_at_ns = plan.created_at_ns + 3_600_000_000_000;
    retain(&held, plan, &bytes)
}

fn plans(journal: &Path) -> Result<Vec<(String, Plan)>> {
    let (ids, issues) = crate::fleet::operation_ids(journal, true)?;
    if !issues.is_empty() {
        return Err(Error::Blocked(
            "repair unreadable journal before Auth planning".into(),
        ));
    }
    let mut plans = Vec::new();
    for id in ids {
        let dir = journal.join(&id);
        if !crate::fleet_plan::exists(&dir)? {
            plans.push((id.clone(), plan::load(&dir, &id)?.0));
        }
    }
    Ok(plans)
}
fn retain(held: &store::Store, plan: Plan, bytes: &[u8]) -> Result<(String, Plan)> {
    let approval = plan.auth().expect("Auth caller");
    let mut retained = None;
    let mut upgrade = None;
    for (id, existing) in plans(&held.path)? {
        if existing.network == plan.network
            && existing.target().is_some_and(|target| {
                target == &approval.root.canister || target == &approval.receiver.canister
            })
        {
            upgrade = Some(id.clone());
        }
        let Some(old) = existing.auth() else {
            continue;
        };
        if existing.network != plan.network || old.root.canister != approval.root.canister {
            continue;
        }
        let mut comparable = existing.clone();
        if let PlanAction::Auth(value) = &mut comparable.action {
            value.revoke = approval.revoke;
        }
        if !plan::same_authority(&comparable, &plan) {
            return Err(Error::Blocked(format!(
                "Root already has a different Auth approval {id}"
            )));
        }
        if old.revoke == approval.revoke && retained.replace((id, existing)).is_some() {
            return Err(Error::Blocked(
                "multiple Auth approvals; preserve original evidence".into(),
            ));
        }
    }
    if let Some(existing) = retained {
        return Ok(existing);
    }
    if !approval.revoke
        && let Some(id) = upgrade
    {
        return Err(Error::Blocked(format!(
            "Auth requires a fresh owner without an admitted upgrade ({id}); volatile-state migration is not qualified"
        )));
    }
    if !approval.revoke
        && (crate::now()? < approval.grant.not_before_ns
            || crate::now()? >= approval.grant.expires_at_ns)
    {
        return Err(invalid(
            "Auth configuration needs a currently valid grant window",
        ));
    }
    plan::publish_locked(held, plan, bytes)
}

pub(crate) fn permit_upgrade(journal: &Path, plan: &Plan) -> Result<()> {
    let Some(target) = plan.target() else {
        return Ok(());
    };
    for (_, old) in plans(journal)? {
        if old.network == plan.network
            && old
                .auth()
                .is_some_and(|a| &a.root.canister == target || &a.receiver.canister == target)
        {
            return Err(Error::Blocked("Auth has volatile replay/revocation state; this journal's Root or receiver cannot be upgraded through a preservation workflow".into()));
        }
    }
    Ok(())
}
