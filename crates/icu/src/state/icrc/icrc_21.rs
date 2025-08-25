use crate::spec::icrc::icrc21::{ConsentMessage, ConsentMessageRequest, ErrorInfo, Icrc21Error};
use std::{cell::RefCell, collections::HashMap, sync::Arc};

//
// ICRC 21 REGISTRY
//

thread_local! {
    static ICRC_21_REGISTRY: RefCell<HashMap<String, Icrc21ConsentHandlerFn>> = RefCell::new(HashMap::new());
}

///
/// ConsentHandlerFn
/// this is what the user has to pass into icu
///

pub type Icrc21ConsentHandlerFn = Arc<dyn Fn(ConsentMessageRequest) -> ConsentMessage + 'static>;

///
/// Icrc21Registry
///

pub struct Icrc21Registry {}

impl Icrc21Registry {
    ///
    /// Use the builder at
    /// https://docs.rs/icrc-ledger-types/latest/icrc_ledger_types/icrc21/lib/struct.ConsentMessageBuilder.html#
    ///
    /// and then register the method and handler here
    ///

    pub fn register<F>(method: &str, handler: F)
    where
        F: Fn(ConsentMessageRequest) -> ConsentMessage + 'static,
    {
        ICRC_21_REGISTRY.with_borrow_mut(|reg| {
            reg.insert(method.to_string(), Arc::new(handler));
        });
    }

    pub fn register_static_with<F>(method: &str, generator: F)
    where
        F: Fn(&ConsentMessageRequest) -> String + 'static,
    {
        Self::register(method, move |req| {
            let message = generator(&req);

            ConsentMessage::GenericDisplayMessage(message)
        });
    }

    #[must_use]
    pub fn get_handler(method: &str) -> Option<Icrc21ConsentHandlerFn> {
        ICRC_21_REGISTRY.with_borrow(|reg| reg.get(method).cloned())
    }

    pub fn consent_message(req: ConsentMessageRequest) -> Result<ConsentMessage, Icrc21Error> {
        match Self::get_handler(&req.method) {
            Some(handler) => Ok(handler(req)),
            None => Err(Icrc21Error::UnsupportedCanisterCall(ErrorInfo {
                description: "No handler registered for this method.".to_string(),
            })),
        }
    }
}
