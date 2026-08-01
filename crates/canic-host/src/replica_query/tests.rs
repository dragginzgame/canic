use super::decode_cycle_balance_response;
use candid::Encode;
use canic_core::dto::error::Error as CanicError;

#[test]
fn decodes_cycle_balance_response_bytes() {
    let response: Result<u128, CanicError> = Ok(99_999_000_000_000);
    let bytes = Encode!(&response).expect("encode cycle balance response");

    assert_eq!(
        decode_cycle_balance_response(&bytes).expect("decode cycle balance"),
        99_999_000_000_000
    );
}
