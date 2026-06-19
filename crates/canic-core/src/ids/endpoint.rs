//! Module: ids::endpoint
//!
//! Responsibility: endpoint identifiers and call-kind labels.
//! Does not own: endpoint dispatch, authorization, or metrics emission.
//! Boundary: provides small typed values used by replay and observability code.

///
/// EndpointCall
///
/// One named endpoint invocation and its IC call mode.
/// Owned by ids and consumed by access, replay, and observability code.
///

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct EndpointCall {
    pub endpoint: EndpointId,
    pub kind: EndpointCallKind,
}

///
/// EndpointCallKind
///
/// IC endpoint call mode used for replay and metrics labels.
/// Owned by ids and consumed by access, replay, and observability code.
///

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum EndpointCallKind {
    Query,
    QueryComposite,
    Update,
}

impl EndpointCallKind {
    /// Return the stable metrics label for this endpoint call mode.
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
/// Static endpoint name carried through replay and observability paths.
/// Owned by ids and constructed by endpoint macros and tests.
///

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct EndpointId {
    pub name: &'static str,
}

impl EndpointId {
    /// Create an endpoint id from a static endpoint name.
    #[must_use]
    pub const fn new(name: &'static str) -> Self {
        Self { name }
    }
}
