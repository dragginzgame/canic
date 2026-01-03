use crate::{
    cdk::spec::icrc::icrc21::{ConsentMessageRequest, ConsentMessageResponse},
    dispatch::icrc21::Icrc21Dispatcher,
    domain::icrc::icrc10::Icrc10Registry,
};

pub fn icrc10_supported_standards() -> Vec<(String, String)> {
    Icrc10Registry::supported_standards()
}

pub fn icrc21_consent_message(req: ConsentMessageRequest) -> ConsentMessageResponse {
    Icrc21Dispatcher::consent_message(req)
}
