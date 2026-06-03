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

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum EndpointCallKind {
    Query,
    QueryComposite,
    Update,
}

impl EndpointCallKind {
    #[must_use]
    pub const fn metric_label(self) -> &'static str {
        match self {
            Self::Query => "query",
            Self::QueryComposite => "composite_query",
            Self::Update => "update",
        }
    }
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
