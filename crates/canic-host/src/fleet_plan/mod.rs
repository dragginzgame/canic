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
mod releases;
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
    /// Immutable release in this fleet's approved Store; absent in retained inline fleets.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub release: Option<canic_wasm_store_protocol::Manifest>,
    /// Child attachment drawn from Root's funded budget, not another host debit.
    pub creation_cycles: u64,
    /// Earliest autonomous dispatch; admission must precede this time.
    pub execute_at_ns: u64,
}

/// Infrastructure installation bound to the approved Root creation output.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FleetStore {
    /// App/fleet/canister identity, outside the application slot allocation.
    pub deployment: DeploymentName,
    /// Root's exact approved creation output.
    pub parent: CreatedCanister,
    /// Retained Store executable, installed by the host before handover.
    pub artifact: Artifact,
    /// Separate operator debit, including the ledger fee.
    pub creation_cycles: u64,
}

/// Bounded fleet journey: one new Root, optional Store and one or two application children.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FleetPlan {
    /// Fleet envelope format, independent of existing single-canister plan formats.
    pub version: u32,
    /// Exact Root creation/install approval; its budget also funds the child's grant.
    pub root: Plan,
    /// Approved children in permanent slot order.
    pub children: Vec<FleetChild>,
    /// Optional per-fleet infrastructure, completed before application admission.
    pub store: Option<FleetStore>,
}

impl FleetPlan {
    /// Total operator debit ceiling; child grants are already included in Root funding.
    #[must_use]
    pub fn creation_cycles(&self) -> u64 {
        self.root.creation_cycles() + self.store.as_ref().map_or(0, |s| s.creation_cycles)
    }
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
                // Two full 32-chunk manifests; Wire charges four nodes per visited value.
                nodes: 16_384,
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
                || child.artifact.bytes > child.artifact_limit()
                || child.creation_cycles != canic_management_protocol::CREATION_CYCLES
                || crate::unhex(&child.artifact.sha256)?.len() != 32
                || child.execute_at_ns <= self.root.created_at_ns
                || child.execute_at_ns != self.children[0].execute_at_ns
                || child.execute_at_ns.checked_add(600_000_000_000) != Some(self.root.expires_at_ns)
                || child.execute_at_ns - self.root.created_at_ns > 3_000_000_000_000
            {
                return Err(invalid());
            }
            if let Some(release) = &child.release
                && (self.store.is_none()
                    || !release.valid()
                    || release.release.name != child.deployment.canister
                    || release.release.version != child.artifact.sha256
                    || crate::hex(&release.hash) != child.artifact.sha256
                    || release.bytes != child.artifact.bytes as u64)
            {
                return Err(invalid());
            }
        }
        if let Some(store) = &self.store {
            store.deployment.validate()?;
            if store.deployment.app != root.app
                || store.deployment.fleet != root.fleet
                || store.deployment.canister == root.canister
                || self
                    .children
                    .iter()
                    .any(|c| c.deployment == store.deployment)
                || store.parent.operation != self.root.digest()?
                || store.creation_cycles != STORE_CYCLES
                || store.artifact.bytes > artifact::MAX_WASM
                || crate::unhex(&store.artifact.sha256)?.len() != 32
            {
                return Err(invalid());
            }
        }
        if self
            .children()
            .filter(|(_, c)| c.release.is_none())
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
    fn child_plan(&self, receipt: &Receipt, store: Option<&Receipt>, slot: Slot) -> Result<Plan> {
        let child = self.child(slot)?;
        self.verify_root_receipt(receipt)?;
        let plan = Plan {
            version: 2,
            network: self.root.network.clone(),
            controller: self.root.controller.clone(),
            artifact: child.artifact.clone(),
            action: crate::PlanAction::CreateChild(ManagedOwner {
                source: self.source(receipt, store, slot)?,
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
    fn store_plan(&self, receipt: &Receipt) -> Result<Plan> {
        let store = self.store.as_ref().ok_or_else(invalid)?;
        // The same Root receipt checks bind both kinds of dependent operation.
        self.verify_root_receipt(receipt)?;
        let plan = Plan {
            artifact: store.artifact.clone(),
            action: crate::PlanAction::CreateStore {
                cycles: store.creation_cycles,
                root: crate::UpgradeTarget {
                    canister: receipt.canister.clone(),
                    module: self.root.artifact.sha256.clone(),
                },
            },
            deployment: Some(store.deployment.clone()),
            ..self.root.clone()
        };
        plan.validate()?;
        Ok(plan)
    }
    fn verify_root_receipt(&self, receipt: &Receipt) -> Result<()> {
        if receipt.operation != self.root.digest()?
            || receipt.module_hash != self.root.artifact.sha256
            || receipt.controller != self.root.controller
        {
            return Err(invalid());
        }
        Ok(())
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
        let bytes = store::read(&path.join("module.wasm"), child.artifact_limit())?;
        if artifact::admit_bounded(&bytes, child.artifact_limit())? != child.artifact
            || child.release.as_ref().is_some_and(|m| {
                crate::publication::release_manifest(m.release.clone(), &bytes) != *m
            })
        {
            return Err(invalid());
        }
        if path.join("plan.json").try_exists().map_err(supplier)? {
            let receipt = root_receipt(dir, &value)?.ok_or_else(invalid)?;
            let expected =
                value.child_plan(&receipt, store_receipt(dir, &value)?.as_ref(), index)?;
            if plan::load(&path, &expected.digest()?)?.0 != expected {
                return Err(invalid());
            }
        }
        releases::validate(dir, &value, index, &bytes)?;
    }
    if let Some(definition) = &value.store {
        let path = dir.join("store");
        if artifact::admit(&store::read(&path.join("module.wasm"), artifact::MAX_WASM)?)?
            != definition.artifact
        {
            return Err(invalid());
        }
        if path.join("plan.json").try_exists().map_err(supplier)? {
            let expected = value.store_plan(&root_receipt(dir, &value)?.ok_or_else(invalid)?)?;
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
    value.verify_root_receipt(&receipt)?;
    Ok(Some(receipt))
}

/// Compare artifacts with retained completion and approve creation, child upgrade or Root maintenance.
/// Unfinished identical work retains its original schedule, even after expiry. No network calls.
/// # Errors
/// Refuses anything other than an unbound Root, optional Store and one or two direct applications, occupied fleet authority,
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
    let stores: Vec<_> = slots
        .iter()
        .filter(|(_, s)| s.role.as_deref() == Some("wasm-store"))
        .collect();
    if stores.len() > 1
        || !(2 + stores.len()..=SLOTS.len() + 1 + stores.len()).contains(&slots.len())
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
        .filter(|(_, s)| {
            s.parent.as_ref() == Some(root_name) && s.role.as_deref() != Some("wasm-store")
        })
        .collect();
    if children.len() + 1 + stores.len() != slots.len()
        || stores
            .first()
            .is_some_and(|(_, s)| s.parent.as_ref() != Some(root_name))
    {
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
                if stores.is_empty() {
                    canic_management_protocol::MAX_WASM
                } else {
                    artifact::MAX_ARTIFACT_BYTES
                },
            )
        })
        .collect::<Result<_>>()?;
    let store_bytes = stores
        .first()
        .map(|(name, _)| {
            store::read(
                &selected.wasm_path(config_path, name, None)?,
                artifact::MAX_WASM,
            )
        })
        .transpose()?;
    let store_cycles = if stores.is_empty() { 0 } else { STORE_CYCLES };
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
            cycles: cycles
                .unwrap_or((children.len() as u64 + 2) * 1_000_000_000_000 + store_cycles)
                .checked_sub(store_cycles)
                .ok_or_else(invalid)?,
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
            artifact: artifact::admit_bounded(
                &child_bytes[index],
                if stores.is_empty() {
                    canic_management_protocol::MAX_WASM
                } else {
                    artifact::MAX_ARTIFACT_BYTES
                },
            )?,
            release: (!stores.is_empty()).then(|| {
                crate::publication::release_manifest(
                    canic_wasm_store_protocol::Release {
                        name: children[index].0.clone(),
                        version: artifact::digest(&child_bytes[index]),
                    },
                    &child_bytes[index],
                )
            }),
            creation_cycles: canic_management_protocol::CREATION_CYCLES,
            execute_at_ns: dispatch,
        })
    };
    let mut value = FleetPlan {
        version: children.len() as u32,
        children: (0..children.len()).map(definition).collect::<Result<_>>()?,
        root,
        store: stores
            .first()
            .zip(store_bytes.as_ref())
            .map(|((slot, _), bytes)| -> Result<FleetStore> {
                Ok(FleetStore {
                    deployment: name(slot),
                    parent: CreatedCanister {
                        operation: String::new(),
                    },
                    artifact: artifact::admit(bytes)?,
                    creation_cycles: STORE_CYCLES,
                })
            })
            .transpose()?,
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
    let mut publications = Vec::new();
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
                    && cycles.is_none_or(|budget| old.creation_cycles() == budget)
                    && old.store.as_ref().map(|s| &s.deployment)
                        == value.store.as_ref().map(|s| &s.deployment)
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
                publications.push((id, old));
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
        // Existing approvals keep their delivery contract; a Store alone never migrates inline custody.
        for (child, retained) in value.children.iter_mut().zip(&old.children) {
            if retained.release.is_none() {
                child.release = None;
            }
        }
        return convergence::plan(
            &store,
            &id,
            old,
            value,
            (&root_bytes, &child_bytes, store_bytes.as_deref()),
            &individual,
            &publications,
        );
    }
    if !individual.is_empty() {
        return Err(Error::Blocked("fleet contains single-canister operations; whole-fleet planning requires its retained creation approval".into()));
    }
    for child in &mut value.children {
        child.parent.operation = value.root.digest()?;
    }
    if let Some(store) = &mut value.store {
        store.parent.operation = value.root.digest()?;
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
    if let Some(bytes) = store_bytes {
        ic_host_fs::durable::prepare_private_parents(&staging.join("store")).map_err(supplier)?;
        store::put(&staging.join("store/module.wasm"), &bytes)?;
    }
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
    let store = if value.store.is_some() {
        let plan = value.store_plan(&root)?;
        let path = dir.join("store");
        if !path.join("plan.json").try_exists().map_err(supplier)? {
            store::put_json(&path.join("plan.json"), &plan)?;
        }
        load(dir, id)?;
        let ApplyOutcome::Complete(receipt) =
            operation::apply_at(&path, &plan.digest()?, identity, icp).await?
        else {
            return Err(invalid());
        };
        Some(receipt)
    } else {
        None
    };
    releases::apply(dir, &value, &root, store.as_ref(), identity, icp).await?;
    let mut outcomes = Vec::new();
    for (index, definition) in value.children() {
        let child = value.child_plan(&root, store.as_ref(), index)?;
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
        store,
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
    let store = if value.store.is_some() {
        let root = root_receipt(dir, &value)?.ok_or_else(invalid)?;
        let path = dir.join("store");
        if !path.join("plan.json").try_exists().map_err(supplier)? {
            return Err(Error::Blocked(
                "Store bootstrap has not begun; resume fleet apply".into(),
            ));
        }
        let ApplyOutcome::Complete(receipt) =
            operation::reconcile_at(&path, &value.store_plan(&root)?.digest()?, identity, icp)
                .await?
        else {
            return Err(invalid());
        };
        Some(receipt)
    } else {
        None
    };
    releases::reconcile(dir, &value, identity, icp).await?;
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
                &value.child_plan(&root, store.as_ref(), index)?.digest()?,
                identity,
                icp,
            )
            .await?,
        });
    }
    Ok(ApplyOutcome::Fleet {
        operation: id.into(),
        root,
        store,
        children: outcomes,
    })
}

pub(crate) fn status(dir: &Path, id: &str) -> Result<serde_json::Value> {
    let value = load(dir, id)?;
    let root = operation::status_at(&dir.join("root"), &value.root.digest()?)?;
    let mut result = serde_json::json!({"operation": id, "fleet_plan": value, "root": root, "cancellation": cancellation::read(dir,id)?});
    if let Some(definition) = &value.store {
        result["store"] = serde_json::json!({"name": definition.deployment.canister, "state": store_state(dir, &value)?});
    }
    let mut children = Vec::new();
    for (index, definition) in value.children() {
        let path = dir.join(record::child_leaf(index));
        let child = if path.join("plan.json").try_exists().map_err(supplier)? {
            let receipt = root_receipt(dir, &value)?.ok_or_else(invalid)?;
            operation::status_at(
                &path,
                &value
                    .child_plan(&receipt, store_receipt(dir, &value)?.as_ref(), index)?
                    .digest()?,
            )?
        } else {
            serde_json::Value::Null
        };
        children.push(serde_json::json!({"name": definition.deployment.canister, "state": child, "publication": releases::status(dir, &value, index)?}));
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
    if value.store.is_some() && dir.join("store/plan.json").try_exists().map_err(supplier)? {
        let root = root_receipt(dir, &value)?.ok_or_else(invalid)?;
        records.push(crate::fleet::local_operation_at(
            &dir.join("store"),
            &value.store_plan(&root)?.digest()?,
            now,
        )?);
    }
    for (index, _) in value.children() {
        let path = dir.join(record::child_leaf(index));
        if path.join("plan.json").try_exists().map_err(supplier)? {
            let receipt = root_receipt(dir, &value)?.ok_or_else(invalid)?;
            records.push(crate::fleet::local_operation_at(
                &path,
                &value
                    .child_plan(&receipt, store_receipt(dir, &value)?.as_ref(), index)?
                    .digest()?,
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
const STORE_CYCLES: u64 = 1_000_000_000_000;

fn store_state(dir: &Path, value: &FleetPlan) -> Result<serde_json::Value> {
    if !dir.join("store/plan.json").try_exists().map_err(supplier)? {
        return Ok(serde_json::Value::Null);
    }
    let root = root_receipt(dir, value)?.ok_or_else(invalid)?;
    operation::status_at(&dir.join("store"), &value.store_plan(&root)?.digest()?)
}

fn store_receipt(dir: &Path, value: &FleetPlan) -> Result<Option<Receipt>> {
    let path = dir.join("store");
    if !path.join("plan.json").try_exists().map_err(supplier)? {
        return Ok(None);
    }
    let root = root_receipt(dir, value)?.ok_or_else(invalid)?;
    let (plan, bytes) = plan::load(&path, &value.store_plan(&root)?.digest()?)?;
    operation::retained_receipt(&path, &plan, &bytes)
}
