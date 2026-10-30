//! Discovery-only mounting does not select a consent endpoint or handler registry.
const LIMITS: canic::endpoint::ArgumentLimits = canic::endpoint::ArgumentLimits {
    max_bytes: 1024,
    decoding_quota: 100_000,
    skipping_quota: 100,
    max_type_len: 32,
    max_header_len: 512,
};
canic::icrc::mount!(limits = LIMITS);
ic_platform::candid::contract::export_service!(ic_platform::candid::contract);
#[test]
fn discovery_only_advertises_exactly_the_mounted_standard() {
    let did = __export_service();
    assert!(did.contains("icrc10_supported_standards : ()"), "{did}");
    assert!(
        !did.contains("icrc21_canister_call_consent_message"),
        "{did}"
    );
    let supported: Vec<ic_platform::icrc_ledger_types::icrc10::SupportedStandard> =
        canic::icrc::supported_standards(false);
    assert_eq!(
        supported
            .iter()
            .map(|standard| standard.name.as_str())
            .collect::<Vec<_>>(),
        ["ICRC-10"]
    );
}

#[test]
fn discovery_matches_the_external_named_record_contract() {
    use ic_platform::candid_parser::{CandidSource, service_equal};
    // Protocol facts from ICRC/ICRCs/ICRC-10/ICRC-10.did: named fields, not a tuple.
    let expected = "type SupportedStandard = record { name : text; url : text }; service : { icrc10_supported_standards : () -> (vec SupportedStandard) query }";
    let generated = __export_service();
    service_equal(CandidSource::Text(&generated), CandidSource::Text(expected)).unwrap();
}
