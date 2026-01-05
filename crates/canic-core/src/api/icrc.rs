use crate::{
    cdk::spec::icrc::icrc21::{ConsentMessageRequest, ConsentMessageResponse},
    workflow::icrc::query::{Icrc10Query, Icrc21Query},
};

///
/// Icrc10Api
///

pub struct Icrc10Api;

impl Icrc10Api {
    #[must_use]
    pub fn supported_standards() -> Vec<(String, String)> {
        Icrc10Query::supported_standards()
    }
}

///
/// Icrc21Api
///

pub struct Icrc21Api;

impl Icrc21Api {
    #[must_use]
    pub fn canister_call_consent_message(req: ConsentMessageRequest) -> ConsentMessageResponse {
        Icrc21Query::consent_message(req)
    }
}
