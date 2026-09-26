//! Completed receipt audit checks historical bytes without external mutation.

use super::*;
use crate::fleet_ensure::ops::retained_contract::{RetainedContractError, check};
use std::{fs, path::PathBuf};

#[test]
#[ignore = "requires explicitly supplied read-only completed source records"]
fn inspect_supplied_completed_receipts_without_external_mutation() {
    let root = std::env::var_os("CANIC_COMPLETED_SOURCE_WORKSPACE").unwrap();
    let environment = std::env::var("CANIC_COMPLETED_SOURCE_ENVIRONMENT").unwrap();
    let fleet = std::env::var("CANIC_COMPLETED_SOURCE_FLEET").unwrap();
    let paths = EnsurePaths::under(std::path::Path::new(&root), &environment, &fleet);
    let audit = inspect(&paths, &environment, &fleet).unwrap();
    assert!(audit.effect_count > 0);
    assert!(audit.phase_count > 0);
}

fn historical_fixture() -> (PathBuf, EnsurePaths) {
    let (fixture, paths, _) = crate::fleet_ensure::tests::terminal_retirement_fixture();
    let mut plan = read_json(&paths.plan);
    remove_recovery_declarations(&mut plan);
    assert_eq!(
        plan.as_object_mut()
            .unwrap()
            .remove("infrastructure_bootstrap"),
        Some(Value::Null)
    );
    plan["recovery_review"] = Value::Null;
    plan["reviewed_desired"]["desired"]["bootstrap"] = serde_json::json!({
        "admission_identity_origin": null,
        "admission": {"schema_version": 1, "fleet_principals": [], "rules": [], "template_digest": vec![0; 32]},
        "app": "receipt_test", "canonical_network_id": "11".repeat(32),
        "component_deployment_configuration": {
            "component_topology": {"component_specs": [], "provisioning_grants": []},
            "component_group_topology": {"component_groups": []},
            "deployment_topology": {"component_group_deployments": []},
            "fleet_service_topology": {"targets": []},
        },
        "coordinator": fixture.desired.canisters[0].principal,
        "coordinator_subnet": fixture.desired.canisters[0].subnet,
        "fleet_id": "22".repeat(32), "fresh_estate": false,
        "release_build_id": "33".repeat(32), "root_funding": null, "roots": [],
    });
    let mut plan: CompletedPhaseEvidence = decode(plan).unwrap();
    plan.plan_sha256 = phase_hash(&plan).unwrap();
    let mut journal = read_json(&paths.journal);
    journal["plan_sha256"] = plan.plan_sha256.clone().into();
    let mut actions = initial_actions(&plan.canisters).unwrap();
    for reference in journal["successor_phases"].as_array_mut().unwrap() {
        let old = reference["plan_sha256"].as_str().unwrap();
        let mut phase = read_json(
            &paths
                .plan
                .with_file_name("phases")
                .join(format!("{old}.json")),
        );
        remove_recovery_declarations(&mut phase);
        assert_eq!(
            phase
                .as_object_mut()
                .unwrap()
                .remove("infrastructure_bootstrap"),
            Some(Value::Null)
        );
        phase["reviewed_desired"] = json::to_value(&plan.reviewed_desired).unwrap();
        let mut phase: CompletedPhaseEvidence = decode(phase).unwrap();
        phase.plan_sha256 = phase_hash(&phase).unwrap();
        reference["plan_sha256"] = phase.plan_sha256.clone().into();
        write_json(
            &paths
                .plan
                .with_file_name("phases")
                .join(format!("{}.json", phase.plan_sha256)),
            &json::to_value(&phase).unwrap(),
        );
        actions.extend(phase.protocol_actions);
    }
    for (effect, action) in journal["effects"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .zip(&actions)
    {
        effect["action_sha256"] = sha256_hex(&json::to_vec(action).unwrap()).into();
    }
    let mut state = read_json(&paths.state);
    remove_recovery_declarations(&mut state);
    write_json(&paths.state, &state);
    write_json(&paths.plan, &json::to_value(&plan).unwrap());
    write_json(&paths.journal, &journal);
    (fixture.root, paths)
}

fn read_json(path: &std::path::Path) -> Value {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}
fn write_json(path: &std::path::Path, value: &Value) {
    fs::write(path, serde_json::to_vec_pretty(value).unwrap()).unwrap();
}
fn remove_recovery_declarations(value: &mut Value) {
    match value {
        Value::Object(fields) => {
            fields.remove("recovery_controllers");
            fields.values_mut().for_each(remove_recovery_declarations);
        }
        Value::Array(values) => {
            values.iter_mut().for_each(remove_recovery_declarations);
        }
        _ => {}
    }
}

#[test]
fn historical_receipt_audit_does_not_supply_missing_estate_authority() {
    let (root, paths) = historical_fixture();
    let before: Vec<_> = [&paths.plan, &paths.journal, &paths.state]
        .into_iter()
        .map(|path| fs::read(path).unwrap())
        .collect();
    let audit = inspect(&paths, "local", "test-fleet").unwrap();
    assert_eq!(audit.recorded_funding_cycles, 110);
    assert_eq!(audit.recorded_operator_debit_cycles, 120);
    assert_eq!(audit.original_maximum_execution_burn_cycles, 200);
    // This receipt-only fixture has no complete Root inventory. A valid receipt
    // audit cannot manufacture that separate authority or pass the full preflight.
    assert!(matches!(
        check(&root, "local", "test-fleet"),
        Err(RetainedContractError::ReceiptAudit(_))
    ));
    assert_eq!(fs::read(&paths.plan).unwrap(), before[0]);
    assert_eq!(fs::read(&paths.journal).unwrap(), before[1]);
    assert_eq!(fs::read(&paths.state).unwrap(), before[2]);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn original_root_balances_bind_the_declared_ledger_and_exact_owner() {
    use crate::fleet_ensure::model::EstateFundingDomainPlan;

    let (root, paths) = historical_fixture();
    let plan: CompletedPhaseEvidence = decode(read_json(&paths.plan)).unwrap();
    let mut desired = plan.reviewed_desired.unwrap().desired;
    desired.canisters[0].kind = DesiredCanisterKind::Root;
    let declaration = &desired.canisters[0];
    let domain = EstateFundingDomainPlan {
        allocated_workloads: 0,
        available_cycles: Some(30),
        available_pool_slots: 0,
        creation_amount_cycles: 50,
        cycles_ledger: desired.cycles_ledger.clone(),
        creation_execution_margin_cycles: 0,
        readiness_floor_cycles: 20,
        eligible_ready_pool_assets: 0,
        initial_pool_assets: Vec::new(),
        ledger_fee_cycles: 10,
        management_creation_fee_cycles: 50,
        maximum_creation_debit_cycles: 0,
        maximum_creation_fee_cycles: 0,
        maximum_funding_cycles: 0,
        occupied_pool_assets: 0,
        pending_creation_count: 0,
        pending_creation: None,
        pool_maximum_size: 1,
        planned_initial_workloads: 0,
        required_creation_count: 0,
        root: declaration.name.clone(),
        root_principal: declaration.principal.clone(),
        shortfall_cycles: 0,
    };
    let journal = serde_json::json!({
        "initial_estate_funding_cycles_by_root": { &domain.root: "30" },
    });
    let mut bounds = plan.conservation;
    bounds.estate_funding_domains = vec![domain.clone()];
    assert_eq!(
        initial_root_accounts(&journal, &bounds, &desired).unwrap(),
        BTreeMap::from([(domain.root, 30)])
    );
    for mutation in ["ledger", "owner", "absent-owner", "kind", "duplicate"] {
        let mut changed_bounds = bounds.clone();
        let mut changed_desired = desired.clone();
        let domain = &mut changed_bounds.estate_funding_domains[0];
        match mutation {
            "ledger" => domain.cycles_ledger = changed_desired.operator.clone(),
            "owner" => domain.root_principal = Some(changed_desired.operator.clone()),
            "absent-owner" => domain.root_principal = None,
            "kind" => changed_desired.canisters[0].kind = DesiredCanisterKind::Coordinator,
            "duplicate" => changed_desired
                .canisters
                .push(changed_desired.canisters[0].clone()),
            _ => unreachable!(),
        }
        assert!(
            matches!(
                initial_root_accounts(&journal, &changed_bounds, &changed_desired),
                Err(EnsureStateError::InvalidTerminalSource)
            ),
            "{mutation}"
        );
    }
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn incomplete_changed_and_overbudget_receipts_reject() {
    for (pointer, value) in [
        ("/completion", Value::from("in_progress")),
        ("/effects/0/state", Value::from("issued")),
        ("/effects/0/action_sha256", Value::from("00".repeat(32))),
        ("/effects/0/maintenance_attempts", Value::from(1)),
        ("/effects/0/publication_attempts", Value::from(1)),
        ("/effects/0/receipt", Value::Null),
        ("/initial_controlled_cycles", Value::from("invalid")),
        (
            "/initial_controlled_cycles",
            Value::from(u128::MAX.to_string()),
        ),
        ("/initial_operator_cycles", Value::from("119")),
        ("/effects/0/post_cycles", Value::from("499")),
        (
            "/successor_phases/0/execution_burn_before_phase",
            Value::from("201"),
        ),
    ] {
        let (root, paths) = historical_fixture();
        let mut journal = read_json(&paths.journal);
        *journal.pointer_mut(pointer).unwrap() = value;
        write_json(&paths.journal, &journal);
        assert!(matches!(
            inspect(&paths, "local", "test-fleet"),
            Err(EnsureStateError::InvalidTerminalSource)
        ));
        fs::remove_dir_all(root).unwrap();
    }
}

#[test]
fn extra_receipts_unknown_paid_fields_and_invented_controllers_reject() {
    for mutation in [
        "extra-effect",
        "unknown-paid-field",
        "controller-declaration",
        "budget",
    ] {
        let (root, paths) = historical_fixture();
        let mut journal = read_json(&paths.journal);
        let mut plan = read_json(&paths.plan);
        match mutation {
            "extra-effect" => {
                let duplicate = journal["effects"][0].clone();
                journal["effects"].as_array_mut().unwrap().push(duplicate);
            }
            "unknown-paid-field" => {
                journal["unaccounted_payment"] = serde_json::json!({"amount":"1"});
            }
            "controller-declaration" => {
                plan["reviewed_desired"]["desired"]["bootstrap"]["recovery_controllers"] =
                    serde_json::json!([]);
            }
            "budget" => {
                plan["conservation"]["maximum_execution_burn_cycles"] = "999".into();
            }
            _ => unreachable!(),
        }
        write_json(&paths.plan, &plan);
        write_json(&paths.journal, &journal);
        assert!(matches!(
            inspect(&paths, "local", "test-fleet"),
            Err(EnsureStateError::InvalidTerminalSource)
        ));
        fs::remove_dir_all(root).unwrap();
    }
}

#[test]
fn frozen_source_store_adoption_hash_matches_its_independent_receipt() {
    let fixture: Value = serde_json::from_str(include_str!("store-adoption.json")).unwrap();
    let action: CompletedActionEvidence = decode(fixture["action"].clone()).unwrap();
    assert_eq!(
        sha256_hex(&json::to_vec(&action).unwrap()),
        fixture["action_sha256"].as_str().unwrap()
    );
}

#[test]
fn changed_successor_payload_cannot_reuse_its_completed_phase_digest() {
    let (root, paths) = historical_fixture();
    let journal = read_json(&paths.journal);
    let label = journal["successor_phases"][0]["plan_sha256"]
        .as_str()
        .unwrap();
    let path = paths
        .plan
        .with_file_name("phases")
        .join(format!("{label}.json"));
    let mut phase = read_json(&path);
    phase["protocol_actions"][0]["name"] = "substituted-owner".into();
    write_json(&path, &phase);
    assert!(matches!(
        inspect(&paths, "local", "test-fleet"),
        Err(EnsureStateError::InvalidTerminalSource)
    ));
    fs::remove_dir_all(root).unwrap();
}
