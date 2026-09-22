use canic_core::ids::ReleaseBuildId;
use canic_host::candid_endpoints::EndpointEntry;
use serde::Serialize;

///
/// EndpointReport
///

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(super) struct EndpointReport {
    pub(super) source: String,
    pub(super) source_kind: EndpointSourceKind,
    pub(super) release_build_id: Option<ReleaseBuildId>,
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

/// Origin of the declaration being inspected, independent of its display path.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum EndpointSourceKind {
    Built,
    Live,
    Local,
}
