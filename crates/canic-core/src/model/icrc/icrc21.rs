use crate::cdk::spec::icrc::icrc21::{
    ConsentMessage, ConsentMessageRequest, ConsentMessageResponse, ErrorInfo,
};
use std::{cell::RefCell, collections::HashMap, sync::Arc};

//
// ICRC 21 REGISTRY
//

thread_local! {
    static ICRC_21_REGISTRY: RefCell<HashMap<String, ConsentHandlerFn>> = RefCell::new(HashMap::new());
}

///
/// ConsentHandlerFn
/// this is what the user has to pass into canic
///

pub type ConsentHandlerFn = Arc<dyn Fn(ConsentMessageRequest) -> ConsentMessageResponse + 'static>;

///
/// Icrc21Registry
///

pub struct Icrc21Registry {}

impl Icrc21Registry {
    ///
    /// Use the builder at
    /// https://docs.rs/icrc-ledger-types/latest/icrc_ledger_types/icrc21/lib/struct.ConsentMessageBuilder.html
    ///
    /// and then register the method and handler here
    ///

    pub fn register<F>(method: &str, handler: F)
    where
        F: Fn(ConsentMessageRequest) -> ConsentMessageResponse + 'static,
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

            ConsentMessageResponse::Ok(crate::cdk::spec::icrc::icrc21::ConsentInfo {
                consent_message: ConsentMessage::GenericDisplayMessage(message),
                metadata: crate::cdk::spec::icrc::icrc21::ConsentMessageMetadata {
                    language: "en".to_string(),
                    utc_offset_minutes: None,
                },
            })
        });
    }

    #[must_use]
    pub fn get_handler(method: &str) -> Option<ConsentHandlerFn> {
        ICRC_21_REGISTRY.with_borrow(|reg| reg.get(method).cloned())
    }

    #[must_use]
    pub fn consent_message(req: ConsentMessageRequest) -> ConsentMessageResponse {
        match Self::get_handler(&req.method) {
            Some(handler) => handler(req),
            None => ConsentMessageResponse::Err(
                crate::cdk::spec::icrc::icrc21::Icrc21Error::UnsupportedCanisterCall(ErrorInfo {
                    description: "No handler registered for this method.".to_string(),
                }),
            ),
        }
    }
}
