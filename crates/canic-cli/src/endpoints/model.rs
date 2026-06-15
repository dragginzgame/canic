use canic_host::candid_endpoints::EndpointEntry;
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
/// EndpointTarget
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct EndpointTarget {
    pub(super) canister: String,
    pub(super) role: Option<String>,
}
