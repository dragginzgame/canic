use super::*;
use crate::fleet_ensure::model::{
    CycleConservation, EnsureAction, EstateFundingDomainPlan, FLEET_ENSURE_SCHEMA_VERSION,
    FleetEnsurePlan, FleetEnsurePlanScope, FleetEnsureReport,
};
use canic_control_plane::{
    dto::template::TemplateChunkInput,
    ids::{TemplateId, TemplateVersion},
};

fn report_funding_domain() -> EstateFundingDomainPlan {
    EstateFundingDomainPlan {
        allocated_workloads: 1,
        available_cycles: Some(u128::MAX - 2),
        available_pool_slots: 2,
        creation_amount_cycles: u128::MAX - 1,
        creation_execution_margin_cycles: 1,
        readiness_floor_cycles: u128::MAX - 3,
        cycles_ledger: "um5iw-rqaaa-aaaaq-qaaba-cai".to_string(),
        eligible_ready_pool_assets: 1,
        initial_pool_assets: Vec::new(),
        ledger_fee_cycles: 1,
        management_creation_fee_cycles: 1,
        maximum_creation_debit_cycles: u128::MAX,
        maximum_creation_fee_cycles: 2,
        maximum_funding_cycles: 2,
        occupied_pool_assets: 2,
        pending_creation_count: 0,
        pending_creation: None,
        planned_initial_workloads: 2,
        pool_maximum_size: 4,
        required_creation_count: 1,
        root: "root-0".to_string(),
        root_principal: Some("rrkah-fqaaa-aaaaa-aaaaq-cai".to_string()),
        shortfall_cycles: 2,
    }
}

#[test]
fn report_projects_store_chunk_as_bounded_local_content_reference() {
    let bytes = vec![42; 64 * 1_024];
    let bytes_sha256 = sha256_hex(&bytes);
    let report = FleetEnsureReport {
        actual_conservation: None,
        effects_applied: 0,
        plan: FleetEnsurePlan {
            continuation: None,
            canisters: Vec::new(),
            conservation: CycleConservation {
                estate_funding_domains: vec![report_funding_domain()],
                expected_post_operation_cycles: 0,
                maximum_execution_burn_cycles: 0,
                maximum_new_funding_cycles: 0,
                maximum_operator_debit_cycles: u128::MAX,
                maximum_unavoidable_fee_cycles: 0,
                observed_controlled_cycles: 0,
                retained_in_reused_canisters_cycles: 0,
                scheduled_transfer_cycles: 0,
            },
            desired_sha256: "11".repeat(32),
            environment: "local".to_string(),
            fleet: "demo".to_string(),
            operation_id: "12".repeat(32),
            plan_sha256: "13".repeat(32),
            planned_at_time: 1,
            protocol_actions: vec![EnsureAction::FleetProtocol {
                action: Box::new(CurrentFleetProtocolAction::PublishStoreChunk {
                    request: TemplateChunkInput {
                        template_id: TemplateId::owned("component:app".to_string()),
                        version: TemplateVersion::owned("14".repeat(32)),
                        chunk_index: 3,
                        bytes: bytes.clone(),
                    },
                }),
                candid: "store.did".to_string(),
                candid_sha256: "15".repeat(32),
                maximum_execution_burn_cycles: 7,
                name: "publish-app-chunk-3".to_string(),
                principal: "rrkah-fqaaa-aaaaa-aaaaq-cai".to_string(),
            }],
            root_reinstall_bindings: Vec::new(),
            root_start_authority: None,
            reviewed_desired: None,
            schema_version: FLEET_ENSURE_SCHEMA_VERSION,
            scope: FleetEnsurePlanScope::Full,
            terminal_inventory_operation_id: None,
        },
        terminal: false,
    };

    let projection = report_json_value(&report).expect("project bounded Fleet report");
    for field in [
        "continuation",
        "root_start_authority",
        "reviewed_desired",
        "terminal_inventory_operation_id",
    ] {
        assert_eq!(projection["plan"].get(field), Some(&Value::Null));
    }
    assert_eq!(projection["plan"]["scope"], "full");
    let request = &projection["plan"]["protocol_actions"][0]["action"]["request"];
    assert!(request.get("bytes").is_none());
    assert_eq!(request["bytes_sha256"], bytes_sha256);
    assert_eq!(request["bytes_size"], bytes.len() as u64);
    assert_eq!(
        request["bytes_path"],
        format!("{STORE_CHUNK_OBJECT_DIRECTORY}/{bytes_sha256}")
    );
    assert_eq!(request["chunk_index"], 3);
    assert_eq!(
        projection["plan"]["conservation"]["maximum_operator_debit_cycles"],
        u128::MAX.to_string()
    );
    assert_eq!(
        projection["plan"]["conservation"]["estate_funding_domains"][0]["maximum_creation_debit_cycles"],
        u128::MAX.to_string()
    );

    let encoded = serde_json::to_vec(&projection).expect("encode bounded Fleet report");
    assert!(encoded.len() < 10_000);

    let mut without_chunks = report;
    without_chunks.plan.protocol_actions.clear();
    assert_eq!(
        report_json_value(&without_chunks).expect("project report without Store chunks"),
        to_value(&without_chunks).expect("encode ordinary report without Store chunks")
    );
}

#[test]
fn current_funding_action_emits_explicit_cycle_bounds() {
    let action = EnsureAction::Fund {
        pool_root: None,
        amount: 1,
        created_at_time: 1,
        expected_post_cycles: 0,
        funding_deficit_cycles: 0,
        funding_margin_cycles: 0,
        ledger: "ledger".into(),
        name: "funding".into(),
        principal: "target".into(),
    };
    let document = to_value(&action).unwrap();
    assert_eq!(document.get("pool_root"), Some(&Value::Null));
    for field in [
        "expected_post_cycles",
        "funding_deficit_cycles",
        "funding_margin_cycles",
    ] {
        assert_eq!(document[field], "0");
    }
    assert_eq!(
        serde_json::from_value::<EnsureAction>(document).unwrap(),
        action
    );
}
