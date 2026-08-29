//! Module: workflow::icrc::query
//!
//! Responsibility: expose ICRC-10 supported standards and ICRC-21 consent-message queries.
//! Does not own: endpoint authorization, dispatcher internals, or standards DTO schemas.
//! Boundary: workflow query facade over the standards projection and dispatcher services.

use crate::{
    dispatch::icrc21::Icrc21Dispatcher,
    domain::icrc::icrc10::supported_standards,
    dto::icrc21::{ConsentMessageRequest, ConsentMessageResponse},
    ops::config::ConfigOps,
};

///
/// Icrc10Query
///

pub struct Icrc10Query;

impl Icrc10Query {
    #[must_use]
    pub fn supported_standards() -> Vec<(String, String)> {
        supported_standards(ConfigOps::current_icrc21_enabled())
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
