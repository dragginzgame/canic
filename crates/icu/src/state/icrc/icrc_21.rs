use crate::Error;
use candid::CandidType;
use derive_more::{Deref, DerefMut};
use serde::{Deserialize, Serialize};
use std::{cell::RefCell, collections::HashMap};

//
// ICRC 21 REGISTRY
//

thread_local! {
    static ICRC_21_REGISTRY: RefCell<Icrc21Registry> = RefCell::new(Icrc21Registry::new());
}

///
/// Data Structures
///

pub type ConsentHandlerFn =
    fn(arg: Vec<u8>, prefs: ConsentPreferences) -> Result<Option<ConsentMessage>, Error>;

#[derive(Clone, Debug, CandidType, Deserialize, Serialize)]
pub struct ConsentMessage {
    pub consent_message: String,
    pub language: String,
}

#[derive(CandidType, Deserialize, Clone)]
pub struct ConsentPreferences {
    pub language: String,
}

#[derive(CandidType, Serialize)]
pub enum Icrc21ConsentMessageResponse {
    Ok {
        consent_message: String,
        language: String,
    },
}

#[derive(CandidType, Deserialize)]
pub struct Icrc21ConsentMessageRequest {
    pub method: String,
    pub arg: Vec<u8>, // Candid-encoded
    pub consent_preferences: ConsentPreferences,
}

///
/// Icrc21Registry
///

#[derive(Default, Debug, Deref, DerefMut)]
pub struct Icrc21Registry(pub HashMap<String, ConsentHandlerFn>);

impl Icrc21Registry {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, method: &str, handler: ConsentHandlerFn) {
        self.insert(method.to_string(), handler);
    }

    #[must_use]
    pub fn get_handler(&self, method: &str) -> Option<&ConsentHandlerFn> {
        self.get(method)
    }

    #[must_use]
    pub fn consent_message(req: Icrc21ConsentMessageRequest) -> Icrc21ConsentMessageResponse {
        ICRC_21_REGISTRY.with_borrow(|reg| match reg.get_handler(&req.method) {
            Some(handler) => match handler(req.arg.clone(), req.consent_preferences.clone()) {
                Ok(Some(msg)) => Icrc21ConsentMessageResponse::Ok {
                    consent_message: msg.consent_message,
                    language: msg.language,
                },
                Ok(None) => Icrc21ConsentMessageResponse::Ok {
                    consent_message: "No consent message available.".to_string(),
                    language: "en-US".to_string(),
                },
                Err(_) => Icrc21ConsentMessageResponse::Ok {
                    consent_message: "Error generating consent message.".to_string(),
                    language: "en-US".to_string(),
                },
            },
            None => Icrc21ConsentMessageResponse::Ok {
                consent_message: "No handler registered for method.".to_string(),
                language: "en-US".to_string(),
            },
        })
    }
}
