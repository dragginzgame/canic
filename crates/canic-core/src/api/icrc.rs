use crate::{
    PublicError,
    cdk::spec::icrc::icrc21::{ConsentMessageRequest, ConsentMessageResponse},
    workflow,
};

pub fn icrc10_supported_standards() -> Result<Vec<(String, String)>, PublicError> {
    Ok(workflow::icrc::query::icrc10_supported_standards())
}

pub fn icrc21_canister_call_consent_message(
    req: ConsentMessageRequest,
) -> Result<ConsentMessageResponse, PublicError> {
    Ok(workflow::icrc::query::icrc21_consent_message(req))
}
