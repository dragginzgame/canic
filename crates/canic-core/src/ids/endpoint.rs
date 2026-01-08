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

///
/// EndpointId
///

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct EndpointId {
    pub name: &'static str,
}

impl EndpointId {
    #[must_use]
    pub const fn new(name: &'static str) -> Self {
        Self { name }
    }
}
