//! Read current generation inputs before artifact compilation or remote observations.

#[cfg(test)]
mod tests;

use crate::fleet_ensure::{
    generate::{
        EstateSeed, FleetGenerateError, FleetSource, infrastructure_bootstrap, load_toml,
        parse_principal, require_schema, validate_identity_seed,
    },
    model::infrastructure_bootstrap::{
        BootstrapCoordinatorSelection, InfrastructureBootstrapFundingTarget,
    },
    ops::{EnsurePaths, operation_selection, retained_contract},
};
use candid::Principal;
use std::path::Path;

/// Current source and seed selection; no build or historical executable input is needed.
pub struct FleetGenerationInputsRequest<'a> {
    pub root: &'a Path,
    pub environment: &'a str,
    pub fleet: &'a str,
    pub source: &'a Path,
    pub seed: &'a Path,
}

/// Parsed inputs shared by pre-build readiness and artifact-bound generation.
pub(super) struct GenerationInputs {
    pub source: FleetSource,
    pub seed: EstateSeed,
    pub initialization: Option<BootstrapCoordinatorSelection>,
    pub clean_reinstall: bool,
}

/// Check the intended generator inputs and their signer/Ledger binding without effects.
pub fn validate_generation_inputs(
    request: &FleetGenerationInputsRequest<'_>,
    operator: Principal,
    cycles_ledger: Principal,
) -> Result<(), FleetGenerateError> {
    let inputs = load(request, None)?;
    validate_input_authority(&inputs, operator, cycles_ledger)
}

fn validate_input_authority(
    inputs: &GenerationInputs,
    operator: Principal,
    cycles_ledger: Principal,
) -> Result<(), FleetGenerateError> {
    if parse_principal("operator", &inputs.source.operator)? != operator
        || parse_principal("cycles ledger", &inputs.seed.cycles_ledger)? != cycles_ledger
    {
        return Err(FleetGenerateError::Authority(
            "generation policy/seed operator or Cycles Ledger differs from readiness selection"
                .into(),
        ));
    }
    Ok(())
}

/// Project current reset owners without loading artifacts or historical executable records.
pub(in crate::fleet_ensure) fn bootstrap_funding_targets(
    request: &FleetGenerationInputsRequest<'_>,
    operator: Principal,
    cycles_ledger: Principal,
) -> Result<Vec<InfrastructureBootstrapFundingTarget>, FleetGenerateError> {
    let inputs = load(request, None)?;
    validate_input_authority(&inputs, operator, cycles_ledger)?;
    if !inputs.clean_reinstall {
        return Ok(Vec::new());
    }
    let controllers =
        super::sorted_controllers(&inputs.source.operator, &inputs.source.recovery_controllers);
    let target = |name: String,
                  principal: String,
                  controllers: Vec<String>,
                  minimum_cycles: u128| InfrastructureBootstrapFundingTarget {
        name,
        principal,
        controllers,
        minimum_cycles,
        observation_burn_cycles: super::GENERATED_RETAINED_MAXIMUM_OBSERVATION_BURN_CYCLES,
        update_burn_cycles: super::GENERATED_RETAINED_MAXIMUM_UPDATE_BURN_CYCLES,
    };
    let mut targets = vec![target(
        "coordinator".into(),
        inputs.seed.coordinator.clone(),
        controllers.clone(),
        inputs
            .source
            .coordinator
            .root_funding
            .minimum_reserve_cycles
            .to_u128(),
    )];
    let mut roots = inputs.source.fleet_subnet_roots.iter().collect::<Vec<_>>();
    roots.sort_by_key(|source| Principal::from_text(&source.placement_subnet).ok());
    for (index, source) in roots.into_iter().enumerate() {
        let seed = inputs
            .seed
            .roots
            .iter()
            .find(|seed| seed.placement_subnet == source.placement_subnet)
            .ok_or_else(|| {
                FleetGenerateError::SeedTopology("missing reset Root inventory".into())
            })?;
        targets.push(target(
            format!("root-{index}"),
            seed.root.clone(),
            controllers.clone(),
            source.root_funding.request_threshold.to_u128(),
        ));
        let mut store_controllers = controllers.clone();
        store_controllers.push(seed.root.clone());
        store_controllers.sort();
        store_controllers.dedup();
        targets.push(target(
            format!("store-{index}"),
            seed.store.clone(),
            store_controllers,
            0,
        ));
    }
    Ok(targets)
}

pub(super) fn load(
    request: &FleetGenerationInputsRequest<'_>,
    initialization: Option<BootstrapCoordinatorSelection>,
) -> Result<GenerationInputs, FleetGenerateError> {
    crate::fleet_ensure::policy::validate_path_labels(request.environment, request.fleet)?;
    let clean_reinstall = initialization.is_none()
        && operation_selection::completed_fleet(
            &EnsurePaths::under(request.root, request.environment, request.fleet),
            request.environment,
            request.fleet,
        )
        .map_err(|error| FleetGenerateError::Authority(error.to_string()))?;
    if clean_reinstall {
        retained_contract::check(request.root, request.environment, request.fleet)
            .map_err(|error| FleetGenerateError::Authority(error.to_string()))?;
    }
    let initialization = initialization
        .or_else(|| clean_reinstall.then_some(BootstrapCoordinatorSelection::Initialize));
    let source: FleetSource = load_toml(request.source, "source")?;
    let seed: EstateSeed = load_toml(request.seed, "seed")?;
    require_schema(source.schema_version, "source")?;
    require_schema(seed.schema_version, "seed")?;
    if clean_reinstall && seed.fresh_estate {
        return Err(FleetGenerateError::CompletedFleetRequiresExplicitInventory);
    }
    if let Some(selection) = initialization {
        infrastructure_bootstrap::validate_seed(&source, &seed, selection)?;
    } else {
        validate_identity_seed(&source, &seed)?;
    }
    Ok(GenerationInputs {
        source,
        seed,
        initialization,
        clean_reinstall,
    })
}
