//! One immutable Root-and-children approval composed from the existing effect owners.
use crate::{
    ApplyOutcome, Artifact, DeploymentName, Error, ManagedOwner, Plan, Receipt, Result,
    SelectedFleet, artifact, operation, plan,
    store::{self, Store},
    supplier,
};
use canic_management_protocol::children::Slot;
use serde::{Deserialize, Serialize};
use std::path::Path;

mod cancellation;
mod convergence;
mod record;
#[cfg(test)]
mod tests;
pub use cancellation::{Cancellation, cancel_fleet_plan};
pub use convergence::FleetPlanning;
pub(crate) use convergence::permits_upgrade;

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
    /// Permanent runtime child identity, independent of config ordering.
    pub slot: Slot,
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

/// Bounded first fleet journey: one new Root and one or two application children.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FleetPlan {
    /// Fleet envelope format, independent of existing single-canister plan formats.
    pub version: u32,
    /// Exact Root creation/install approval; its budget also funds the child's grant.
    pub root: Plan,
    /// Approved children in permanent slot order.
    pub children: Vec<FleetChild>,
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
        let mut domain = format!("canic:fleet-plan:v{}\0", self.version).into_bytes();
        domain.extend(bytes);
        Ok(artifact::digest(&domain))
    }
    pub(crate) fn children(&self) -> impl Iterator<Item = (Slot, &FleetChild)> {
        self.children.iter().map(|child| (child.slot, child))
    }
    fn child(&self, slot: Slot) -> Result<&FleetChild> {
        self.children
            .iter()
            .find(|child| child.slot == slot)
            .ok_or_else(invalid)
    }
    fn validate(&self) -> Result<()> {
        self.root.validate()?;
        let root = self.root.deployment.as_ref().ok_or_else(invalid)?;
        if self.children.is_empty()
            || self.children.len() > SLOTS.len()
            || !self
                .children
                .iter()
                .map(|child| child.slot)
                .eq(SLOTS[..self.children.len()].iter().copied())
            || self.version != self.children.len() as u32
            || !matches!(self.root.action, crate::PlanAction::Create { .. })
            || self.root.version != 2
            || self.root.creation_cycles() < (1 + self.children.len() as u64) * 1_000_000_000_000
            || self
                .children
                .iter()
                .map(|c| &c.deployment.canister)
                .collect::<std::collections::BTreeSet<_>>()
                .len()
                != self.children.len()
        {
            return Err(invalid());
        }
        for (_, child) in self.children() {
            child.deployment.validate()?;
            if child.deployment.app != root.app
                || child.deployment.fleet != root.fleet
                || child.deployment.canister == root.canister
                || child.parent.operation != self.root.digest()?
                || child.artifact.bytes > canic_management_protocol::MAX_WASM
                || child.creation_cycles != canic_management_protocol::CREATION_CYCLES
                || crate::unhex(&child.artifact.sha256)?.len() != 32
                || child.execute_at_ns <= self.root.created_at_ns
                || child.execute_at_ns != self.children[0].execute_at_ns
                || child.execute_at_ns.checked_add(600_000_000_000) != Some(self.root.expires_at_ns)
                || child.execute_at_ns - self.root.created_at_ns > 3_000_000_000_000
            {
                return Err(invalid());
            }
        }
        if self
            .children()
            .map(|(_, c)| c.artifact.bytes)
            .sum::<usize>()
            > canic_management_protocol::children::MAX_ARTIFACT_BYTES
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
    fn child_plan(&self, receipt: &Receipt, slot: Slot) -> Result<Plan> {
        let child = self.child(slot)?;
        if receipt.operation != child.parent.operation
            || receipt.module_hash != self.root.artifact.sha256
            || receipt.controller != self.root.controller
        {
            return Err(invalid());
        }
        let plan = Plan {
            version: 2,
            network: self.root.network.clone(),
            controller: self.root.controller.clone(),
            artifact: child.artifact.clone(),
            action: crate::PlanAction::CreateChild(ManagedOwner {
                slot: (self.version == 2).then_some(slot),
                root: receipt.canister.clone(),
                root_module: self.root.artifact.sha256.clone(),
                execute_at_ns: child.execute_at_ns,
            }),
            created_at_ns: self.root.created_at_ns,
            expires_at_ns: child.execute_at_ns + 600_000_000_000,
            deployment: Some(child.deployment.clone()),
        };
        plan.validate()?;
        Ok(plan)
    }
}
fn invalid() -> Error {
    Error::Invalid("invalid Root-and-children fleet approval or retained output binding".into())
}

pub(crate) fn exists(dir: &Path) -> Result<bool> {
    dir.join("fleet.json").try_exists().map_err(supplier)
}

/// Load and admit the whole envelope and every artifact before any effect.
pub(crate) fn load(dir: &Path, id: &str) -> Result<FleetPlan> {
    let value: FleetPlan = store::json(&dir.join("fleet.json"))?;
    if value.digest()? != id {
        return Err(invalid());
    }
    let (root, _) = plan::load(&dir.join("root"), &value.root.digest()?)?;
    if root != value.root {
        return Err(invalid());
    }
    for (index, child) in value.children() {
        let path = dir.join(record::child_leaf(index));
        if artifact::admit(&store::read(&path.join("module.wasm"), artifact::MAX_WASM)?)?
            != child.artifact
        {
            return Err(invalid());
        }
        if path.join("plan.json").try_exists().map_err(supplier)? {
            let receipt = root_receipt(dir, &value)?.ok_or_else(invalid)?;
            let expected = value.child_plan(&receipt, index)?;
            if plan::load(&path, &expected.digest()?)?.0 != expected {
                return Err(invalid());
            }
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
    value.child_plan(&receipt, Slot::First)?;
    Ok(Some(receipt))
}

/// Compare artifacts with retained completion and approve creation, child upgrade or Root maintenance.
/// Unfinished identical work retains its original schedule, even after expiry. No network calls.
/// # Errors
/// Refuses anything other than an unbound Root with one or two direct children, occupied fleet authority,
/// unavailable artifacts, invalid budgets, journal corruption or concurrent writers.
pub fn plan_fleet(
    journal: &Path,
    config_path: &Path,
    selected: &SelectedFleet,
    controller: &str,
    cycles: Option<u64>,
    execute_at_ns: Option<u64>,
) -> Result<FleetPlanning> {
    selected.validate()?;
    let slots = &selected.config.canisters;
    if slots
        .values()
        .any(|s| s.role.as_deref() == Some("wasm-store"))
    {
        return Err(Error::Invalid(
            "Store bootstrap is not qualified yet; publish into an existing bound Store with --publish-version".into(),
        ));
    }
    if !(2..=SLOTS.len() + 1).contains(&slots.len())
        || slots.values().any(|s| s.id.is_some() || s.profile_manager)
    {
        return Err(invalid());
    }
    let (root_name, _) = slots
        .iter()
        .find(|(_, s)| s.role.as_deref() == Some("root") && s.parent.is_none())
        .ok_or_else(invalid)?;
    let children: Vec<_> = slots
        .iter()
        .filter(|(_, s)| s.parent.as_ref() == Some(root_name))
        .collect();
    if children.len() + 1 != slots.len() {
        return Err(invalid());
    }
    let root_bytes = store::read(
        &selected.wasm_path(config_path, root_name, None)?,
        artifact::MAX_WASM,
    )?;
    let child_bytes: Vec<_> = children
        .iter()
        .map(|(name, _)| {
            store::read(
                &selected.wasm_path(config_path, name, None)?,
                artifact::MAX_WASM,
            )
        })
        .collect::<Result<_>>()?;
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
        action: crate::PlanAction::Create {
            cycles: cycles.unwrap_or((children.len() as u64 + 2) * 1_000_000_000_000),
        },
        created_at_ns: now,
        expires_at_ns: dispatch.checked_add(600_000_000_000).ok_or_else(invalid)?,
        deployment: Some(name(root_name)),
    };
    let definition = |index: usize| -> Result<FleetChild> {
        Ok(FleetChild {
            slot: SLOTS[index],
            deployment: name(children[index].0),
            parent: CreatedCanister {
                operation: String::new(),
            },
            artifact: artifact::admit(&child_bytes[index])?,
            creation_cycles: canic_management_protocol::CREATION_CYCLES,
            execute_at_ns: dispatch,
        })
    };
    let mut value = FleetPlan {
        version: children.len() as u32,
        children: (0..children.len()).map(definition).collect::<Result<_>>()?,
        root,
    };
    let store = Store::open(journal)?;
    // One fleet reservation covers Root and its children before any ID is known.
    let (ids, issues) = crate::fleet::operation_ids(journal, true)?;
    if !issues.is_empty() {
        return Err(Error::Invalid(
            "repair unreadable journal evidence before planning a fleet".into(),
        ));
    }
    let mut retained = None;
    let mut individual = Vec::new();
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
                    && cycles.is_none_or(|budget| old.root.creation_cycles() == budget)
                    && old.root.deployment == value.root.deployment
                    && old
                        .children()
                        .map(|(_, c)| &c.deployment)
                        .eq(value.children().map(|(_, c)| &c.deployment))
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
            if old.publication().is_some() {
                continue;
            }
            if old
                .deployment
                .as_ref()
                .is_some_and(|n| n.app == selected.app && n.fleet == selected.fleet)
            {
                individual.push((id, old));
            }
        }
    }
    if let Some((id, old)) = retained {
        return convergence::plan(
            &store,
            &id,
            old,
            value,
            (&root_bytes, &child_bytes),
            &individual,
        );
    }
    if !individual.is_empty() {
        return Err(Error::Blocked("fleet contains single-canister operations; whole-fleet planning requires its retained creation approval".into()));
    }
    for child in &mut value.children {
        child.parent.operation = value.root.digest()?;
    }
    let id = value.digest()?;
    let dir = store.operation(&id)?;
    let staging = ic_host_fs::directory::create_unique_directory(
        journal,
        ".fleet-plan",
        0o700,
        std::num::NonZeroUsize::new(32),
    )
    .map_err(supplier)?;
    for leaf in std::iter::once("root").chain(value.children().map(|(i, _)| record::child_leaf(i)))
    {
        ic_host_fs::durable::prepare_private_parents(&staging.join(leaf)).map_err(supplier)?;
    }
    store::put(&staging.join("root/module.wasm"), &root_bytes)?;
    store::put_json(&staging.join("root/plan.json"), &value.root)?;
    for (index, bytes) in child_bytes.iter().enumerate() {
        store::put(
            &staging
                .join(record::child_leaf(SLOTS[index]))
                .join("module.wasm"),
            bytes,
        )?;
    }
    store::put_json(&staging.join("fleet.json"), &value)?;
    ic_host_fs::durable::commit_existing_directory_with(
        &staging,
        &dir,
        |_, path| load(path, &id).map(|_| ()),
        |_, _| Ok(()),
    )
    .map_err(supplier)?;
    Ok(FleetPlanning::Create {
        operation: id,
        fleet_plan: value,
    })
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
    if crate::now()? >= value.children[0].execute_at_ns
        && !dir
            .join("root/create/signed.json")
            .try_exists()
            .map_err(supplier)?
    {
        return Err(Error::Blocked(
            "fleet child admission time passed; no Root creation was signed".into(),
        ));
    }
    let root = operation::apply_at(&dir.join("root"), &value.root.digest()?, identity, icp).await?;
    let ApplyOutcome::Complete(root) = root else {
        return Err(invalid());
    };
    let mut outcomes = Vec::new();
    for (index, definition) in value.children() {
        let child = value.child_plan(&root, index)?;
        let child_id = child.digest()?;
        let path = dir.join(record::child_leaf(index));
        if !path.join("plan.json").try_exists().map_err(supplier)? {
            store::put_json(&path.join("plan.json"), &child)?;
        }
        load(dir, id)?;
        outcomes.push(crate::operation::ChildOutcome {
            name: definition.deployment.canister.clone(),
            outcome: operation::apply_at(&path, &child_id, identity, icp).await?,
        });
    }
    Ok(ApplyOutcome::Fleet {
        operation: id.into(),
        root,
        children: outcomes,
    })
}

pub(crate) async fn reconcile(
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
    for (index, _) in value.children() {
        if !dir
            .join(record::child_leaf(index))
            .join("plan.json")
            .try_exists()
            .map_err(supplier)?
        {
            return Err(Error::Blocked("reconciliation requires retained child admission for every approved slot; resume the original apply first".into()));
        }
    }
    let root = root_receipt(dir, &value)?.ok_or_else(invalid)?;
    let mut outcomes = Vec::new();
    for (index, definition) in value.children() {
        let path = dir.join(record::child_leaf(index));
        outcomes.push(crate::operation::ChildOutcome {
            name: definition.deployment.canister.clone(),
            outcome: operation::reconcile_at(
                &path,
                &value.child_plan(&root, index)?.digest()?,
                identity,
                icp,
            )
            .await?,
        });
    }
    Ok(ApplyOutcome::Fleet {
        operation: id.into(),
        root,
        children: outcomes,
    })
}

pub(crate) fn status(dir: &Path, id: &str) -> Result<serde_json::Value> {
    let value = load(dir, id)?;
    let root = operation::status_at(&dir.join("root"), &value.root.digest()?)?;
    let mut result = serde_json::json!({"operation": id, "fleet_plan": value, "root": root, "cancellation": cancellation::read(dir,id)?});
    let mut children = Vec::new();
    for (index, definition) in value.children() {
        let path = dir.join(record::child_leaf(index));
        let child = if path.join("plan.json").try_exists().map_err(supplier)? {
            let receipt = root_receipt(dir, &value)?.ok_or_else(invalid)?;
            operation::status_at(&path, &value.child_plan(&receipt, index)?.digest()?)?
        } else {
            serde_json::Value::Null
        };
        children.push(serde_json::json!({"name": definition.deployment.canister, "state": child}));
    }
    result["children"] = children.into();
    Ok(result)
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
        &value.root.digest()?,
        now,
    )?];
    for (index, _) in value.children() {
        let path = dir.join(record::child_leaf(index));
        if path.join("plan.json").try_exists().map_err(supplier)? {
            let receipt = root_receipt(dir, &value)?.ok_or_else(invalid)?;
            records.push(crate::fleet::local_operation_at(
                &path,
                &value.child_plan(&receipt, index)?.digest()?,
                now,
            )?);
        }
    }
    for (record, _) in &mut records {
        record.operation = id.into();
    }
    Ok(records)
}

pub(crate) fn cancelled(dir: &Path, id: &str) -> Result<bool> {
    cancellation::read(dir, id).map(|record| record.is_some())
}

pub(crate) const SLOTS: [Slot; 2] = [Slot::First, Slot::Second];
