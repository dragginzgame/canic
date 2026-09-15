//! Module: fleet::subnet_catalog
//!
//! Responsibility: render catalog acquisition evidence beside generated desired state.
//! Boundary: presentation cannot change reviewed authority or refresh the cache.

use canic_host::subnet_catalog::view::{
    CatalogAcquisitionProgress, CatalogAcquisitionStage, RegistryCollectionStage,
    SubnetCatalogObservation,
};
use std::fmt::Write as _;

pub(super) fn print_progress(progress: &CatalogAcquisitionProgress) {
    eprintln!("{}", render_progress(progress));
}

fn render_progress(progress: &CatalogAcquisitionProgress) -> String {
    let phase = match &progress.stage {
        CatalogAcquisitionStage::CacheLookup => "checking cached placement evidence".to_string(),
        CatalogAcquisitionStage::Collecting { endpoint } => {
            format!("collecting Registry evidence from {endpoint}")
        }
        CatalogAcquisitionStage::Collected {
            endpoint,
            registry_version,
            query_calls,
        } => format!(
            "collected {endpoint}: Registry {registry_version}, {query_calls} queries; agreement pending"
        ),
        CatalogAcquisitionStage::Complete { cache_disposition } => {
            format!("validated placement evidence ({cache_disposition})")
        }
    };
    let mut text = format!(
        "subnet catalog: {phase}; elapsed={}s, deadline={}s, completed_endpoints={}",
        progress.elapsed_seconds, progress.deadline_seconds, progress.completed_endpoints
    );
    if !progress.active_endpoints.is_empty() {
        write!(text, "; active={}", progress.active_endpoints.join(", ")).unwrap();
    }
    for endpoint in &progress.registry {
        let detail = match &endpoint.stage {
            RegistryCollectionStage::Started => "starting".to_string(),
            RegistryCollectionStage::Pinned { registry_version } => {
                format!("pinned Registry {registry_version}")
            }
            RegistryCollectionStage::History {
                registry_version,
                through_version,
                reused,
            } => format!("history {through_version}/{registry_version}; reused={reused}"),
            RegistryCollectionStage::Record { key, completed, .. } => {
                format!("record {key}; completed={completed}")
            }
            RegistryCollectionStage::Retry {
                method,
                next_attempt,
                delay_millis,
            } => format!("retry {method}; attempt={next_attempt}; backoff={delay_millis}ms"),
        };
        write!(
            text,
            "\n  {}: {detail}; queries={}",
            endpoint.endpoint, endpoint.query_calls
        )
        .unwrap();
    }
    text
}

pub(super) fn render(observation: Option<&SubnetCatalogObservation>) -> String {
    let Some(value) = observation else {
        return "subnet_catalog: not applicable (local network)\n".to_string();
    };
    let mut text = String::from("subnet_catalog:\n");
    writeln!(text, "  cache: {}", value.cache_disposition).unwrap();
    writeln!(text, "  path: {}", value.cache_path.display()).unwrap();
    writeln!(text, "  fetched_at: {}", value.fetched_at).unwrap();
    writeln!(
        text,
        "  observed_at_unix_secs: {}",
        value.observed_at_unix_secs
    )
    .unwrap();
    let age = value
        .age_seconds
        .map_or_else(|| "unknown".to_string(), |age| age.to_string());
    writeln!(
        text,
        "  age_seconds: {age}; maximum: {}",
        value.max_age_seconds
    )
    .unwrap();
    writeln!(text, "  registry_version: {}", value.registry_version).unwrap();
    writeln!(text, "  catalog_digest: {}", value.catalog_digest).unwrap();
    writeln!(text, "  assurance: {}", value.assurance).unwrap();
    writeln!(
        text,
        "  source_endpoints: {}",
        value.source_endpoints.join(", ")
    )
    .unwrap();
    text
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use canic_host::subnet_catalog::view::RegistryCollectionProgress;

    #[test]
    fn acquisition_progress_keeps_collection_distinct_from_validated_agreement() {
        let mut progress = CatalogAcquisitionProgress {
            stage: CatalogAcquisitionStage::Collecting {
                endpoint: "https://ic0.app".into(),
            },
            completed_endpoints: 0,
            active_endpoints: Vec::new(),
            registry: Vec::new(),
            elapsed_seconds: 10,
            deadline_seconds: 600,
        };
        assert!(
            render_progress(&progress)
                .contains("collecting Registry evidence from https://ic0.app")
        );
        assert!(render_progress(&progress).contains("elapsed=10s, deadline=600s"));
        progress.stage = CatalogAcquisitionStage::Collected {
            endpoint: "https://ic0.app".into(),
            registry_version: 123,
            query_calls: 158,
        };
        assert!(render_progress(&progress).contains("agreement pending"));
        progress.stage = CatalogAcquisitionStage::Complete {
            cache_disposition: "refreshed_missing".into(),
        };
        assert!(
            render_progress(&progress).contains("validated placement evidence (refreshed_missing)")
        );
    }

    #[test]
    fn registry_progress_reports_both_endpoints_and_retry_details() {
        let progress = CatalogAcquisitionProgress {
            stage: CatalogAcquisitionStage::Collecting {
                endpoint: "https://two.example".into(),
            },
            completed_endpoints: 0,
            active_endpoints: vec!["https://one.example".into(), "https://two.example".into()],
            registry: vec![
                RegistryCollectionProgress {
                    endpoint: "https://one.example".into(),
                    query_calls: 20,
                    stage: RegistryCollectionStage::History {
                        registry_version: 99,
                        through_version: 80,
                        reused: true,
                    },
                },
                RegistryCollectionProgress {
                    endpoint: "https://two.example".into(),
                    query_calls: 3,
                    stage: RegistryCollectionStage::Retry {
                        method: "get_changes_since".into(),
                        next_attempt: 2,
                        delay_millis: 250,
                    },
                },
            ],
            elapsed_seconds: 10,
            deadline_seconds: 600,
        };
        let text = render_progress(&progress);
        assert!(text.contains("active=https://one.example, https://two.example"));
        assert!(text.contains("history 80/99; reused=true; queries=20"));
        assert!(text.contains("retry get_changes_since; attempt=2; backoff=250ms; queries=3"));
    }

    #[test]
    fn catalog_report_distinguishes_local_and_agreement_evidence() {
        assert_eq!(
            render(None),
            "subnet_catalog: not applicable (local network)\n"
        );
        let value = SubnetCatalogObservation {
            cache_path: "/tmp/catalog.json".into(),
            cache_disposition: "refreshed_stale".into(),
            fetched_at: "2026-09-15T00:00:00Z".into(),
            observed_at_unix_secs: 1_789_430_410,
            age_seconds: Some(10),
            max_age_seconds: 3_600,
            registry_version: 123,
            catalog_digest: "ab".repeat(32),
            assurance: "multi_endpoint_agreement".into(),
            source_endpoints: vec!["https://one.example".into(), "https://two.example".into()],
        };
        let output = render(Some(&value));
        for expected in [
            "refreshed_stale",
            "age_seconds: 10; maximum: 3600",
            "registry_version: 123",
            "assurance: multi_endpoint_agreement",
            "https://one.example, https://two.example",
        ] {
            assert!(output.contains(expected), "missing {expected}: {output}");
        }
        assert!(output.contains(&value.catalog_digest));
    }
}
