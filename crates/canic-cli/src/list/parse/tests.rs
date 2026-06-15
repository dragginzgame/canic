use super::*;

// Ensure metadata responses provide the Canic framework version for list output.
#[test]
fn parses_canic_version_from_metadata_output() {
    assert_eq!(
        parse_canic_metadata_version_response(r#"{"package_name":"app","canic_version":"0.33.6"}"#),
        Some("0.33.6".to_string())
    );
    assert_eq!(
        parse_canic_metadata_version_response(
            r#"[{"package_name":"app","canic_version":"0.33.7"}]"#
        ),
        Some("0.33.7".to_string())
    );
    assert_eq!(
        parse_canic_metadata_version_response(
            r#"(record { package_name = "app"; canic_version = "0.33.8" })"#
        ),
        Some("0.33.8".to_string())
    );
    assert_eq!(
        parse_canic_metadata_version_response(
            r#"{"response_candid":"(record { package_name = \"app\"; canic_version = \"0.33.9\" })"}"#
        ),
        Some("0.33.9".to_string())
    );
    assert_eq!(parse_canic_metadata_version_response("{}"), None);
}
