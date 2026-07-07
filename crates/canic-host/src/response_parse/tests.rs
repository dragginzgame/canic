use super::*;

#[test]
fn parses_cycle_balance_response_from_json_ok() {
    assert_eq!(
        parse_cycle_balance_response(r#"{"Ok":"4487280757485"}"#),
        Some(4_487_280_757_485)
    );
}

#[test]
fn rejects_cycle_balance_error_responses() {
    assert_eq!(parse_cycle_balance_response(r#"{"Err":{"code":1}}"#), None);
}
