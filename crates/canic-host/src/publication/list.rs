//! Projection of existing publication custody, never a second artifact catalog.
use super::{Publication, manifest, receipt};
use crate::{
    Artifact, DeploymentName, Network, Plan, Result, SelectedFleet, fleet, fleet_plan, plan, store,
};
use serde::Serialize;
use std::path::Path;

/// One retained publication approval and its independently validated completion.
#[derive(Debug, Serialize)]
pub struct FleetArtifact {
    /// Status/apply address, including the outer operation for nested fleet work.
    pub operation: String,
    /// Exact publication approval digest; may be nested under `operation`.
    pub publication_operation: String,
    /// Original app, fleet and application slot.
    pub deployment: DeploymentName,
    /// Original Root/Store code bindings and immutable release label.
    pub publication: Publication,
    /// Exact retained raw Wasm identity and size.
    pub artifact: Artifact,
    /// A validated signed publication observation is retained, not proof of live availability.
    pub publication_retained: bool,
}

/// Bounded, local publication evidence for one configured network-bound fleet.
#[derive(Debug, Serialize)]
pub struct FleetArtifacts {
    /// Selected application.
    pub app: String,
    /// Selected fleet.
    pub fleet: String,
    /// Exact configured network/trust binding; equal local principals do not merge networks.
    pub network: Network,
    /// Coverage and freshness limitation, also retained in JSON output.
    pub scope: &'static str,
    /// Lexically ordered release observations; ordering never selects a latest version.
    pub publications: Vec<FleetArtifact>,
    /// Unreadable or invalid journal entries; valid entries remain visible.
    pub issues: Vec<String>,
}

pub(crate) fn selected(plan: &Plan, selection: &SelectedFleet) -> bool {
    plan.network == selection.config.network
        && plan
            .deployment
            .as_ref()
            .is_some_and(|name| name.app == selection.app && name.fleet == selection.fleet)
}

pub(crate) fn project(
    dir: &Path,
    operation: &str,
    digest: &str,
    plan: &Plan,
    bytes: &[u8],
) -> Result<FleetArtifact> {
    let publication = super::binding(plan)?.clone();
    let publication_retained = receipt(dir, plan, digest, &manifest(plan, bytes)?)?.is_some();
    Ok(FleetArtifact {
        operation: operation.into(),
        publication_operation: digest.into(),
        deployment: plan
            .deployment
            .clone()
            .expect("validated publication deployment"),
        publication,
        artifact: plan.artifact.clone(),
        publication_retained,
    })
}

/// List selected-fleet publications using existing plan and signed-receipt admission.
///
/// Includes nested bootstrap publications once materialized, plus independent
/// publication plans. No network calls, signer invocation or journal mutation.
/// The existing journal lock is held while reading each operation. Missing
/// journals remain absent. Unconfirmed publication may have executed remotely.
/// # Errors
/// Refuses invalid selection, unreadable inventory or its existing operation bound.
pub fn fleet_artifacts(root: &Path, selection: &SelectedFleet) -> Result<FleetArtifacts> {
    selection.validate()?;
    let (ids, mut issues) = fleet::operation_ids(root, true)?;
    let mut publications = Vec::new();
    for id in ids {
        let result = (|| {
            let journal = store::Store::open(root)?;
            let dir = journal.operation(&id)?;
            if fleet_plan::exists(&dir)? {
                return fleet_plan::artifact_inventory(&dir, &id, selection);
            }
            let (plan, bytes) = plan::load(&dir, &id)?;
            if !selected(&plan, selection) || plan.publication().is_none() {
                return Ok(Vec::new());
            }
            Ok(vec![project(&dir, &id, &id, &plan, &bytes)?])
        })();
        match result {
            Ok(mut entries) => publications.append(&mut entries),
            Err(error) => issues.push(format!("{id}: {error}")),
        }
    }
    publications.sort_by(|a, b| {
        (
            &a.publication.store.canister,
            &a.publication.release.name,
            &a.publication.release.version,
            &a.publication_operation,
        )
            .cmp(&(
                &b.publication.store.canister,
                &b.publication.release.name,
                &b.publication.release.version,
                &b.publication_operation,
            ))
    });
    Ok(FleetArtifacts {
        app: selection.app.clone(),
        fleet: selection.fleet.clone(),
        network: selection.config.network.clone(),
        scope: "local publication evidence; not the live Store catalog or deployment status",
        publications,
        issues,
    })
}
