use super::*;

#[test]
fn policy_gate_report_schema_is_stable() {
    assert_eq!(
        policy_gate_report_schema(),
        PayloadSchemaRefV1 {
            id: "canic.policy_gate_report.v1".to_string(),
            version: "1".to_string(),
            stability: PayloadSchemaStabilityV1::Stable,
        }
    );
}
