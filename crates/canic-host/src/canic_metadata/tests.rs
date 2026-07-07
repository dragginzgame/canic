use super::*;

#[test]
fn parses_canic_version_from_json_metadata_output() {
    assert_eq!(
        parse_canic_metadata_version_response(r#"{"canic_version":"0.67.0"}"#),
        Some("0.67.0".to_string())
    );
    assert_eq!(
        parse_canic_metadata_version_response(r#"[{"canic_version":"0.67.1"}]"#),
        Some("0.67.1".to_string())
    );
}

#[test]
fn rejects_missing_canic_version_metadata() {
    assert_eq!(parse_canic_metadata_version_response("{}"), None);
}
