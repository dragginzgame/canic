//! Bind fresh configuration to a fully enumerated prepared source estate.
//!
//! Historical records supply physical evidence only; the ordinary compiler owns current authority.

use crate::{
    fleet_ensure::{
        generate::{
            EstateSeed, FleetGenerateError, FleetGenerateRequest, FleetSource, ObservedCanister,
        },
        model::DesiredCanisterKind,
        ops::{self, EnsurePaths, completed_preparation as preparation, retained_contract},
    },
    icp::{IcpCli, LocalReplicaTarget},
};
use canic_core::ids::AppId;
use std::collections::{BTreeMap, BTreeSet};

/// Prepared native observations remain distinguishable from ordinary live generation.
pub(super) struct PreparedEstate {
    pub balances: BTreeMap<String, ObservedCanister>,
    pub review_sha256: String,
}

pub(super) fn observe(
    request: &FleetGenerateRequest<'_>,
    source: &FleetSource,
    seed: &EstateSeed,
    app: &AppId,
    local_replica: Option<&LocalReplicaTarget>,
) -> Result<Option<PreparedEstate>, FleetGenerateError> {
    let paths = EnsurePaths::under(request.root, request.environment, request.fleet);
    if ops::completed_handoff::committed(&paths)
        .map_err(|error| FleetGenerateError::Authority(error.to_string()))?
        .is_some()
    {
        preparation::require_no_intent(&paths)
            .map_err(|error| FleetGenerateError::Authority(error.to_string()))?;
        return Ok(None);
    }
    let Some(review) = preparation::review(&paths).map_err(Box::new)? else {
        preparation::require_no_intent(&paths)
            .map_err(|error| FleetGenerateError::Authority(error.to_string()))?;
        return Ok(None);
    };
    let _lock = ops::lock_completed_preparation(&paths)
        .map_err(|error| FleetGenerateError::Authority(error.to_string()))?;
    let journal = preparation::journal(&paths, &review).map_err(Box::new)?
        .filter(|journal| journal.prepared)
        .ok_or_else(|| FleetGenerateError::Authority(format!(
            "complete the reviewed source preparation before generation: fleet ensure {} --apply {}",
            request.fleet, review.review_sha256,
        )))?;
    let inspected = retained_contract::inspect_completed_source(
        request.root,
        request.environment,
        request.fleet,
    )
    .map_err(|error| FleetGenerateError::Authority(error.to_string()))?;
    let inventory = &inspected.inventory;
    if seed.fresh_estate
        || seed.fleet_id != inventory.fleet.fleet.fleet_id
        || app != &inventory.fleet.app
        || source.operator != inventory.receipts.source_operator
        || seed.cycles_ledger != inventory.receipts.cycles_ledger
        || seed.coordinator != inventory.coordinator.to_text()
    {
        return Err(FleetGenerateError::SeedTopology(
            "completed-source generation requires the exact retained Fleet, operator, Ledger and Coordinator identities".into(),
        ));
    }
    validate_seed(seed, source, inventory)?;
    let icp = IcpCli::new(request.icp_executable, Some(request.environment.into()))
        .with_identity(request.signing_identity)
        .with_cwd(request.root)
        .with_local_replica(local_replica.cloned());
    let observed = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|error| FleetGenerateError::Authority(error.to_string()))?
        .block_on(retained_contract::inspect_completed_membership(
            request.root,
            request.environment,
            request.fleet,
            &icp,
        ))
        .map_err(|error| FleetGenerateError::Authority(error.to_string()))?;
    preparation::revalidate(&paths, &review, &inspected, &observed).map_err(Box::new)?;
    for action in &review.actions {
        if !preparation::sealed(&icp, &paths, &review, action).map_err(Box::new)? {
            return Err(FleetGenerateError::Authority(
                "completed-source authority seal reopened".into(),
            ));
        }
    }
    // This is the reviewed management sample, not a new conservation baseline or live usage.
    let balances = review
        .custody
        .canisters
        .iter()
        .map(|(name, custody)| {
            let balance = journal.inspections[name].balance.ok_or_else(|| {
                FleetGenerateError::Authority("incomplete prepared balance inventory".into())
            })?;
            Ok((
                custody.binding.principal.to_text(),
                ObservedCanister {
                    cycles: balance.native_cycles,
                    module_sha256: custody.binding.module_sha256.clone(),
                    subnet: custody.binding.subnet.to_string(),
                    pool_status: None,
                },
            ))
        })
        .collect::<Result<BTreeMap<_, _>, FleetGenerateError>>()?;
    Ok(Some(PreparedEstate {
        balances,
        review_sha256: review.review_sha256,
    }))
}

fn validate_seed(
    seed: &EstateSeed,
    source: &FleetSource,
    inventory: &crate::fleet_ensure::CompletedEstateInventoryView,
) -> Result<(), FleetGenerateError> {
    let conflict = || {
        FleetGenerateError::SeedTopology(
        "completed-source seed must name every retained ID exactly once under its observed Root and subnet".into(),
    )
    };
    let coordinator = inventory
        .canisters
        .values()
        .find(|c| c.kind == DesiredCanisterKind::Coordinator)
        .ok_or_else(conflict)?;
    if coordinator.subnet.to_string() != source.coordinator.subnet.subnet {
        return Err(conflict());
    }
    let mut selected = BTreeSet::from([seed.coordinator.clone()]);
    for root in &seed.roots {
        let (name, physical_root) = inventory
            .canisters
            .iter()
            .find(|(_, c)| c.principal.to_text() == root.root)
            .ok_or_else(conflict)?;
        if physical_root.kind != DesiredCanisterKind::Root
            || physical_root.subnet.to_string() != root.placement_subnet
            || !selected.insert(root.root.clone())
        {
            return Err(conflict());
        }
        for (id, store) in std::iter::once((&root.store, true))
            .chain(root.pool_imports.iter().map(|id| (id, false)))
        {
            let canister = inventory
                .canisters
                .values()
                .find(|c| c.principal.to_text() == *id)
                .ok_or_else(conflict)?;
            let role = if store {
                canister.kind == DesiredCanisterKind::Store
            } else {
                matches!(
                    canister.kind,
                    DesiredCanisterKind::Pool | DesiredCanisterKind::Component
                )
            };
            if !role
                || canister.root.as_ref() != Some(name)
                || canister.subnet != physical_root.subnet
                || !selected.insert(id.clone())
            {
                return Err(conflict());
            }
        }
    }
    let expected = inventory
        .canisters
        .values()
        .map(|c| c.principal.to_text())
        .collect::<BTreeSet<_>>();
    if selected != expected {
        return Err(FleetGenerateError::SeedTopology(format!(
            "completed-source seed omitted retained IDs: {}; add them to their Root pool_imports before generation",
            expected
                .difference(&selected)
                .cloned()
                .collect::<Vec<_>>()
                .join(", "),
        )));
    }
    Ok(())
}
