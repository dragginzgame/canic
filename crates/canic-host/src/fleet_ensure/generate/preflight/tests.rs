use super::*;
use crate::fleet_ensure::{
    generate::{
        FreshEstateSeedRequest, initialize_fresh_estate_seed, tests::multi_component_source_toml,
    },
    workflow::readiness::{
        FleetReadinessError, FleetReadinessRequest, ReadinessGenerationInputs, inspect,
    },
};
use std::fs;

#[test]
fn completed_fresh_seed_rejects_before_build_or_network_and_explicit_inventory_passes() {
    let root = crate::test_support::temp_dir("generation-preflight");
    fs::create_dir_all(&root).unwrap();
    let source = root.join("policy.toml");
    let seed = root.join("seed.toml");
    let operator = Principal::from_slice(&[20]);
    let subnet = Principal::from_slice(&[21]).to_text();
    let ledger = Principal::from_text(super::super::mainnet_cycles_ledger()).unwrap();
    fs::write(
        &source,
        multi_component_source_toml(&operator.to_text(), &subnet, &subnet),
    )
    .unwrap();
    initialize_fresh_estate_seed(&FreshEstateSeedRequest {
        cycles_ledger: &ledger.to_text(),
        management_creation_fee_cycles: 500_000_000_000,
        source: &source,
        seed: &seed,
    })
    .unwrap();
    let request = FleetGenerationInputsRequest {
        root: &root,
        environment: "local",
        fleet: "fleet",
        source: &source,
        seed: &seed,
    };
    validate_generation_inputs(&request, operator, ledger).unwrap();
    assert!(!load(&request, None).unwrap().clean_reinstall);
    let paths = EnsurePaths::under(&root, "local", "fleet");
    write_completion(&paths);
    let original_plan = fs::read(&paths.plan).unwrap();
    let original_journal = fs::read(&paths.journal).unwrap();
    let original_seed = fs::read(&seed).unwrap();
    assert!(matches!(
        validate_generation_inputs(&request, operator, ledger),
        Err(FleetGenerateError::CompletedFleetRequiresExplicitInventory)
    ));
    let readiness = FleetReadinessRequest {
        workspace: &root,
        environment: "local",
        fleet: "fleet",
        icp_executable: "must-not-run",
        signing_identity: None,
        operator,
        cycles_ledger: ledger,
        estimated_required_cycles: None,
        desired: None,
        conversion: None,
        generation_inputs: Some(ReadinessGenerationInputs {
            source: &source,
            seed: &seed,
        }),
    };
    assert!(
        matches!(inspect(&readiness), Err(FleetReadinessError::GenerationInputs(error))
        if matches!(*error, FleetGenerateError::CompletedFleetRequiresExplicitInventory))
    );
    assert_eq!(fs::read(&seed).unwrap(), original_seed);
    assert_eq!(fs::read(&paths.plan).unwrap(), original_plan);
    assert_eq!(fs::read(&paths.journal).unwrap(), original_journal);
    assert!(!paths.lock.exists());

    let mut inventory: EstateSeed = load_toml(&seed, "seed").unwrap();
    inventory.fresh_estate = false;
    inventory.coordinator = Principal::from_slice(&[30]).to_text();
    inventory.roots[0].root = Principal::from_slice(&[31]).to_text();
    inventory.roots[0].store = Principal::from_slice(&[32]).to_text();
    inventory.roots[0].pool_imports = vec![Principal::from_slice(&[33]).to_text()];
    fs::write(&seed, toml::to_string(&inventory).unwrap()).unwrap();
    validate_generation_inputs(&request, operator, ledger).unwrap();
    let current = load(&request, None).unwrap();
    assert_reset_funding_projection(&request, &inventory, operator, ledger);
    assert!(current.clean_reinstall);
    assert_eq!(
        current.initialization,
        Some(BootstrapCoordinatorSelection::Initialize)
    );
    assert!(matches!(
        validate_generation_inputs(&request, Principal::from_slice(&[40]), ledger),
        Err(FleetGenerateError::Authority(_))
    ));
    assert!(matches!(
        validate_generation_inputs(&request, operator, Principal::from_slice(&[40])),
        Err(FleetGenerateError::Authority(_))
    ));

    inventory.roots[0].pool_imports = vec![inventory.coordinator.clone()];
    fs::write(&seed, toml::to_string(&inventory).unwrap()).unwrap();
    assert!(matches!(
        validate_generation_inputs(&request, operator, ledger),
        Err(FleetGenerateError::SeedTopology(_))
    ));
    assert_eq!(fs::read(&paths.plan).unwrap(), original_plan);
    assert_eq!(fs::read(&paths.journal).unwrap(), original_journal);
    fs::remove_dir_all(root).unwrap();
}

fn write_completion(paths: &EnsurePaths) {
    fs::create_dir_all(paths.plan.parent().unwrap()).unwrap();
    let operation = "ab".repeat(32);
    let digest = "cd".repeat(32);
    fs::write(
        &paths.plan,
        serde_json::to_vec(&serde_json::json!({
            "environment": "local", "fleet": "fleet", "operation_id": operation,
            "plan_sha256": digest, "scope": "full",
        }))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        &paths.journal,
        serde_json::to_vec(&serde_json::json!({
            "fleet": "fleet", "operation_id": operation, "plan_sha256": digest,
            "completion": "converged", "effects": [{"state": "applied"}],
        }))
        .unwrap(),
    )
    .unwrap();
}

fn assert_reset_funding_projection(
    request: &FleetGenerationInputsRequest<'_>,
    inventory: &EstateSeed,
    operator: Principal,
    ledger: Principal,
) {
    let targets = bootstrap_funding_targets(request, operator, ledger).unwrap();
    assert!(matches!(
        bootstrap_funding_targets(request, Principal::anonymous(), ledger),
        Err(FleetGenerateError::Authority(_))
    ));
    assert!(matches!(
        bootstrap_funding_targets(request, operator, Principal::anonymous()),
        Err(FleetGenerateError::Authority(_))
    ));
    assert_eq!(
        targets
            .iter()
            .map(|target| target.principal.as_str())
            .collect::<Vec<_>>(),
        [
            &inventory.coordinator,
            &inventory.roots[0].root,
            &inventory.roots[0].store
        ]
    );
    assert!(targets.iter().all(|target| target.observation_burn_cycles
        == super::super::GENERATED_RETAINED_MAXIMUM_OBSERVATION_BURN_CYCLES));
    assert_eq!(targets[2].minimum_cycles, 0);
}
