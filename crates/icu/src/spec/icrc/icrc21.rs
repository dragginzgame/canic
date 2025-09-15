use crate::interface::prelude::*;

pub use icrc_ledger_types::icrc21::{
    errors::{ErrorInfo, Icrc21Error},
    lib::ConsentMessageBuilder,
    requests::{
        ConsentMessageMetadata, ConsentMessageRequest, ConsentMessageSpec, DisplayMessageType,
    },
    responses::{ConsentInfo, ConsentMessage},
};

///
/// ConsentMessageResponse
///

#[derive(CandidType, Deserialize)]
pub enum ConsentMessageResponse {
    Ok(ConsentInfo),
    Err(Icrc21Error),
}
