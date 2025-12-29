use crate::api::endpoint::EndpointId;

///
/// Call
///

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Call {
    pub endpoint: EndpointId,
    pub kind: CallKind,
}

///
/// CallKind
///

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum CallKind {
    Query,
    QueryComposite,
    Update,
}
