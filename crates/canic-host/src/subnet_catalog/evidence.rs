//! Module: subnet_catalog::evidence
//!
//! Responsibility: project upstream typed catalog failures into Canic planning evidence.
//! Does not own: catalog collection, retry classification, report rendering, or effects.
//! Boundary: every field is copied from typed `ic-query` data without parsing error prose.

use ic_query::subnet_catalog::{
    CatalogSourceSelection, SubnetCatalogFailureCacheDisposition, SubnetCatalogField,
    SubnetCatalogLoadFailure, SubnetCatalogLoadStage, SubnetCatalogRefreshTrigger,
    SubnetCatalogRegistryRecordEvidence, SubnetCatalogRegistryRecordKind,
    SubnetCatalogRegistryValueEncoding, SubnetCatalogRetryability, SubnetCatalogSubject,
    SubnetCatalogUnknownRetryReason,
};
use serde::Serialize;

/// Complete Canic-owned projection of one typed upstream catalog-load failure.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SubnetCatalogLoadFailureEvidenceV1 {
    pub schema_version: u16,
    pub network: String,
    pub source_kind: Option<SubnetCatalogSourceKindV1>,
    pub source_endpoints: Vec<String>,
    pub source_assurance: Option<String>,
    pub minimum_assurance: String,
    pub stage: SubnetCatalogLoadStageV1,
    pub registry_version: Option<u64>,
    pub returned_registry_value_version: Option<u64>,
    pub source_endpoint: Option<String>,
    pub assurance: Option<String>,
    pub registry_records: Vec<SubnetCatalogRegistryRecordEvidenceV1>,
    pub cache_disposition: SubnetCatalogFailureCacheDispositionV1,
    pub subject: Option<SubnetCatalogSubjectV1>,
    pub code: String,
    pub category: String,
    pub retryability: SubnetCatalogRetryabilityV1,
    pub source_message: String,
    pub effects: SubnetCatalogFailureEffectsV1,
}

impl SubnetCatalogLoadFailureEvidenceV1 {
    /// Project one detailed failure observed at Canic's read-only preflight boundary.
    #[must_use]
    pub fn from_preflight_failure(failure: &SubnetCatalogLoadFailure) -> Self {
        let (source_kind, source_endpoints, source_assurance) = failure
            .request
            .source
            .as_ref()
            .map_or_else(|| (None, Vec::new(), None), source_evidence);
        Self {
            schema_version: 1,
            network: failure.request.network.clone(),
            source_kind,
            source_endpoints,
            source_assurance,
            minimum_assurance: failure.request.minimum_assurance.as_str().to_string(),
            stage: failure.stage.into(),
            registry_version: failure.registry_version,
            returned_registry_value_version: failure.returned_registry_value_version,
            source_endpoint: failure.source_endpoint.clone(),
            assurance: failure
                .assurance
                .map(|assurance| assurance.as_str().to_string()),
            registry_records: failure.registry_records.iter().map(Into::into).collect(),
            cache_disposition: failure.cache_disposition.into(),
            subject: failure.subject.as_ref().map(Into::into),
            code: failure.code.as_str().to_string(),
            category: failure.category.as_str().to_string(),
            retryability: failure.retryability.into(),
            source_message: failure.source.to_string(),
            effects: SubnetCatalogFailureEffectsV1::none_started(),
        }
    }
}

fn source_evidence(
    source: &CatalogSourceSelection,
) -> (
    Option<SubnetCatalogSourceKindV1>,
    Vec<String>,
    Option<String>,
) {
    let (kind, endpoints) = match source {
        CatalogSourceSelection::UncertifiedQuery { endpoint } => (
            SubnetCatalogSourceKindV1::UncertifiedQuery,
            vec![endpoint.clone()],
        ),
        CatalogSourceSelection::MultiEndpointAgreement { endpoints } => (
            SubnetCatalogSourceKindV1::MultiEndpointAgreement,
            endpoints.clone(),
        ),
    };
    (
        Some(kind),
        endpoints,
        Some(source.assurance().as_str().to_string()),
    )
}

/// Typed source-selection shape attempted by the failed request.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SubnetCatalogSourceKindV1 {
    UncertifiedQuery,
    MultiEndpointAgreement,
}

/// Exact typed upstream load stage.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SubnetCatalogLoadStageV1 {
    RequestValidation,
    CacheOnlyLoad,
    CacheLookup,
    CacheAbsence,
    CacheRejection,
    CacheBypass,
    RefreshAttempted,
    RefreshFailed,
    PostRefreshCacheLoadFailed,
    RuntimeAdapter,
}

impl From<SubnetCatalogLoadStage> for SubnetCatalogLoadStageV1 {
    fn from(value: SubnetCatalogLoadStage) -> Self {
        match value {
            SubnetCatalogLoadStage::RequestValidation => Self::RequestValidation,
            SubnetCatalogLoadStage::CacheOnlyLoad => Self::CacheOnlyLoad,
            SubnetCatalogLoadStage::CacheLookup => Self::CacheLookup,
            SubnetCatalogLoadStage::CacheAbsence => Self::CacheAbsence,
            SubnetCatalogLoadStage::CacheRejection => Self::CacheRejection,
            SubnetCatalogLoadStage::CacheBypass => Self::CacheBypass,
            SubnetCatalogLoadStage::RefreshAttempted => Self::RefreshAttempted,
            SubnetCatalogLoadStage::RefreshFailed => Self::RefreshFailed,
            SubnetCatalogLoadStage::PostRefreshCacheLoadFailed => Self::PostRefreshCacheLoadFailed,
            SubnetCatalogLoadStage::RuntimeAdapter => Self::RuntimeAdapter,
        }
    }
}

/// Exact typed cache state or attempted action at failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SubnetCatalogFailureCacheDispositionV1 {
    NotExamined,
    CacheOnly,
    CacheBypassed,
    CacheMissing,
    CacheRejected,
    CacheReadFailed,
    RefreshAttempted {
        trigger: SubnetCatalogRefreshTriggerV1,
    },
    RefreshFailed {
        trigger: SubnetCatalogRefreshTriggerV1,
    },
    PostRefreshLoadFailed {
        trigger: SubnetCatalogRefreshTriggerV1,
    },
}

impl From<SubnetCatalogFailureCacheDisposition> for SubnetCatalogFailureCacheDispositionV1 {
    fn from(value: SubnetCatalogFailureCacheDisposition) -> Self {
        match value {
            SubnetCatalogFailureCacheDisposition::NotExamined => Self::NotExamined,
            SubnetCatalogFailureCacheDisposition::CacheOnly => Self::CacheOnly,
            SubnetCatalogFailureCacheDisposition::CacheBypassed => Self::CacheBypassed,
            SubnetCatalogFailureCacheDisposition::CacheMissing => Self::CacheMissing,
            SubnetCatalogFailureCacheDisposition::CacheRejected => Self::CacheRejected,
            SubnetCatalogFailureCacheDisposition::CacheReadFailed => Self::CacheReadFailed,
            SubnetCatalogFailureCacheDisposition::RefreshAttempted(trigger) => {
                Self::RefreshAttempted {
                    trigger: trigger.into(),
                }
            }
            SubnetCatalogFailureCacheDisposition::RefreshFailed(trigger) => Self::RefreshFailed {
                trigger: trigger.into(),
            },
            SubnetCatalogFailureCacheDisposition::PostRefreshLoadFailed(trigger) => {
                Self::PostRefreshLoadFailed {
                    trigger: trigger.into(),
                }
            }
        }
    }
}

/// Typed reason the refresh path was selected.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SubnetCatalogRefreshTriggerV1 {
    Missing,
    Rejected,
    Stale,
    Forced,
}

impl From<SubnetCatalogRefreshTrigger> for SubnetCatalogRefreshTriggerV1 {
    fn from(value: SubnetCatalogRefreshTrigger) -> Self {
        match value {
            SubnetCatalogRefreshTrigger::Missing => Self::Missing,
            SubnetCatalogRefreshTrigger::Rejected => Self::Rejected,
            SubnetCatalogRefreshTrigger::Stale => Self::Stale,
            SubnetCatalogRefreshTrigger::Forced => Self::Forced,
        }
    }
}

/// Typed offending catalog identity retained by the upstream collector.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SubnetCatalogSubjectV1 {
    Network {
        network: String,
    },
    Endpoint {
        endpoint: String,
    },
    CachePath {
        path: String,
    },
    RegistryLatestVersion,
    RegistryRecord {
        record_kind: SubnetCatalogRegistryRecordKindV1,
        key: String,
        subnet: Option<String>,
        canister_range_start: Option<String>,
    },
    Subnet {
        subnet: String,
        field: Option<SubnetCatalogFieldV1>,
    },
    RegistryRoutingTableEntry {
        index: usize,
        field: Option<SubnetCatalogFieldV1>,
    },
    RoutingRange {
        start_canister_id: String,
        end_canister_id: String,
        subnet_principal: String,
        field: Option<SubnetCatalogFieldV1>,
    },
    Field {
        field: SubnetCatalogFieldV1,
    },
}

impl From<&SubnetCatalogSubject> for SubnetCatalogSubjectV1 {
    fn from(value: &SubnetCatalogSubject) -> Self {
        match value {
            SubnetCatalogSubject::Network(network) => Self::Network {
                network: network.clone(),
            },
            SubnetCatalogSubject::Endpoint(endpoint) => Self::Endpoint {
                endpoint: endpoint.clone(),
            },
            SubnetCatalogSubject::CachePath(path) => Self::CachePath {
                path: path.display().to_string(),
            },
            SubnetCatalogSubject::RegistryLatestVersion => Self::RegistryLatestVersion,
            SubnetCatalogSubject::RegistryRecord(record) => Self::RegistryRecord {
                record_kind: record.kind.into(),
                key: record.key.clone(),
                subnet: record.subnet.map(|subnet| subnet.to_text()),
                canister_range_start: record
                    .canister_range_start
                    .map(|canister| canister.to_text()),
            },
            SubnetCatalogSubject::Subnet { subnet, field } => Self::Subnet {
                subnet: subnet.to_text(),
                field: field.map(Into::into),
            },
            SubnetCatalogSubject::RegistryRoutingTableEntry { index, field } => {
                Self::RegistryRoutingTableEntry {
                    index: *index,
                    field: field.map(Into::into),
                }
            }
            SubnetCatalogSubject::RoutingRange { range, field } => Self::RoutingRange {
                start_canister_id: range.start_canister_id.clone(),
                end_canister_id: range.end_canister_id.clone(),
                subnet_principal: range.subnet_principal.clone(),
                field: field.map(Into::into),
            },
            SubnetCatalogSubject::Field(field) => Self::Field {
                field: (*field).into(),
            },
        }
    }
}

/// Typed Registry record family retained on failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SubnetCatalogRegistryRecordKindV1 {
    SubnetList,
    RoutingTable,
    SubnetRecord,
}

impl From<SubnetCatalogRegistryRecordKind> for SubnetCatalogRegistryRecordKindV1 {
    fn from(value: SubnetCatalogRegistryRecordKind) -> Self {
        match value {
            SubnetCatalogRegistryRecordKind::SubnetList => Self::SubnetList,
            SubnetCatalogRegistryRecordKind::RoutingTable => Self::RoutingTable,
            SubnetCatalogRegistryRecordKind::SubnetRecord => Self::SubnetRecord,
        }
    }
}

/// One successful Registry value read completed before the catalog failure.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SubnetCatalogRegistryRecordEvidenceV1 {
    pub record_kind: SubnetCatalogRegistryRecordKindV1,
    pub key: String,
    pub subnet: Option<String>,
    pub canister_range_start: Option<String>,
    pub requested_registry_version: u64,
    pub returned_registry_version: u64,
    pub timestamp_nanoseconds: u64,
    pub source_endpoint: String,
    pub assurance: String,
    pub value_encoding: SubnetCatalogRegistryValueEncodingV1,
}

impl From<&SubnetCatalogRegistryRecordEvidence> for SubnetCatalogRegistryRecordEvidenceV1 {
    fn from(value: &SubnetCatalogRegistryRecordEvidence) -> Self {
        Self {
            record_kind: value.record.kind.into(),
            key: value.record.key.clone(),
            subnet: value.record.subnet.map(|subnet| subnet.to_text()),
            canister_range_start: value
                .record
                .canister_range_start
                .map(|canister| canister.to_text()),
            requested_registry_version: value.requested_registry_version,
            returned_registry_version: value.returned_registry_version,
            timestamp_nanoseconds: value.timestamp_nanoseconds,
            source_endpoint: value.source_endpoint.clone(),
            assurance: value.assurance.as_str().to_string(),
            value_encoding: value.value_encoding.into(),
        }
    }
}

/// Exact transport representation used for a completed Registry value read.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SubnetCatalogRegistryValueEncodingV1 {
    Inline,
    Chunked,
}

impl From<SubnetCatalogRegistryValueEncoding> for SubnetCatalogRegistryValueEncodingV1 {
    fn from(value: SubnetCatalogRegistryValueEncoding) -> Self {
        match value {
            SubnetCatalogRegistryValueEncoding::Inline => Self::Inline,
            SubnetCatalogRegistryValueEncoding::Chunked => Self::Chunked,
        }
    }
}

/// Typed catalog field retained on failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SubnetCatalogFieldV1 {
    SubnetListEntry,
    RoutingTableRange,
    RoutingTableSubnetId,
    RoutingRangeStart,
    RoutingRangeEnd,
    Network,
    RegistryCanister,
    RegistryVersion,
    SourceEndpoint,
    SubnetPrincipal,
    CollectionTimestamp,
    Classification,
    AgreementDigest,
    CatalogDigest,
    Provenance,
}

impl From<SubnetCatalogField> for SubnetCatalogFieldV1 {
    fn from(value: SubnetCatalogField) -> Self {
        match value {
            SubnetCatalogField::SubnetListEntry => Self::SubnetListEntry,
            SubnetCatalogField::RoutingTableRange => Self::RoutingTableRange,
            SubnetCatalogField::RoutingTableSubnetId => Self::RoutingTableSubnetId,
            SubnetCatalogField::RoutingRangeStart => Self::RoutingRangeStart,
            SubnetCatalogField::RoutingRangeEnd => Self::RoutingRangeEnd,
            SubnetCatalogField::Network => Self::Network,
            SubnetCatalogField::RegistryCanister => Self::RegistryCanister,
            SubnetCatalogField::RegistryVersion => Self::RegistryVersion,
            SubnetCatalogField::SourceEndpoint => Self::SourceEndpoint,
            SubnetCatalogField::SubnetPrincipal => Self::SubnetPrincipal,
            SubnetCatalogField::CollectionTimestamp => Self::CollectionTimestamp,
            SubnetCatalogField::Classification => Self::Classification,
            SubnetCatalogField::AgreementDigest => Self::AgreementDigest,
            SubnetCatalogField::CatalogDigest => Self::CatalogDigest,
            SubnetCatalogField::Provenance => Self::Provenance,
        }
    }
}

/// Truthful retry classification, including the typed unknown reason.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SubnetCatalogRetryabilityV1 {
    Retryable,
    NotRetryable,
    Unknown {
        reason: SubnetCatalogUnknownRetryReasonV1,
    },
}

impl From<SubnetCatalogRetryability> for SubnetCatalogRetryabilityV1 {
    fn from(value: SubnetCatalogRetryability) -> Self {
        match value {
            SubnetCatalogRetryability::Retryable => Self::Retryable,
            SubnetCatalogRetryability::NotRetryable => Self::NotRetryable,
            SubnetCatalogRetryability::Unknown(reason) => Self::Unknown {
                reason: reason.into(),
            },
        }
    }
}

/// Typed reason binary retryability is not justified.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SubnetCatalogUnknownRetryReasonV1 {
    CacheOperation,
    RegistryResponse,
    RegistryTransport,
    RuntimeAdapter,
}

impl From<SubnetCatalogUnknownRetryReason> for SubnetCatalogUnknownRetryReasonV1 {
    fn from(value: SubnetCatalogUnknownRetryReason) -> Self {
        match value {
            SubnetCatalogUnknownRetryReason::CacheOperation => Self::CacheOperation,
            SubnetCatalogUnknownRetryReason::RegistryResponse => Self::RegistryResponse,
            SubnetCatalogUnknownRetryReason::RegistryTransport => Self::RegistryTransport,
            SubnetCatalogUnknownRetryReason::RuntimeAdapter => Self::RuntimeAdapter,
        }
    }
}

/// Canic-local fact that catalog failure propagation remains before every effect boundary.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub struct SubnetCatalogFailureEffectsV1 {
    pub build_started: bool,
    pub workspace_mutation_started: bool,
    pub ic_mutation_started: bool,
}

impl SubnetCatalogFailureEffectsV1 {
    const fn none_started() -> Self {
        Self {
            build_started: false,
            workspace_mutation_started: false,
            ic_mutation_started: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use candid::Principal;
    use ic_query::subnet_catalog::{
        CatalogAssurance, SubnetCatalogErrorCategory, SubnetCatalogErrorCode,
        SubnetCatalogHostError, SubnetCatalogLoadFailureRequest,
        SubnetCatalogRegistryRecordSubject, SubnetCatalogRegistryValueEncoding,
    };

    #[test]
    fn projection_preserves_every_typed_failure_field_without_reclassification() {
        let subnet = Principal::from_slice(&[9; 29]);
        let canister_range_start = Principal::from_slice(&[7; 29]);
        let failure = detailed_failure_fixture();

        let evidence = SubnetCatalogLoadFailureEvidenceV1::from_preflight_failure(&failure);

        assert_eq!(evidence.network, "ic");
        assert_eq!(
            evidence.source_kind,
            Some(SubnetCatalogSourceKindV1::UncertifiedQuery)
        );
        assert_eq!(evidence.source_endpoints, ["https://ic0.app"]);
        assert_eq!(
            evidence.source_assurance.as_deref(),
            Some("uncertified_query")
        );
        assert_eq!(evidence.minimum_assurance, "uncertified_query");
        assert_eq!(evidence.stage, SubnetCatalogLoadStageV1::RefreshFailed);
        assert_eq!(evidence.registry_version, Some(881_337));
        assert_eq!(evidence.returned_registry_value_version, Some(881_336));
        assert_eq!(evidence.source_endpoint.as_deref(), Some("https://ic0.app"));
        assert_eq!(evidence.assurance.as_deref(), Some("uncertified_query"));
        assert_eq!(
            evidence.registry_records,
            [SubnetCatalogRegistryRecordEvidenceV1 {
                record_kind: SubnetCatalogRegistryRecordKindV1::RoutingTable,
                key: "canister_ranges_test".to_string(),
                subnet: None,
                canister_range_start: Some(canister_range_start.to_text()),
                requested_registry_version: 881_337,
                returned_registry_version: 881_330,
                timestamp_nanoseconds: 42,
                source_endpoint: "https://ic0.app".to_string(),
                assurance: "uncertified_query".to_string(),
                value_encoding: SubnetCatalogRegistryValueEncodingV1::Chunked,
            }]
        );
        assert_eq!(
            evidence.cache_disposition,
            SubnetCatalogFailureCacheDispositionV1::RefreshFailed {
                trigger: SubnetCatalogRefreshTriggerV1::Missing,
            }
        );
        assert_eq!(
            evidence.subject,
            Some(SubnetCatalogSubjectV1::RegistryRecord {
                record_kind: SubnetCatalogRegistryRecordKindV1::SubnetRecord,
                key: "subnet_record_test".to_string(),
                subnet: Some(subnet.to_text()),
                canister_range_start: None,
            })
        );
        assert_eq!(evidence.code, "invalid_read_policy");
        assert_eq!(evidence.category, "input");
        assert_eq!(
            evidence.retryability,
            SubnetCatalogRetryabilityV1::Unknown {
                reason: SubnetCatalogUnknownRetryReasonV1::RegistryResponse,
            }
        );
        assert!(!evidence.effects.build_started);
        assert!(!evidence.effects.workspace_mutation_started);
        assert!(!evidence.effects.ic_mutation_started);

        let json = serde_json::to_value(evidence).expect("serialize typed projection");
        assert_eq!(json["retryability"]["kind"], "unknown");
        assert_eq!(json["retryability"]["reason"], "registry_response");
        assert_eq!(json["registry_records"][0]["value_encoding"], "chunked");
    }

    fn detailed_failure_fixture() -> SubnetCatalogLoadFailure {
        let subnet = Principal::from_slice(&[9; 29]);
        let canister_range_start = Principal::from_slice(&[7; 29]);
        SubnetCatalogLoadFailure {
            request: SubnetCatalogLoadFailureRequest {
                network: "ic".to_string(),
                source: Some(CatalogSourceSelection::uncertified_query("https://ic0.app")),
                minimum_assurance: CatalogAssurance::UncertifiedQuery,
            },
            stage: SubnetCatalogLoadStage::RefreshFailed,
            registry_version: Some(881_337),
            returned_registry_value_version: Some(881_336),
            source_endpoint: Some("https://ic0.app".to_string()),
            assurance: Some(CatalogAssurance::UncertifiedQuery),
            registry_records: vec![SubnetCatalogRegistryRecordEvidence {
                record: SubnetCatalogRegistryRecordSubject {
                    kind: SubnetCatalogRegistryRecordKind::RoutingTable,
                    key: "canister_ranges_test".to_string(),
                    subnet: None,
                    canister_range_start: Some(canister_range_start),
                },
                requested_registry_version: 881_337,
                returned_registry_version: 881_330,
                timestamp_nanoseconds: 42,
                source_endpoint: "https://ic0.app".to_string(),
                assurance: CatalogAssurance::UncertifiedQuery,
                value_encoding: SubnetCatalogRegistryValueEncoding::Chunked,
            }],
            cache_disposition: SubnetCatalogFailureCacheDisposition::RefreshFailed(
                SubnetCatalogRefreshTrigger::Missing,
            ),
            subject: Some(SubnetCatalogSubject::RegistryRecord(
                SubnetCatalogRegistryRecordSubject {
                    kind: SubnetCatalogRegistryRecordKind::SubnetRecord,
                    key: "subnet_record_test".to_string(),
                    subnet: Some(subnet),
                    canister_range_start: None,
                },
            )),
            code: SubnetCatalogErrorCode::InvalidReadPolicy,
            category: SubnetCatalogErrorCategory::Input,
            retryability: SubnetCatalogRetryability::Unknown(
                SubnetCatalogUnknownRetryReason::RegistryResponse,
            ),
            source: SubnetCatalogHostError::InvalidReadPolicy {
                reason: "typed fixture".to_string(),
            },
        }
    }
}
