use super::parse_ready_json_value;

// Ensure readiness parsing accepts common command-line JSON result shapes.
#[test]
fn parse_ready_json_value_accepts_nested_true_shapes() {
    assert!(parse_ready_json_value(&serde_json::json!(true)));
    assert!(parse_ready_json_value(&serde_json::json!({ "Ok": true })));
    assert!(parse_ready_json_value(&serde_json::json!([{ "Ok": true }])));
}

// Ensure readiness parsing rejects false and non-boolean result shapes.
#[test]
fn parse_ready_json_value_rejects_false_shapes() {
    assert!(!parse_ready_json_value(&serde_json::json!(false)));
    assert!(!parse_ready_json_value(&serde_json::json!({ "Ok": false })));
    assert!(!parse_ready_json_value(&serde_json::json!("true")));
    assert!(!parse_ready_json_value(&serde_json::json!({ "Err": true })));
    assert!(!parse_ready_json_value(&serde_json::json!({
        "status": true
    })));
}
