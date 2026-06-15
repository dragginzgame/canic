use super::*;

#[test]
fn parses_cycle_balance_response_from_plain_candid_ok() {
    assert_eq!(
        parse_cycle_balance_response("(variant { 17_724 = 8_200_000_000_000 : nat })"),
        Some(8_200_000_000_000)
    );
    assert_eq!(
        parse_cycle_balance_response(
            r"
(
  variant {
    Ok = 99_999_000_000_000 : nat;
  },
)
"
        ),
        Some(99_999_000_000_000)
    );
}

#[test]
fn parses_cycle_balance_response_from_json_ok_and_response_candid() {
    assert_eq!(
        parse_cycle_balance_response(r#"{"Ok":"4487280757485"}"#),
        Some(4_487_280_757_485)
    );
    assert_eq!(
        parse_cycle_balance_response(
            r#"{"response_bytes":"4449444c","response_text":null,"response_candid":"(variant { 17_724 = 4_487_280_757_485 : nat })"}"#
        ),
        Some(4_487_280_757_485)
    );
    assert_eq!(
        parse_cycle_balance_response(
            r#"{"response_candid":"(variant { Ok = 8_200_000_000_000 : nat })"}"#
        ),
        Some(8_200_000_000_000)
    );
}

#[test]
fn rejects_cycle_balance_error_responses() {
    assert_eq!(
        parse_cycle_balance_response("(variant { 17_725 = record { code = 1 : nat } })"),
        None
    );
    assert_eq!(
        parse_cycle_balance_response("(variant { Err = record { code = 1 : nat } })"),
        None
    );
}
