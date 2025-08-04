use crate::interface::icrc::*;
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

pub type Icrc21ConsentHandlerFn =
    Arc<dyn Fn(Icrc21ConsentMessageRequest) -> Icrc21ConsentMessageResponse + 'static>;

///
/// Icrc21Registry
///

pub struct Icrc21Registry {}

impl Icrc21Registry {
    pub fn register<F>(method: &str, handler: F)
    where
        F: Fn(Icrc21ConsentMessageRequest) -> Icrc21ConsentMessageResponse + 'static,
    {
        ICRC_21_REGISTRY.with_borrow_mut(|reg| {
            reg.insert(method.to_string(), Arc::new(handler));
        });
    }

    pub fn register_static_with<F>(method: &str, generator: F)
    where
        F: Fn(&Icrc21ConsentMessageRequest) -> String + 'static,
    {
        Self::register(method, move |req| {
            let message = generator(&req);
            Icrc21ConsentMessageResponse::Ok(Icrc21ConsentInfo {
                consent_message: Icrc21ConsentMessage::GenericDisplayMessage(message),
                metadata: req.user_preferences.metadata,
            })
        });
    }

    #[must_use]
    pub fn get_handler(method: &str) -> Option<Icrc21ConsentHandlerFn> {
        ICRC_21_REGISTRY.with_borrow(|reg| reg.get(method).cloned())
    }

    #[must_use]
    pub fn consent_message(req: Icrc21ConsentMessageRequest) -> Icrc21ConsentMessageResponse {
        match Self::get_handler(&req.method) {
            Some(handler) => handler(req),
            None => Icrc21ConsentMessageResponse::Err(Icrc21Error::UnsupportedCanisterCall(
                Icrc21ErrorInfo {
                    description: "No handler registered for this method.".to_string(),
                },
            )),
        }
    }
}
