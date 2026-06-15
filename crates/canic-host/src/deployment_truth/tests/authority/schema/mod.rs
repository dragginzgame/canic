use super::super::*;

#[test]
fn authority_v1_json_schema_shape_is_stable() {
    let evidence = sample_authority_evidence();
    let value = serde_json::to_value(&evidence).expect("encode authority evidence");

    assert_object_keys(
        &value,
        &[
            "schema_version",
            "evidence_id",
            "check_id",
            "generated_at",
            "reconciliation_plan",
            "authority_report",
            "authority_receipt",
        ],
    );

    assert_object_keys(
        &value["reconciliation_plan"],
        &[
            "schema_version",
            "plan_id",
            "inventory_id",
            "authority_profile_hash",
            "canister_actions",
            "automatic_actions",
            "hard_failures",
            "external_actions_required",
        ],
    );
    assert_object_keys(
        &value["authority_report"],
        &[
            "schema_version",
            "report_id",
            "check_id",
            "reconciliation_plan_id",
            "inventory_id",
            "authority_profile_hash",
            "status",
            "summary",
            "counts",
            "apply_readiness",
            "action_counts",
            "control_class_counts",
            "observation_gaps",
            "automatic_actions",
            "hard_failures",
            "external_actions_required",
            "next_actions",
        ],
    );
    assert_object_keys(
        &value["authority_receipt"],
        &[
            "schema_version",
            "operation_id",
            "check_id",
            "reconciliation_plan_id",
            "authority_report_id",
            "inventory_id",
            "authority_profile_hash",
            "operation_status",
            "started_at",
            "finished_at",
            "attempted_actions",
            "verified_controller_observations",
            "hard_failures",
            "unresolved_observation_gaps",
            "unresolved_external_actions",
            "command_result",
        ],
    );

    assert_eq!(value["authority_report"]["status"], "Safe");
    assert_eq!(
        value["reconciliation_plan"]["canister_actions"][0]["state"],
        "AlreadyCorrect"
    );
    assert_eq!(
        value["reconciliation_plan"]["canister_actions"][0]["action"],
        "None"
    );
    assert_eq!(
        value["reconciliation_plan"]["canister_actions"][0]["control_classification"],
        "DeploymentControlled"
    );
    assert_eq!(value["authority_receipt"]["operation_status"], "Complete");
    assert_eq!(value["authority_receipt"]["command_result"], "Succeeded");
}
