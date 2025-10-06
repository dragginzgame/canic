use crate::spec::prelude::*;

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
/// Wrapper capturing the ok/error variants from an ICRC-21 consent request.
///

#[derive(CandidType, Deserialize)]
pub enum ConsentMessageResponse {
    Ok(ConsentInfo),
    Err(Icrc21Error),
}
