//! Mount the optional adapter with a synchronous, effect-free fixture consent policy.

#[cfg(feature = "icrc-consent")]
use canic::icrc::{ConsentInfo, ConsentMessageRequest, Icrc21Error};
#[cfg(feature = "icrc-consent")]
use ic_platform::icrc_ledger_types::icrc21::{ConsentMessage, DisplayMessageType, ErrorInfo};

const LIMITS: canic::endpoint::ArgumentLimits = canic::endpoint::ArgumentLimits {
    max_bytes: 4096,
    decoding_quota: 100_000,
    skipping_quota: 1000,
    max_type_len: 128,
    max_header_len: 512,
};

#[cfg(not(feature = "icrc-consent"))]
canic::icrc::mount!(limits = LIMITS);
#[cfg(feature = "icrc-consent")]
canic::icrc::mount!(limits = LIMITS, consent = consent);

// The fixture application owns method/argument and display admission. It neither
// invokes the described operation nor writes application or framework state.
#[cfg(feature = "icrc-consent")]
fn consent(request: ConsentMessageRequest) -> Result<ConsentInfo, Icrc21Error> {
    let valid_arguments = ic_wire::native::decode_request::<()>(
        crate::protocol::GUARD,
        &request.arg,
        crate::protocol::REQUEST_LIMITS,
    )
    .is_ok();
    if request.method != "managed_guard_admission_probe" || !valid_arguments {
        return Err(Icrc21Error::UnsupportedCanisterCall(ErrorInfo {
            description: "fixture method or arguments are unsupported".to_owned(),
        }));
    }
    let preferences = request.user_preferences;
    if preferences.metadata.language != "en"
        || matches!(
            preferences.device_spec,
            Some(DisplayMessageType::FieldsDisplay)
        )
    {
        return Err(Icrc21Error::ConsentMessageUnavailable(ErrorInfo {
            description: "fixture supports an English generic display".to_owned(),
        }));
    }
    Ok(ConsentInfo {
        consent_message: ConsentMessage::GenericDisplayMessage("fixture consent".to_owned()),
        metadata: preferences.metadata,
    })
}
