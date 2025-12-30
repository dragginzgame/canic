use crate::api::endpoint::EndpointId;

///
/// EndpointCall
///

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct EndpointCall {
    pub endpoint: EndpointId,
    pub kind: EndpointCallKind,
}

///
/// EndpointCallKind
///

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum EndpointCallKind {
    Query,
    QueryComposite,
    Update,
}
