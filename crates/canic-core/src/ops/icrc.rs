use crate::{
    cdk::spec::icrc::{
        icrc10::Icrc10Standard,
        icrc21::{ConsentMessageRequest, ConsentMessageResponse},
    },
    dispatch::icrc21::Icrc21Dispatcher,
    domain::icrc::icrc10::Icrc10Registry,
};

///
/// Icrc10Ops
///
/// Used by macro-generated endpoints in downstream crates.
///

#[allow(dead_code)]
pub struct Icrc10Ops;

#[allow(dead_code)]
impl Icrc10Ops {
    /// Return the supported standards as `(name, url)` tuples.
    #[must_use]
    pub fn supported_standards() -> Vec<(String, String)> {
        Icrc10Registry::supported_standards()
    }

    /// Check whether a standard is registered.
    #[must_use]
    pub fn is_registered(standard: Icrc10Standard) -> bool {
        Icrc10Registry::is_registered(standard)
    }
}

///
/// Icrc21Ops
///
/// Used by macro-generated endpoints in downstream crates.
///

#[allow(dead_code)]
pub struct Icrc21Ops;

#[allow(dead_code)]
impl Icrc21Ops {
    /// Register a consent message handler for a method.
    pub fn register<F>(method: &str, handler: F)
    where
        F: Fn(ConsentMessageRequest) -> ConsentMessageResponse + 'static,
    {
        Icrc21Dispatcher::register(method, handler);
    }

    /// Register a consent message generator for a method (static message).
    pub fn register_static_with<F>(method: &str, generator: F)
    where
        F: Fn(&ConsentMessageRequest) -> String + 'static,
    {
        Icrc21Dispatcher::register_static_with(method, generator);
    }

    /// Generate a consent message using registered handlers.
    #[must_use]
    pub fn consent_message(req: ConsentMessageRequest) -> ConsentMessageResponse {
        Icrc21Dispatcher::consent_message(req)
    }
}
