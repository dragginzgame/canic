//! One-shot mainnet placement diagnostics for explicitly configured bindings.
use crate::{Error, Network, Result, SelectedFleet, supplier};
use ic_platform::principal::Principal;
use ic_topology::placement::collection::{self, PlacementCollectionReport};
use serde::Serialize;
use std::{collections::BTreeSet, future::Future};

/// Query-owned per-canister evidence, exposed for diagnostic presentation.
pub use ic_topology::placement::collection::PlacementOutcome;

/// Application declarations paired with independent routing observations.
#[derive(Debug, Serialize)]
pub struct FleetPlacements {
    /// Selected application name.
    pub app: String,
    /// Selected fleet name.
    pub fleet: String,
    /// Slots in canonical name order; unbound slots remain visible.
    pub slots: Vec<PlacementSlot>,
    /// Complete Query report, or None when no slot has a configured binding.
    pub observation: Option<PlacementCollectionReport>,
}

impl FleetPlacements {
    /// Whether at least one lookup was attempted and all selected lookups succeeded.
    #[must_use]
    pub fn collection_complete(&self) -> bool {
        self.observation
            .as_ref()
            .is_some_and(|report| report.collection_complete)
    }
}

/// A configured application slot; none of these declarations grants authority.
#[derive(Debug, Serialize)]
pub struct PlacementSlot {
    /// Application-local slot name.
    pub name: String,
    /// Declared application responsibility, not controller identity.
    pub role: Option<String>,
    /// Declared parent slot, not subnet or controller identity.
    pub parent: Option<String>,
    /// Configured mainnet identity; None is an unbound slot.
    pub canister_id: Option<String>,
    /// Index in the retained Query collection, absent for an unbound slot.
    pub observation_index: Option<usize>,
}

/// Observe only configured IDs for one selected mainnet fleet.
///
/// Reads no journal, signer or retained creation output. At most 32 unique bound
/// identities are queried sequentially within Query's shared 60-second budget.
/// All-unbound fleets return declarations without IO and are not complete reads.
/// Partial Query outcomes are retained; they never authorize deployment effects.
///
/// # Errors
/// Rejects non-mainnet, malformed/duplicate/excessive bindings before IO, and
/// retains Query collection-admission failures as supplier errors.
pub async fn observe(selected: &SelectedFleet) -> Result<FleetPlacements> {
    collect(selected, |ids| async move { collection::fetch(&ids).await }).await
}

async fn collect<F>(
    selected: &SelectedFleet,
    reader: impl FnOnce(Vec<Principal>) -> F,
) -> Result<FleetPlacements>
where
    F: Future<Output = std::result::Result<PlacementCollectionReport, collection::CollectionError>>,
{
    if selected.config.network != Network::Mainnet {
        return Err(Error::Invalid(
            "fleet placements supports configured mainnet fleets only".into(),
        ));
    }
    let mut ids = Vec::new();
    let mut seen = BTreeSet::new();
    let mut slots = Vec::new();
    for (name, slot) in &selected.config.canisters {
        let observation_index = if let Some(raw) = &slot.id {
            let id: Principal = raw
                .parse()
                .map_err(|_| Error::Invalid(format!("invalid canister ID for slot {name}")))?;
            if !seen.insert(id) {
                return Err(Error::Invalid(format!(
                    "duplicate canister binding at slot {name}"
                )));
            }
            if ids.len() == collection::MAX_CANISTERS {
                return Err(Error::Invalid(format!(
                    "fleet placements admits at most {} bound canisters",
                    collection::MAX_CANISTERS
                )));
            }
            let index = ids.len();
            ids.push(id);
            Some(index)
        } else {
            None
        };
        slots.push(PlacementSlot {
            name: name.clone(),
            role: slot.role.clone(),
            parent: slot.parent.clone(),
            canister_id: slot.id.clone(),
            observation_index,
        });
    }
    let observation = if ids.is_empty() {
        None
    } else {
        Some(reader(ids).await.map_err(supplier)?)
    };
    Ok(FleetPlacements {
        app: selected.app.clone(),
        fleet: selected.fleet.clone(),
        slots,
        observation,
    })
}

#[cfg(test)]
mod tests;
