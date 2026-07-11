use super::{QueryOutcome, decode_query_response, decode_status_root_key, encode_anonymous_query};

#[test]
fn anonymous_query_envelope_has_exact_wire_bytes() {
    let bytes = encode_anonymous_query(&[1, 2, 3], "canic_ready", &[0x44, 0x49, 0x44, 0x4c], 42)
        .expect("encode query");

    assert_eq!(
        bytes,
        fixture(
            "a167636f6e74656e74a66c726571756573745f747970656571756572796b63616e69737465725f6964430102036b6d6574686f645f6e616d656b63616e69635f726561647963617267444449444c6673656e64657241046e696e67726573735f657870697279182a"
        )
    );
}

#[test]
fn replied_query_response_has_exact_wire_bytes() {
    let bytes = fixture("a266737461747573677265706c696564657265706c79a163617267444449444c");

    assert!(matches!(
        decode_query_response(&bytes).expect("decode reply"),
        QueryOutcome::Replied(arg) if arg == [0x44, 0x49, 0x44, 0x4c]
    ));
}

#[test]
fn rejected_query_response_has_exact_wire_bytes() {
    let bytes = fixture(
        "a3667374617475736872656a65637465646b72656a6563745f636f6465056e72656a6563745f6d65737361676567626c6f636b6564",
    );

    assert!(matches!(
        decode_query_response(&bytes).expect("decode rejection"),
        QueryOutcome::Rejected { code: 5, message } if message == "blocked"
    ));
}

#[test]
fn status_root_key_bytes_have_exact_wire_bytes() {
    let bytes = fixture("a168726f6f745f6b657943308182");

    assert_eq!(
        decode_status_root_key(&bytes).expect("decode status root key"),
        Some("308182".to_string())
    );
}

fn fixture(hex: &str) -> Vec<u8> {
    hex.as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let pair = std::str::from_utf8(pair).expect("fixture hex is UTF-8");
            u8::from_str_radix(pair, 16).expect("fixture hex byte is valid")
        })
        .collect()
}

#[test]
fn malformed_and_unsupported_cbor_are_rejected() {
    assert!(decode_status_root_key(&[0xff]).is_err());
    assert!(decode_status_root_key(&[0xf6]).is_err());
    assert!(decode_query_response(&[0xff]).is_err());
}
