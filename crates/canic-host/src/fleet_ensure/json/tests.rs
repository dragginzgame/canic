use super::*;
use crate::fleet_ensure::model::{
    CycleConservation, EnsureAction, FLEET_ENSURE_SCHEMA_VERSION, FleetEnsurePlan,
    FleetEnsurePlanScope, FleetEnsureReport,
};
use canic_control_plane::{
    dto::template::TemplateChunkInput,
    ids::{TemplateId, TemplateVersion},
};

#[test]
fn report_projects_store_chunk_as_bounded_local_content_reference() {
    let bytes = vec![42; 64 * 1_024];
    let bytes_sha256 = sha256_hex(&bytes);
    let report = FleetEnsureReport {
        actual_conservation: None,
        effects_applied: 0,
        plan: FleetEnsurePlan {
            canisters: Vec::new(),
            conservation: CycleConservation {
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
            root_start_authority: None,
            reviewed_desired: None,
            schema_version: FLEET_ENSURE_SCHEMA_VERSION,
            scope: FleetEnsurePlanScope::Full,
            terminal_inventory_operation_id: None,
        },
        terminal: false,
    };

    let projection = report_json_value(&report).expect("project bounded Fleet report");
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

    let encoded = serde_json::to_vec(&projection).expect("encode bounded Fleet report");
    assert!(encoded.len() < 10_000);

    let mut without_chunks = report;
    without_chunks.plan.protocol_actions.clear();
    assert_eq!(
        report_json_value(&without_chunks).expect("project report without Store chunks"),
        to_value(&without_chunks).expect("encode ordinary report without Store chunks")
    );
}
