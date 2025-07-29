use crate::interface::icrc::*;
use derive_more::{Deref, DerefMut};
use std::{cell::RefCell, collections::HashMap};

//
// ICRC 21 REGISTRY
//

thread_local! {
    static ICRC_21_REGISTRY: RefCell<Icrc21Registry> = RefCell::new(Icrc21Registry::new());
}

///
/// ConsentHandlerFn
/// this is what the user has to pass into icu
///

pub type Icrc21ConsentHandlerFn =
    fn(request: Icrc21ConsentMessageRequest) -> Result<Icrc21ConsentMessageResponse, String>;

///
/// Icrc21Registry
///

#[derive(Default, Debug, Deref, DerefMut)]
pub struct Icrc21Registry(pub HashMap<String, Icrc21ConsentHandlerFn>);

impl Icrc21Registry {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(method: &str, handler: Icrc21ConsentHandlerFn) {
        ICRC_21_REGISTRY.with_borrow_mut(|reg| reg.insert(method.to_string(), handler));
    }

    #[must_use]
    pub fn get_handler(method: &str) -> Option<Icrc21ConsentHandlerFn> {
        ICRC_21_REGISTRY.with_borrow(|reg| reg.get(method).copied())
    }

    #[must_use]
    pub fn consent_message(req: Icrc21ConsentMessageRequest) -> Icrc21ConsentMessageResponse {
        match Self::get_handler(&req.method) {
            Some(handler) => match handler(req) {
                Ok(response) => response,

                Err(desc) => Icrc21ConsentMessageResponse::Err(Icrc21Error::GenericError {
                    error_code: 1,
                    description: desc,
                }),
            },
            None => Icrc21ConsentMessageResponse::Err(Icrc21Error::UnsupportedCanisterCall(
                Icrc21ErrorInfo {
                    description: "No handler registered for this method.".to_string(),
                },
            )),
        }
    }
}
