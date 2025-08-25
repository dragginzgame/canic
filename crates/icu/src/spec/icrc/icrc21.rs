pub use crate::cdk::icrc_ledger_types::icrc21::{
    errors::{ErrorInfo, Icrc21Error},
    lib::ConsentMessageBuilder,
    requests::{ConsentMessageRequest, DisplayMessageType},
    responses::{ConsentInfo, ConsentMessage},
};

pub type ConsentMessageResponse = Result<ConsentMessage, Icrc21Error>;
