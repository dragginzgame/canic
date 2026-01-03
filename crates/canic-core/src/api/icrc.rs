use crate::{
    cdk::spec::icrc::icrc21::{ConsentMessageRequest, ConsentMessageResponse},
    workflow,
};

#[must_use]
pub fn icrc10_supported_standards() -> Vec<(String, String)> {
    workflow::icrc::query::icrc10_supported_standards()
}

#[must_use]
pub fn icrc21_canister_call_consent_message(req: ConsentMessageRequest) -> ConsentMessageResponse {
    workflow::icrc::query::icrc21_consent_message(req)
}
