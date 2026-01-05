use crate::{
    dispatch::icrc21::Icrc21Dispatcher,
    domain::icrc::icrc10::Icrc10Registry,
    ops::ic::icrc::{ConsentMessageRequest, ConsentMessageResponse},
};

///
/// Icrc10Query
///

pub struct Icrc10Query;

impl Icrc10Query {
    #[must_use]
    pub fn supported_standards() -> Vec<(String, String)> {
        Icrc10Registry::supported_standards()
    }
}

///
/// Icrc21Query
///

pub struct Icrc21Query;

impl Icrc21Query {
    #[must_use]
    pub fn consent_message(req: ConsentMessageRequest) -> ConsentMessageResponse {
        Icrc21Dispatcher::consent_message(req)
    }
}
