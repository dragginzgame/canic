use serde::Serialize;

///
/// EndpointReport
///

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(super) struct EndpointReport {
    pub(super) source: String,
    pub(super) endpoints: Vec<EndpointEntry>,
}

///
/// EndpointEntry
///

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(super) struct EndpointEntry {
    pub(super) name: String,
    pub(super) candid: String,
    pub(super) modes: Vec<EndpointMode>,
    pub(super) arguments: Vec<EndpointType>,
    pub(super) returns: Vec<EndpointType>,
}

///
/// EndpointMode
///

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum EndpointMode {
    Query,
    CompositeQuery,
    Oneway,
}

///
/// EndpointCardinality
///

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum EndpointCardinality {
    Single,
    Optional,
    Many,
}

///
/// EndpointType
///

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(super) enum EndpointType {
    Primitive {
        candid: String,
        cardinality: EndpointCardinality,
        name: String,
    },
    Named {
        candid: String,
        cardinality: EndpointCardinality,
        name: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        resolved: Option<Box<Self>>,
    },
    Optional {
        candid: String,
        cardinality: EndpointCardinality,
        inner: Box<Self>,
    },
    Vector {
        candid: String,
        cardinality: EndpointCardinality,
        inner: Box<Self>,
    },
    Record {
        candid: String,
        cardinality: EndpointCardinality,
        fields: Vec<EndpointField>,
    },
    Variant {
        candid: String,
        cardinality: EndpointCardinality,
        cases: Vec<EndpointField>,
    },
    Function {
        candid: String,
        cardinality: EndpointCardinality,
        modes: Vec<EndpointMode>,
        arguments: Vec<Self>,
        returns: Vec<Self>,
    },
    Service {
        candid: String,
        cardinality: EndpointCardinality,
        methods: Vec<EndpointServiceMethod>,
    },
    Class {
        candid: String,
        cardinality: EndpointCardinality,
        initializers: Vec<Self>,
        service: Box<Self>,
    },
}

///
/// EndpointField
///

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(super) struct EndpointField {
    pub(super) label: String,
    pub(super) id: u32,
    pub(super) ty: EndpointType,
}

///
/// EndpointServiceMethod
///

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(super) struct EndpointServiceMethod {
    pub(super) name: String,
    pub(super) ty: EndpointType,
}

///
/// EndpointTarget
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct EndpointTarget {
    pub(super) canister: String,
    pub(super) role: Option<String>,
}
