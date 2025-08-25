pub use crate::cdk::icrc_ledger_types::icrc21::{
    errors::{ErrorInfo, Icrc21Error},
    requests::ConsentMessageRequest,
    responses::ConsentMessage,
};

pub type ConsentMessageResponse = Result<ConsentMessage, Icrc21Error>;
