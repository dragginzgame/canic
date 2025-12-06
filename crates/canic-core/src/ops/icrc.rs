use crate::{
    model::icrc::{Icrc10Registry, Icrc21Registry},
    spec::icrc::{icrc10::Icrc10Standard, icrc21::ConsentMessageRequest},
};

///
/// Icrc10Ops
///

pub struct Icrc10Ops;

impl Icrc10Ops {
    #[must_use]
    pub fn supported_standards() -> Vec<(String, String)> {
        Icrc10Registry::supported_standards()
    }

    #[must_use]
    pub fn is_registered(standard: Icrc10Standard) -> bool {
        Icrc10Registry::is_registered(standard)
    }
}

///
/// Icrc21Ops
///

pub struct Icrc21Ops;

impl Icrc21Ops {
    pub fn register<F>(method: &str, handler: F)
    where
        F: Fn(ConsentMessageRequest) -> crate::spec::icrc::icrc21::ConsentMessageResponse + 'static,
    {
        Icrc21Registry::register(method, handler);
    }

    pub fn register_static_with<F>(method: &str, generator: F)
    where
        F: Fn(&ConsentMessageRequest) -> String + 'static,
    {
        Icrc21Registry::register_static_with(method, generator);
    }

    #[must_use]
    pub fn consent_message(
        req: ConsentMessageRequest,
    ) -> crate::spec::icrc::icrc21::ConsentMessageResponse {
        Icrc21Registry::consent_message(req)
    }
}
