//! Exact declaration and infrastructure evidence must survive review hashing and restart.

use super::*;
use crate::fleet_ensure::{
    model::capacity_import::admission::{
        CapacityImportInfrastructureKind, CapacityImportInfrastructureRecord,
    },
    ops::capacity_import::{
        destination::tests::fixture, prepare_review, verify_review, with_admission,
    },
    policy::capacity_import::tests::principal,
};
use canic_core::cdk::utils::hash::hex_bytes;

fn reviewed() -> CapacityImportPlanRecord {
    let (initial, _, registry) = fixture();
    let declaration = CapacityImportDeclaration {
        canister: initial.sources[0].binding.canister_id.to_text(),
        subnet: initial.authority.subnet.into_principal().to_text(),
        controllers: initial.sources[0]
            .binding
            .controllers
            .iter()
            .map(Principal::to_text)
            .collect(),
        module_sha256: hex_bytes(initial.sources[0].binding.module_sha256.unwrap()),
        canister_version: initial.sources[0].binding.canister_version,
        disposition: CapacityImportDispositionKind::Retired,
        no_external_obligations: true,
        no_other_fleet_ownership: true,
        evidence: "Operator verified external balances and retired application obligations.".into(),
    };
    let text = toml::to_string(&CapacityImportDeclarations {
        schema_version: 1,
        operator: initial.authority.operator.to_text(),
        network_root_key_sha256: hex_bytes(initial.authority.network_root_key_sha256),
        canisters: vec![declaration.clone()],
    })
    .unwrap();
    let digest = Sha256::digest(text.as_bytes()).into();
    let mut sources = initial.sources;
    sources[0].disposition = declaration.disposition(digest);
    let plan = prepare_review(initial.authority, sources, initial.root_budget).unwrap();
    let mut controllers = plan.authority.recovery_controllers.clone();
    controllers.push(plan.authority.operator);
    controllers.sort_unstable();
    let infrastructure = vec![
        CapacityImportInfrastructureRecord {
            kind: CapacityImportInfrastructureKind::Coordinator,
            principal: plan.authority.coordinator,
            subnet: registry.authority.binding.coordinator_subnet,
            controllers: controllers.clone(),
            module_sha256: [41; 32],
        },
        CapacityImportInfrastructureRecord {
            kind: CapacityImportInfrastructureKind::Root,
            principal: plan.authority.root,
            subnet: plan.authority.subnet,
            controllers,
            module_sha256: [42; 32],
        },
        CapacityImportInfrastructureRecord {
            kind: CapacityImportInfrastructureKind::Store {
                root: plan.authority.root,
            },
            principal: principal(60),
            subnet: plan.authority.subnet,
            controllers: plan.final_controllers.clone(),
            module_sha256: [43; 32],
        },
    ];
    with_admission(
        plan,
        CapacityImportAdmissionRecord {
            infrastructure,
            registry_candid_hex: hex_bytes(candid::encode_one(&registry).unwrap()),
            declarations_toml: text,
            declarations_sha256: digest,
        },
    )
    .unwrap()
}

#[test]
fn declaration_roundtrip_binds_exact_bytes_and_keeps_cross_subnet_coordinator() {
    let plan = reviewed();
    verify_review(&plan, plan.plan_sha256).unwrap();
    let decoded: CapacityImportPlanRecord =
        serde_json::from_slice(&serde_json::to_vec(&plan).unwrap()).unwrap();
    assert_eq!(decoded, plan);
    let mut altered = plan.clone();
    altered
        .admission
        .as_mut()
        .unwrap()
        .declarations_toml
        .push('\n');
    assert!(matches!(
        validate(&altered),
        Err(CapacityImportReviewError::AdmissionInvalid)
    ));
    assert_ne!(
        plan.admission.as_ref().unwrap().infrastructure[0].subnet,
        plan.authority.subnet
    );
}

#[test]
fn altered_disposition_cannot_rebind_the_original_sources() {
    let original = reviewed();
    let changes: [fn(&mut CapacityImportDeclarations); 8] = [
        |doc| doc.canisters[0].canister_version += 1,
        |doc| doc.canisters[0].controllers.push(principal(90).to_text()),
        |doc| doc.canisters[0].module_sha256 = "empty".into(),
        |doc| doc.canisters[0].no_external_obligations = false,
        |doc| doc.canisters[0].no_other_fleet_ownership = false,
        |doc| doc.canisters[0].evidence.clear(),
        |doc| doc.canisters.push(doc.canisters[0].clone()),
        |doc| doc.operator = principal(90).to_text(),
    ];
    for change in changes {
        let mut plan = original.clone();
        let admission = plan.admission.as_mut().unwrap();
        let mut doc = declarations::parse(&admission.declarations_toml).unwrap();
        change(&mut doc);
        admission.declarations_toml = toml::to_string(&doc).unwrap();
        admission.declarations_sha256 =
            Sha256::digest(admission.declarations_toml.as_bytes()).into();
        // Even recomputing local hashes cannot turn false claims into accepted evidence.
        plan.sources[0].disposition = doc.canisters[0].disposition(admission.declarations_sha256);
        assert!(matches!(
            validate(&plan),
            Err(CapacityImportReviewError::AdmissionInvalid)
        ));
    }
}

#[test]
fn incomplete_or_conflicting_infrastructure_rejects() {
    let changes: [fn(&mut CapacityImportAdmissionRecord); 6] = [
        |record| {
            record.infrastructure.pop();
        },
        |record| record.infrastructure.push(record.infrastructure[0].clone()),
        |record| {
            record.infrastructure[2].kind = CapacityImportInfrastructureKind::Store {
                root: principal(99),
            }
        },
        |record| record.infrastructure[1].controllers.clear(),
        |record| {
            record.infrastructure[2].subnet =
                canic_core::ids::SubnetId::from_principal(principal(99));
        },
        |record| record.infrastructure[2].principal = principal(9),
    ];
    for change in changes {
        let mut plan = reviewed();
        change(plan.admission.as_mut().unwrap());
        assert!(matches!(
            validate(&plan),
            Err(CapacityImportReviewError::AdmissionInvalid)
        ));
    }
}

#[test]
fn whole_import_budget_rejects_underfunded_reservations_before_handoff() {
    use crate::fleet_ensure::{
        dto::capacity_import::CapacityImportReviewRequest,
        ops::capacity_import::journal::CapacityImportJournalError,
    };
    use canic_core::control_plane_support::policy::pool_import;
    let quote = 50_000_000_000;
    let mut request = CapacityImportReviewRequest {
        funding_credits: Vec::new(),
        environment: "test".into(),
        fleet: "test".into(),
        root: Some(principal(1)),
        canisters: (30..54).map(principal).collect(),
        declarations: "disposition.toml".into(),
        policy: "policy.toml".into(),
        seed: "seed.toml".into(),
        maximum_source_debit_cycles: 100_000_000_000,
        maximum_root_debit_cycles: 4_000_000_000_000,
        maximum_root_paid_calls: pool_import::recommended_calls(24).unwrap(),
    };
    assert!(matches!(
        validate_budget(&request, quote),
        Err(CapacityImportJournalError::InsufficientRootBudget { .. })
    ));
    request.maximum_root_debit_cycles =
        pool_import::required_debit(quote, request.maximum_root_paid_calls).unwrap();
    assert!(validate_budget(&request, quote).is_ok());
    request.maximum_root_paid_calls = pool_import::minimum_calls(24).unwrap() - 1;
    assert!(matches!(
        validate_budget(&request, quote),
        Err(CapacityImportJournalError::InsufficientRootBudget { .. })
    ));
    assert!(matches!(
        validate_budget(&request, 0),
        Err(CapacityImportJournalError::RequestInvalid)
    ));
}
