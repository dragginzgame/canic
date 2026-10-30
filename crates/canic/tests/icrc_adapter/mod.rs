//! The explicitly mounted adapter supplies native external metadata and typed consent.
use canic::{
    endpoint::ArgumentLimits,
    icrc::{ConsentInfo, ConsentMessageRequest, Icrc21Error},
};
use ic_platform::icrc_ledger_types::icrc21::{
    ConsentMessage, ConsentMessageMetadata, ConsentMessageSpec, ErrorInfo,
};
const LIMITS: ArgumentLimits = ArgumentLimits {
    max_bytes: 1024,
    decoding_quota: 100_000,
    skipping_quota: 100,
    max_type_len: 32,
    max_header_len: 512,
};
fn consent(request: ConsentMessageRequest) -> Result<ConsentInfo, Icrc21Error> {
    if request.method != "transfer" {
        return Err(Icrc21Error::UnsupportedCanisterCall(ErrorInfo {
            description: "unsupported method".to_string(),
        }));
    }
    Ok(ConsentInfo {
        consent_message: ConsentMessage::GenericDisplayMessage(request.method),
        metadata: request.user_preferences.metadata,
    })
}
canic::icrc::mount!(limits = LIMITS, consent = consent);
ic_platform::candid::contract::export_service!(ic_platform::candid::contract);
#[test]
fn mounted_consent_retains_external_type_identity_and_advertises_both_methods() {
    let did = __export_service();
    assert!(did.contains("icrc10_supported_standards : ()"), "{did}");
    assert!(
        did.contains("icrc21_canister_call_consent_message"),
        "{did}"
    );
    let supported = canic::icrc::supported_standards(true);
    assert_eq!(
        supported
            .iter()
            .map(|standard| standard.name.as_str())
            .collect::<Vec<_>>(),
        ["ICRC-10", "ICRC-21"]
    );
    assert_eq!(
        supported
            .iter()
            .map(|standard| standard.url.as_str())
            .collect::<Vec<_>>(),
        [
            "https://github.com/dfinity/ICRC/blob/main/ICRCs/ICRC-10/ICRC-10.md",
            "https://github.com/dfinity/ICRC/blob/main/ICRCs/ICRC-21/ICRC-21.md",
        ]
    );
    let reply = consent(ConsentMessageRequest {
        method: "transfer".to_string(),
        arg: vec![1],
        user_preferences: ConsentMessageSpec {
            metadata: ConsentMessageMetadata {
                language: "en".to_string(),
                utc_offset_minutes: None,
            },
            device_spec: None,
        },
    })
    .unwrap();
    assert_eq!(
        reply.consent_message,
        ConsentMessage::GenericDisplayMessage("transfer".to_string())
    );
}

#[test]
fn mounted_service_matches_the_complete_external_contract() {
    use ic_platform::candid_parser::{CandidSource, service_equal};
    let generated = __export_service();
    service_equal(
        CandidSource::Text(&generated),
        CandidSource::Text(include_str!("contract.did")),
    )
    .unwrap();
}
