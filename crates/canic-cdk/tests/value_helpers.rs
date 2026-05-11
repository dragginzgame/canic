use canic_cdk::{
    spec::system::http::{HttpResponse, HttpStatus},
    structures::Storable,
    types::{Cycles, WasmModule},
    utils::hash::wasm_hash,
};
use std::borrow::Cow;

// Verify cycle parsing accepts documented suffixes and plain values.
#[test]
fn cycles_parse_plain_and_suffixed_values() {
    assert_eq!("42".parse::<Cycles>().expect("plain cycles").to_u128(), 42);
    assert_eq!(
        "1.5T".parse::<Cycles>().expect("tera cycles").to_u128(),
        1_500_000_000_000
    );
    assert_eq!(
        "2Q".parse::<Cycles>()
            .expect("quadrillion cycles")
            .to_u128(),
        2_000_000_000_000_000
    );
}

// Verify invalid cycle suffixes fail instead of silently truncating meaning.
#[test]
fn cycles_reject_invalid_suffixes() {
    assert!("10TT".parse::<Cycles>().is_err());
    assert!("1X".parse::<Cycles>().is_err());
    assert!("1T2".parse::<Cycles>().is_err());
}

// Verify cycle stable bytes round-trip and corrupted bytes decode defensively.
#[test]
fn cycles_storable_round_trips_fixed_width_bytes() {
    let cycles = Cycles::new(123_456_789);
    let bytes = cycles.to_bytes();

    assert_eq!(bytes.len(), 16);
    assert_eq!(Cycles::from_bytes(bytes), cycles);
    assert_eq!(
        Cycles::from_bytes(Cow::Borrowed(&[1, 2, 3])),
        Cycles::default()
    );
}

// Verify wasm modules expose byte views and the same hash helper as raw bytes.
#[test]
fn wasm_module_exposes_bytes_and_hash() {
    static WASM_BYTES: &[u8] = b"\0asm\x01\0\0\0";
    let module = WasmModule::new(WASM_BYTES);

    assert_eq!(module.bytes(), WASM_BYTES);
    assert_eq!(module.to_vec(), WASM_BYTES);
    assert_eq!(module.len(), WASM_BYTES.len());
    assert!(!module.is_empty());
    assert_eq!(module.module_hash(), wasm_hash(WASM_BYTES));
}

// Verify HTTP error responses carry the selected status and text body.
#[test]
fn http_error_response_sets_status_header_and_body() {
    let response = HttpResponse::error(HttpStatus::BadRequest, "bad payload");

    assert_eq!(response.status_code, 400);
    assert_eq!(
        response.headers,
        vec![(
            "Content-Type".to_string(),
            "text/plain; charset=utf-8".to_string()
        )]
    );
    assert_eq!(response.body, b"bad payload");
    assert!(response.streaming_strategy.is_none());
}
