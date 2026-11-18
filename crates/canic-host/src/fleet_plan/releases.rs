//! Bind approved future Store outputs to the existing publication and child effect owners.
use super::{ChildId, FleetChild, FleetPlan, invalid, record, root_receipt, store_receipt};
use crate::{
    ApplyOutcome, Plan, PlanAction, Publication, Receipt, Result, StoredArtifact, UpgradeTarget,
    artifact, operation, plan, publication, store, supplier,
};
use std::path::Path;

impl FleetChild {
    pub(super) fn artifact_limit(&self) -> usize {
        if self.release.is_some() {
            artifact::MAX_ARTIFACT_BYTES
        } else {
            canic_management_protocol::MAX_WASM
        }
    }
}
impl FleetPlan {
    pub(super) fn source(
        &self,
        root: &Receipt,
        store: Option<&Receipt>,
        slot: ChildId,
    ) -> Result<Option<StoredArtifact>> {
        let Some(manifest) = &self.child(slot)?.release else {
            return Ok(None);
        };
        let receipt = store.ok_or_else(invalid)?;
        let expected = self.store_plan(root)?;
        if receipt.operation != expected.digest()?
            || receipt.module_hash != expected.artifact.sha256
            || receipt.controller != root.canister
        {
            return Err(invalid());
        }
        Ok(Some(StoredArtifact {
            store: UpgradeTarget {
                canister: receipt.canister.clone(),
                module: receipt.module_hash.clone(),
            },
            manifest: manifest.clone(),
            maintenance: None,
        }))
    }

    fn publication(&self, root: &Receipt, store: Option<&Receipt>, slot: ChildId) -> Result<Plan> {
        let child = self.child_plan(root, store, slot)?;
        let source = child
            .management()
            .and_then(|o| o.source.as_ref())
            .ok_or_else(invalid)?;
        publication_plan(&child, source)
    }
}

/// Publication is a separate authority even when its inputs share the fleet approval.
pub(super) fn publication_plan(child: &Plan, source: &StoredArtifact) -> Result<Plan> {
    let owner = child.management().ok_or_else(invalid)?;
    let value = Plan {
        version: 3,
        action: PlanAction::Publish(Publication {
            after_removal: None,
            root: UpgradeTarget {
                canister: owner.root.clone(),
                module: owner.root_module.clone(),
            },
            store: source.store.clone(),
            release: source.manifest.release.clone(),
        }),
        ..child.clone()
    };
    value.validate()?;
    Ok(value)
}

pub(super) fn validate(dir: &Path, fleet: &FleetPlan, slot: ChildId, bytes: &[u8]) -> Result<()> {
    let child = dir.join(record::child_leaf(slot));
    let path = child.join("publication");
    if !path.join("plan.json").try_exists().map_err(supplier)? {
        if fleet.child(slot)?.release.is_some()
            && child.join("plan.json").try_exists().map_err(supplier)?
        {
            return Err(invalid());
        }
        return Ok(());
    }
    let root = root_receipt(dir, fleet)?.ok_or_else(invalid)?;
    let value = fleet.publication(&root, store_receipt(dir, fleet)?.as_ref(), slot)?;
    let id = value.digest()?;
    if plan::load(&path, &id)?.0 != value {
        return Err(invalid());
    }
    if child.join("plan.json").try_exists().map_err(supplier)?
        && publication::receipt(&path, &value, &id, &publication::manifest(&value, bytes)?)?
            .is_none()
    {
        return Err(invalid());
    }
    Ok(())
}

pub(super) async fn apply(
    dir: &Path,
    fleet: &FleetPlan,
    root: &Receipt,
    store: Option<&Receipt>,
    identity: &str,
    icp: &Path,
) -> Result<()> {
    // Publish every selected release before the first child admission. Resume the same signed attempts.
    for (slot, child) in fleet.children().filter(|(_, c)| c.release.is_some()) {
        let value = fleet.publication(root, store, slot)?;
        let child_dir = dir.join(record::child_leaf(slot));
        let path = child_dir.join("publication");
        let bytes = store::read(&child_dir.join("module.wasm"), child.artifact_limit())?;
        if !path.join("plan.json").try_exists().map_err(supplier)? {
            let directory = crate::output_publication::prepare(&path, 0o700)?;
            let module = path.join("module.wasm");
            if module.try_exists().map_err(supplier)? {
                if store::read(&module, child.artifact_limit())? != bytes {
                    return Err(invalid());
                }
            } else {
                store::put_in(&directory, std::ffi::OsStr::new("module.wasm"), &bytes)?;
            }
            store::put_json_in(&directory, std::ffi::OsStr::new("plan.json"), &value)?;
        }
        validate(dir, fleet, slot, &bytes)?;
        let id = value.digest()?;
        if !matches!(
            operation::apply_at(dir.parent().ok_or_else(invalid)?, &path, &id, identity, icp)
                .await?,
            ApplyOutcome::Publication { .. }
        ) || publication::receipt(
            &path,
            &value,
            &id,
            child.release.as_ref().ok_or_else(invalid)?,
        )?
        .is_none()
        {
            return Err(invalid());
        }
    }
    Ok(())
}

pub(super) fn status(
    dir: &Path,
    fleet: &FleetPlan,
    slot: ChildId,
) -> Result<Option<crate::CanisterStatus>> {
    let path = dir.join(record::child_leaf(slot)).join("publication");
    if !path.join("plan.json").try_exists().map_err(supplier)? {
        return Ok(None);
    }
    let root = root_receipt(dir, fleet)?.ok_or_else(invalid)?;
    let value = fleet.publication(&root, store_receipt(dir, fleet)?.as_ref(), slot)?;
    operation::status_at(&path, &value.digest()?).map(Some)
}

pub(super) fn inventory(
    dir: &Path,
    id: &str,
    fleet: &FleetPlan,
) -> Result<Vec<crate::FleetArtifact>> {
    let mut result = Vec::new();
    for (slot, _) in fleet
        .children()
        .filter(|(_, child)| child.release.is_some())
    {
        let path = dir.join(record::child_leaf(slot)).join("publication");
        if !path.join("plan.json").try_exists().map_err(supplier)? {
            continue;
        }
        let root = root_receipt(dir, fleet)?.ok_or_else(invalid)?;
        let expected = fleet.publication(&root, store_receipt(dir, fleet)?.as_ref(), slot)?;
        let digest = expected.digest()?;
        let (plan, bytes) = plan::load(&path, &digest)?;
        result.push(crate::publication::list::project(
            &path, id, &digest, &plan, &bytes,
        )?);
    }
    Ok(result)
}

pub(super) async fn reconcile(
    dir: &Path,
    fleet: &FleetPlan,
    identity: &str,
    icp: &Path,
) -> Result<()> {
    for (slot, _) in fleet.children().filter(|(_, c)| c.release.is_some()) {
        let path = dir.join(record::child_leaf(slot)).join("publication");
        if path.join("plan.json").try_exists().map_err(supplier)? {
            let root = root_receipt(dir, fleet)?.ok_or_else(invalid)?;
            let value = fleet.publication(&root, store_receipt(dir, fleet)?.as_ref(), slot)?;
            operation::reconcile_at(
                dir.parent().ok_or_else(invalid)?,
                &path,
                &value.digest()?,
                identity,
                icp,
            )
            .await?;
        }
    }
    Ok(())
}

pub(super) fn quiesce(dir: &Path, fleet: &FleetPlan, selected: &Plan) -> Result<()> {
    for (slot, _) in fleet
        .children()
        .filter(|(_, child)| child.release.is_some())
    {
        let path = dir.join(record::child_leaf(slot)).join("publication");
        if !path.join("plan.json").try_exists().map_err(supplier)? {
            continue;
        }
        let root = root_receipt(dir, fleet)?.ok_or_else(invalid)?;
        let expected = fleet.publication(&root, store_receipt(dir, fleet)?.as_ref(), slot)?;
        let (value, bytes) = plan::load(&path, &expected.digest()?)?;
        publication::removal::quiesce_matching(&path, &value, &bytes, selected)?;
    }
    Ok(())
}
