use crate::{
    cdk::spec::icrc::icrc21::{ConsentMessageRequest, ConsentMessageResponse},
    ops::icrc::{Icrc10Ops, Icrc21Ops},
};

pub(crate) fn icrc10_supported_standards() -> Vec<(String, String)> {
    Icrc10Ops::supported_standards()
}

pub(crate) fn icrc21_consent_message(req: ConsentMessageRequest) -> ConsentMessageResponse {
    Icrc21Ops::consent_message(req)
}
