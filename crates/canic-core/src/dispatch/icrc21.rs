use crate::{
    cdk::spec::standards::icrc::icrc21::{
        ConsentInfo, ConsentMessage, ConsentMessageMetadata, ConsentMessageRequest,
        ConsentMessageResponse, ErrorInfo, Icrc21Error,
    },
    log,
    log::Topic,
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
///
/// Shared handler shape for registered ICRC-21 consent message builders.
///

pub type ConsentHandlerFn = Arc<dyn Fn(ConsentMessageRequest) -> ConsentMessageResponse + 'static>;

///
/// Icrc21Registry
///
/// Runtime dispatch table for ICRC-21 consent message handlers.
///
/// Invariants:
/// - Handlers must be registered during init/startup before any
///   ICRC-21 consent_message calls occur.
/// - Registry is process-local and cleared on upgrade.
///

pub struct Icrc21Dispatcher {}

impl Icrc21Dispatcher {
    ///
    /// Register the consent message handler for one canister method.
    ///
    /// Handlers are usually built from the upstream ICRC-21 consent message
    /// builder and installed during init/startup.
    ///

    pub fn register<F>(method: &str, handler: F)
    where
        F: Fn(ConsentMessageRequest) -> ConsentMessageResponse + 'static,
    {
        ICRC_21_REGISTRY.with_borrow_mut(|reg| {
            let replaced = reg.insert(method.to_string(), Arc::new(handler));
            if replaced.is_some() {
                log!(
                    Topic::Icrc,
                    Warn,
                    "icrc21 handler replaced for method={method}"
                );
            }
        });
    }

    pub fn register_static_with<F>(method: &str, generator: F)
    where
        F: Fn(&ConsentMessageRequest) -> String + 'static,
    {
        Self::register(method, move |req| {
            let message = generator(&req);

            ConsentMessageResponse::Ok(ConsentInfo {
                consent_message: ConsentMessage::GenericDisplayMessage(message),
                metadata: ConsentMessageMetadata {
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
    /// Resolve and execute the registered consent handler for the request.
    pub fn consent_message(req: ConsentMessageRequest) -> ConsentMessageResponse {
        match Self::get_handler(&req.method) {
            Some(handler) => handler(req),
            None => ConsentMessageResponse::Err(Icrc21Error::UnsupportedCanisterCall(ErrorInfo {
                description: "No handler registered for this method.".to_string(),
            })),
        }
    }
}
