//! One immutable Root-plus-app approval composed from the existing effect owners.
use crate::{
    ApplyOutcome, Artifact, DeploymentName, Error, ManagedOwner, Plan, Receipt, Result,
    SelectedFleet, artifact, operation, plan,
    store::{self, Store},
    supplier,
};
use serde::{Deserialize, Serialize};
use std::path::Path;

mod cancellation;
#[cfg(test)]
mod tests;
pub use cancellation::{Cancellation, cancel_fleet_plan};

/// A typed result reference, never a guessed principal or a mutable config lookup.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CreatedCanister {
    /// Digest of the approved creation whose retained result resolves this reference.
    pub operation: String,
}

/// Child installation authorized before its Root principal is known.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FleetChild {
    /// App/fleet/slot identity.
    pub deployment: DeploymentName,
    /// Sole controller and effect owner, resolved from the Root's creation result.
    pub parent: CreatedCanister,
    /// Exact retained application executable.
    pub artifact: Artifact,
    /// Child attachment drawn from Root's funded budget, not another host debit.
    pub creation_cycles: u64,
    /// Earliest autonomous dispatch; admission must precede this time.
    pub execute_at_ns: u64,
}

/// Bounded first fleet journey: one new Root and its one new application child.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FleetPlan {
    /// Fleet envelope format, independent of existing single-canister plan formats.
    pub version: u32,
    /// Exact Root creation/install approval; its budget also funds the child's grant.
    pub root: Plan,
    /// Approved child definition using the Root's typed output.
    pub child: FleetChild,
}

impl FleetPlan {
    /// Digest of bounded Wire-encoded authority, independent of JSON formatting.
    /// # Errors
    /// Rejects invalid authority, output references or unbounded inputs.
    pub fn digest(&self) -> Result<String> {
        self.validate()?;
        let bytes = ic_wire::native::encode(
            self,
            ic_wire::Limits {
                bytes: 16_384,
                allocation: 262_144,
                nodes: 1024,
                depth: 16,
            },
        )
        .map_err(supplier)?;
        let mut domain = b"canic:fleet-plan:v1\0".to_vec();
        domain.extend(bytes);
        Ok(artifact::digest(&domain))
    }
    fn validate(&self) -> Result<()> {
        self.root.validate()?;
        self.child.deployment.validate()?;
        let Some(root) = &self.root.deployment else {
            return Err(invalid());
        };
        if self.version != 1
            || self.root.target.is_some()
            || self.root.management.is_some()
            || self.root.version != 2
            || self.root.creation_cycles < 2_000_000_000_000
            || self.child.deployment.app != root.app
            || self.child.deployment.fleet != root.fleet
            || self.child.deployment.canister == root.canister
            || self.child.parent.operation != self.root.digest()?
            || self.child.artifact.bytes > canic_management_protocol::MAX_WASM
            || self.child.creation_cycles != canic_management_protocol::CREATION_CYCLES
            || crate::unhex(&self.child.artifact.sha256)?.len() != 32
            || self.child.execute_at_ns <= self.root.created_at_ns
            || self.child.execute_at_ns.checked_add(600_000_000_000)
                != Some(self.root.expires_at_ns)
            || self.child.execute_at_ns - self.root.created_at_ns > 3_000_000_000_000
        {
            return Err(invalid());
        }
        Ok(())
    }
    pub(crate) fn belongs(&self, selection: &SelectedFleet) -> bool {
        self.root.deployment.as_ref().is_some_and(|name| {
            name.app == selection.app
                && name.fleet == selection.fleet
                && self.root.network == selection.config.network
        })
    }
    pub(crate) fn reserves(&self, name: &DeploymentName) -> bool {
        self.root
            .deployment
            .as_ref()
            .is_some_and(|root| root.app == name.app && root.fleet == name.fleet)
    }
    fn child_plan(&self, receipt: &Receipt) -> Result<Plan> {
        if receipt.operation != self.child.parent.operation
            || receipt.module_hash != self.root.artifact.sha256
            || receipt.controller != self.root.controller
        {
            return Err(invalid());
        }
        let plan = Plan {
            version: 2,
            network: self.root.network.clone(),
            controller: self.root.controller.clone(),
            artifact: self.child.artifact.clone(),
            target: None,
            previous_module: None,
            creation_cycles: self.child.creation_cycles,
            created_at_ns: self.root.created_at_ns,
            expires_at_ns: self.child.execute_at_ns + 600_000_000_000,
            deployment: Some(self.child.deployment.clone()),
            management: Some(ManagedOwner {
                root: receipt.canister.clone(),
                root_module: self.root.artifact.sha256.clone(),
                execute_at_ns: self.child.execute_at_ns,
            }),
        };
        plan.validate()?;
        Ok(plan)
    }
}
fn invalid() -> Error {
    Error::Invalid("invalid Root-plus-app fleet approval or retained output binding".into())
}

pub(crate) fn exists(dir: &Path) -> Result<bool> {
    dir.join("fleet.json").try_exists().map_err(supplier)
}

/// Load and admit the whole envelope and both artifacts before any effect.
pub(crate) fn load(dir: &Path, id: &str) -> Result<FleetPlan> {
    let value: FleetPlan = store::json(&dir.join("fleet.json"))?;
    if value.digest()? != id {
        return Err(invalid());
    }
    let (root, _) = plan::load(&dir.join("root"), &value.child.parent.operation)?;
    if root != value.root
        || artifact::admit(&store::read(
            &dir.join("child/module.wasm"),
            artifact::MAX_WASM,
        )?)? != value.child.artifact
    {
        return Err(invalid());
    }
    if dir.join("child/plan.json").try_exists().map_err(supplier)? {
        let receipt = root_receipt(dir, &value)?.ok_or_else(invalid)?;
        let expected = value.child_plan(&receipt)?;
        if plan::load(&dir.join("child"), &expected.digest()?)?.0 != expected {
            return Err(invalid());
        }
    }
    cancellation::read(dir, id)?;
    Ok(value)
}
fn root_receipt(dir: &Path, value: &FleetPlan) -> Result<Option<Receipt>> {
    if !dir
        .join("root/receipt.json")
        .try_exists()
        .map_err(supplier)?
    {
        return Ok(None);
    }
    let receipt: Receipt = store::json(&dir.join("root/receipt.json"))?;
    let created = operation::created(
        &value.root,
        &store::read(&dir.join("root/create/reply.bin"), 2 * 1024 * 1024)?,
    )?;
    if created.to_text() != receipt.canister {
        return Err(invalid());
    }
    value.child_plan(&receipt)?;
    Ok(Some(receipt))
}

/// Retain one complete fleet approval, without network calls or fabricated identities.
/// Identical replanning returns its original schedule and authority, even after expiry.
/// # Errors
/// Refuses anything other than an unbound Root-plus-app tree, occupied fleet authority,
/// unavailable artifacts, invalid budgets, journal corruption or concurrent writers.
pub fn create_fleet_plan(
    journal: &Path,
    config_path: &Path,
    selected: &SelectedFleet,
    controller: &str,
    cycles: u64,
    execute_at_ns: Option<u64>,
) -> Result<(String, FleetPlan)> {
    selected.validate()?;
    let slots = &selected.config.canisters;
    if slots.len() != 2 || slots.values().any(|s| s.id.is_some() || s.profile_manager) {
        return Err(invalid());
    }
    let (root_name, _) = slots
        .iter()
        .find(|(_, s)| s.role.as_deref() == Some("root") && s.parent.is_none())
        .ok_or_else(invalid)?;
    let (child_name, _) = slots
        .iter()
        .find(|(_, s)| s.parent.as_ref() == Some(root_name))
        .ok_or_else(invalid)?;
    let root_bytes = store::read(
        &selected.wasm_path(config_path, root_name, None)?,
        artifact::MAX_WASM,
    )?;
    let child_bytes = store::read(
        &selected.wasm_path(config_path, child_name, None)?,
        artifact::MAX_WASM,
    )?;
    let now = crate::now()?;
    let dispatch = execute_at_ns.unwrap_or(now + 600_000_000_000);
    let name = |slot: &str| DeploymentName {
        app: selected.app.clone(),
        fleet: selected.fleet.clone(),
        canister: slot.into(),
    };
    let root = Plan {
        version: 2,
        network: selected.config.network.clone(),
        controller: controller.into(),
        artifact: artifact::admit(&root_bytes)?,
        target: None,
        previous_module: None,
        creation_cycles: cycles,
        created_at_ns: now,
        expires_at_ns: dispatch.checked_add(600_000_000_000).ok_or_else(invalid)?,
        deployment: Some(name(root_name)),
        management: None,
    };
    let mut value = FleetPlan {
        version: 1,
        child: FleetChild {
            deployment: name(child_name),
            parent: CreatedCanister {
                operation: String::new(),
            },
            artifact: artifact::admit(&child_bytes)?,
            creation_cycles: canic_management_protocol::CREATION_CYCLES,
            execute_at_ns: dispatch,
        },
        root,
    };
    let store = Store::open(journal)?;
    // One fleet reservation covers both slots, including before Root returns its ID.
    let (ids, issues) = crate::fleet::operation_ids(journal, true)?;
    if !issues.is_empty() {
        return Err(Error::Invalid(
            "repair unreadable journal evidence before planning a fleet".into(),
        ));
    }
    let mut retained = None;
    for id in ids {
        let dir = store.operation(&id)?;
        if exists(&dir)? {
            let old = load(&dir, &id)?;
            if cancelled(&dir, &id)? {
                continue;
            }
            if old.reserves(&name(root_name)) {
                if old.belongs(selected)
                    && old.root.controller == controller
                    && old.root.creation_cycles == cycles
                    && old.root.artifact == value.root.artifact
                    && old.child.artifact == value.child.artifact
                    && old.root.deployment == value.root.deployment
                    && old.child.deployment == value.child.deployment
                {
                    if retained.is_some() {
                        return Err(Error::Blocked(
                            "multiple approvals reserve this fleet; inspect the original journal"
                                .into(),
                        ));
                    }
                    retained = Some((id, old));
                    continue;
                }
                return Err(Error::Blocked(format!(
                    "fleet already belongs to operation {id}; original approval is unchanged"
                )));
            }
        } else {
            let (old, _) = plan::load(&dir, &id)?;
            if old
                .deployment
                .as_ref()
                .is_some_and(|n| n.app == selected.app && n.fleet == selected.fleet)
            {
                return Err(Error::Blocked("fleet contains single-canister operations; this first fleet path requires a new fleet".into()));
            }
        }
    }
    if let Some(retained) = retained {
        return Ok(retained);
    }
    value.child.parent.operation = value.root.digest()?;
    let id = value.digest()?;
    let dir = store.operation(&id)?;
    let staging = ic_host_fs::directory::create_unique_directory(
        journal,
        ".fleet-plan",
        0o700,
        std::num::NonZeroUsize::new(32),
    )
    .map_err(supplier)?;
    for leaf in ["root", "child"] {
        ic_host_fs::durable::prepare_private_parents(&staging.join(leaf)).map_err(supplier)?;
    }
    store::put(&staging.join("root/module.wasm"), &root_bytes)?;
    store::put_json(&staging.join("root/plan.json"), &value.root)?;
    store::put(&staging.join("child/module.wasm"), &child_bytes)?;
    store::put_json(&staging.join("fleet.json"), &value)?;
    ic_host_fs::durable::commit_existing_directory_with(
        &staging,
        &dir,
        |_, path| load(path, &id).map(|_| ()),
        |_, _| Ok(()),
    )
    .map_err(supplier)?;
    Ok((id, value))
}

pub(crate) async fn apply(
    dir: &Path,
    id: &str,
    identity: &str,
    icp: &Path,
) -> Result<ApplyOutcome> {
    let value = load(dir, id)?;
    if cancelled(dir, id)? {
        return Err(Error::Blocked(
            "this fleet approval was cancelled; plan and review a new operation".into(),
        ));
    }
    if crate::now()? >= value.child.execute_at_ns
        && !dir
            .join("root/create/signed.json")
            .try_exists()
            .map_err(supplier)?
    {
        return Err(Error::Blocked(
            "fleet child admission time passed; no Root creation was signed".into(),
        ));
    }
    let root = operation::apply_at(
        &dir.join("root"),
        &value.child.parent.operation,
        identity,
        icp,
    )
    .await?;
    let ApplyOutcome::Complete(root) = root else {
        return Err(invalid());
    };
    let child = value.child_plan(&root)?;
    let child_id = child.digest()?;
    let path = dir.join("child/plan.json");
    if !path.try_exists().map_err(supplier)? {
        store::put_json(&path, &child)?;
    }
    // Recheck resolution after publication; never let retained child bytes substitute the output.
    load(dir, id)?;
    let child = operation::apply_at(&dir.join("child"), &child_id, identity, icp).await?;
    Ok(ApplyOutcome::Fleet {
        operation: id.into(),
        root,
        child: Box::new(child),
    })
}

pub(crate) fn status(dir: &Path, id: &str) -> Result<serde_json::Value> {
    let value = load(dir, id)?;
    let root = operation::status_at(&dir.join("root"), &value.child.parent.operation)?;
    let child = if dir.join("child/plan.json").try_exists().map_err(supplier)? {
        let receipt = root_receipt(dir, &value)?.ok_or_else(invalid)?;
        operation::status_at(&dir.join("child"), &value.child_plan(&receipt)?.digest()?)?
    } else {
        serde_json::Value::Null
    };
    Ok(
        serde_json::json!({"operation":id, "fleet_plan":value, "root":root, "child":child, "cancellation":cancellation::read(dir,id)?}),
    )
}

pub(crate) fn inventory(
    dir: &Path,
    id: &str,
    now: u64,
) -> Result<Vec<(crate::FleetOperation, crate::Network)>> {
    let value = load(dir, id)?;
    if cancelled(dir, id)? {
        return Ok(Vec::new());
    }
    let mut records = vec![crate::fleet::local_operation_at(
        &dir.join("root"),
        &value.child.parent.operation,
        now,
    )?];
    if dir.join("child/plan.json").try_exists().map_err(supplier)? {
        let receipt = root_receipt(dir, &value)?.ok_or_else(invalid)?;
        records.push(crate::fleet::local_operation_at(
            &dir.join("child"),
            &value.child_plan(&receipt)?.digest()?,
            now,
        )?);
    }
    for (record, _) in &mut records {
        record.operation = id.into();
    }
    Ok(records)
}

pub(crate) fn cancelled(dir: &Path, id: &str) -> Result<bool> {
    cancellation::read(dir, id).map(|record| record.is_some())
}
