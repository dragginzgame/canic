use super::*;
use canic_host::format::cycles_tc;

// Ensure cycle balances parse from canic_cycle_balance command output.
#[test]
fn parses_cycle_balance_from_endpoint_output() {
    assert_eq!(
        parse_cycle_balance_response("(variant { 17_724 = 4_487_280_757_485 : nat })"),
        Some(4_487_280_757_485)
    );
    assert_eq!(
        parse_cycle_balance_response(
            r#"{"response_bytes":"4449444c","response_text":null,"response_candid":"(variant { 17_724 = 4_487_280_757_485 : nat })"}"#
        ),
        Some(4_487_280_757_485)
    );
    assert_eq!(
        parse_cycle_balance_response("(variant { 17_725 = record { code = 1 : nat } })"),
        None
    );
    assert_eq!(cycles_tc(12_345_678_900_000), "12.35 TC");
}

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
