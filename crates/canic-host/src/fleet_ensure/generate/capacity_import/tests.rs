//! Generator projections preserve estate identity and reject unsafe import destinations.

use super::*;
use crate::fleet_ensure::{
    generate::{RootSeed, tests::multi_component_source_toml},
    ops::capacity_import::prepare_review,
    policy::capacity_import::tests::{plan, principal},
};

pub(in crate::fleet_ensure) fn inputs(plan: &CapacityImportPlanRecord) -> (String, String) {
    let policy = multi_component_source_toml(
        &plan.authority.operator.to_text(),
        &principal(40).to_text(),
        &plan.authority.subnet.into_principal().to_text(),
    )
    .replacen(
        "[admission]",
        &format!(
            "recovery_controllers = [\"{}\"]\n\n[admission]",
            plan.authority.recovery_controllers[0]
        ),
        1,
    );
    let seed = EstateSeed {
        schema_version: 1,
        fleet_id: plan.authority.fleet.fleet.fleet_id,
        fresh_estate: false,
        coordinator: plan.authority.coordinator.to_text(),
        treasury: None,
        cycles_ledger: principal(41).to_text(),
        management_creation_fee_cycles: "500B".to_string(),
        roots: vec![RootSeed {
            placement_subnet: plan.authority.subnet.into_principal().to_text(),
            root: plan.authority.root.to_text(),
            store: principal(42).to_text(),
            pool_imports: vec![principal(43).to_text(), principal(44).to_text()],
        }],
    };
    (policy, toml::to_string_pretty(&seed).unwrap())
}

fn document(bytes: &[u8]) -> toml::Value {
    toml::from_str(std::str::from_utf8(bytes).unwrap()).unwrap()
}

#[test]
fn capacity_import_inventory_preserves_all_other_policy_and_identity_values() {
    let plan = plan();
    let (policy, seed) = inputs(&plan);
    let projection =
        prepare_capacity_import_inventory(&plan, policy.as_bytes(), seed.as_bytes()).unwrap();
    let mut expected_policy = document(policy.as_bytes());
    let mut expected_seed = document(seed.as_bytes());
    let imports = toml::Value::Array(
        [
            principal(43),
            principal(44),
            plan.sources[0].binding.canister_id,
        ]
        .map(|id| toml::Value::String(id.to_text()))
        .to_vec(),
    );
    expected_policy["fleet_subnet_roots"][0]["canister_pool"]
        .as_table_mut()
        .unwrap()
        .insert("imports".to_string(), imports.clone());
    expected_seed["roots"][0]["pool_imports"] = imports;
    assert_eq!(document(&projection.policy.replacement), expected_policy);
    assert_eq!(document(&projection.seed.replacement), expected_seed);
    let generated_policy =
        toml::from_str(std::str::from_utf8(&projection.policy.replacement).unwrap()).unwrap();
    let generated_seed =
        toml::from_str(std::str::from_utf8(&projection.seed.replacement).unwrap()).unwrap();
    validate_identity_seed(&generated_policy, &generated_seed).unwrap();
    assert_eq!(
        prepare_capacity_import_inventory(&plan, policy.as_bytes(), seed.as_bytes()).unwrap(),
        projection
    );
    assert_eq!(
        projection.policy.after_sha256,
        <[u8; 32]>::from(Sha256::digest(&projection.policy.replacement))
    );
    assert!(matches!(
        prepare_capacity_import_inventory(
            &plan,
            &projection.policy.replacement,
            &projection.seed.replacement
        ),
        Err(CapacityImportInventoryError::AlreadyPresent)
    ));
}

#[test]
fn capacity_import_inventory_binds_even_comment_only_input_changes() {
    let plan = plan();
    let (policy, seed) = inputs(&plan);
    let first =
        prepare_capacity_import_inventory(&plan, policy.as_bytes(), seed.as_bytes()).unwrap();
    let edited = format!("# reviewed locally\n{policy}");
    let second =
        prepare_capacity_import_inventory(&plan, edited.as_bytes(), seed.as_bytes()).unwrap();
    assert_ne!(first.policy.before_sha256, second.policy.before_sha256);
    assert_eq!(first.policy.replacement, second.policy.replacement);
    assert_eq!(first.seed, second.seed);
}

#[test]
fn capacity_import_inventory_rejects_other_estate_and_authority() {
    let plan = plan();
    let (policy, seed) = inputs(&plan);
    for changed in [
        seed.replace("fresh_estate = false", "fresh_estate = true"),
        seed.replace(
            &plan.authority.coordinator.to_text(),
            &principal(50).to_text(),
        ),
        seed.replace(&plan.authority.root.to_text(), &principal(50).to_text()),
        seed.replace(
            &plan.authority.fleet.fleet.fleet_id.to_string(),
            &"a5".repeat(32),
        ),
    ] {
        assert!(matches!(
            prepare_capacity_import_inventory(&plan, policy.as_bytes(), changed.as_bytes()),
            Err(CapacityImportInventoryError::AuthorityMismatch)
        ));
    }
    for changed in [
        policy.replace(&plan.authority.operator.to_text(), &principal(50).to_text()),
        policy.replace(
            &plan.authority.recovery_controllers[0].to_text(),
            &principal(50).to_text(),
        ),
        policy.replacen("schema_version = 1", "schema_version = 2", 1),
    ] {
        assert!(matches!(
            prepare_capacity_import_inventory(&plan, changed.as_bytes(), seed.as_bytes()),
            Err(CapacityImportInventoryError::AuthorityMismatch)
        ));
    }
}

#[test]
fn capacity_import_inventory_rejects_capacity_and_global_identity_conflicts() {
    let plan = plan();
    let (policy, seed) = inputs(&plan);
    let full = policy.replace("maximum_size = 3", "maximum_size = 2");
    assert!(matches!(
        prepare_capacity_import_inventory(&plan, full.as_bytes(), seed.as_bytes()),
        Err(CapacityImportInventoryError::Generation(error))
            if matches!(*error, FleetGenerateError::PoolImportCapacity(_))
    ));
    let mut candidates = plan.sources.clone();
    candidates[0].binding.canister_id = principal(42); // Existing Store.
    let conflict = prepare_review(plan.authority.clone(), candidates, plan.root_budget).unwrap();
    assert!(matches!(
        prepare_capacity_import_inventory(&conflict, policy.as_bytes(), seed.as_bytes()),
        Err(CapacityImportInventoryError::Generation(error))
            if matches!(*error, FleetGenerateError::SeedTopology(_))
    ));
    let different_pool = policy.replace(
        "maximum_size = 3",
        &format!("maximum_size = 3\nimports = [\"{}\"]", principal(51)),
    );
    assert!(matches!(
        prepare_capacity_import_inventory(&plan, different_pool.as_bytes(), seed.as_bytes()),
        Err(CapacityImportInventoryError::Generation(error))
            if matches!(*error, FleetGenerateError::SeedTopology(_))
    ));
}

#[test]
fn capacity_import_inventory_rejects_oversize_and_unknown_configuration() {
    let plan = plan();
    let (policy, seed) = inputs(&plan);
    assert!(matches!(
        prepare_capacity_import_inventory(
            &plan,
            &vec![b' '; MAX_GENERATOR_INPUT_BYTES + 1],
            seed.as_bytes()
        ),
        Err(CapacityImportInventoryError::TooLarge)
    ));
    let unknown = format!("unused_pool = []\n{policy}");
    assert!(matches!(
        prepare_capacity_import_inventory(&plan, unknown.as_bytes(), seed.as_bytes()),
        Err(CapacityImportInventoryError::Decode(_))
    ));
}

#[test]
fn initial_import_publishes_only_the_exact_declared_held_set() {
    let plan = plan();
    let (policy, seed) = inputs(&plan);
    let mut seed: EstateSeed = toml::from_str(&seed).unwrap();
    seed.roots[0].pool_imports = plan
        .sources
        .iter()
        .map(|source| source.binding.canister_id.to_text())
        .collect();
    let seed_bytes = toml::to_string_pretty(&seed).unwrap();
    let result =
        prepare_initial_import_inventory(&plan, policy.as_bytes(), seed_bytes.as_bytes()).unwrap();
    assert_eq!(
        document(&result.seed.replacement),
        document(seed_bytes.as_bytes())
    );
    assert_eq!(
        document(&result.policy.replacement)["fleet_subnet_roots"][0]["canister_pool"]["imports"],
        document(seed_bytes.as_bytes())["roots"][0]["pool_imports"]
    );
    seed.roots[0].pool_imports.push(principal(45).to_text());
    assert!(matches!(
        prepare_initial_import_inventory(
            &plan,
            policy.as_bytes(),
            toml::to_string(&seed).unwrap().as_bytes()
        ),
        Err(CapacityImportInventoryError::AuthorityMismatch)
    ));
}
