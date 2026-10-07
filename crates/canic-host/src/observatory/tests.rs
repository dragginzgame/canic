use super::{
    ObservatoryError,
    model::{ObservatoryOptions, ObservatoryProfile},
    ops::{
        self,
        presentation::{http_response, json_bytes, public_view},
        transport::ObservatoryTransport,
    },
    policy,
    view::*,
};
use crate::registry::RegistryEntry;
use canic_core::dto::public_status::PublicMetricFamily;
use std::collections::BTreeMap;

fn profile() -> ObservatoryProfile {
    ObservatoryProfile {
        schema_version: 1,
        title: "Example <Fleet>".into(),
        role_labels: BTreeMap::from([("root".into(), "<script>private label</script>".into())]),
    }
}

fn snapshot() -> ObservatorySnapshotView {
    ObservatorySnapshotView {
        schema_version: 1,
        environment: "private-environment".into(),
        fleet: "private-fleet".into(),
        collected_at_unix_ms: 1000,
        freshness_secs: 10,
        collection_elapsed_ms: 100,
        remote_call_attempts: 3,
        authority: Observation::Observed {
            observed_at_unix_ms: 1000,
            source: ObservationSource::RetainedTerminalReview,
            value: ObservatoryAuthorityView {
                app: "private-app".into(),
                canonical_network_id: "private-network".into(),
                plan_sha256: "private-plan".into(),
                registry_revision: 5,
                admission_principals: 2,
            },
        },
        operation: Observation::Unavailable {
            observed_at_unix_ms: 1000,
            failure: ObservationFailure::AuthorityUnavailable,
        },
        roles: vec![ObservatoryRoleView {
            role: "root".into(),
            canister_id: "private-canister".into(),
            parent_canister_id: Some("private-parent".into()),
            subnet_id: Some("private-subnet".into()),
            release_identity: Some("private-release".into()),
            expected_module_sha256: Some("private-module".into()),
            overview: Observation::Observed {
                observed_at_unix_ms: 1000,
                source: ObservationSource::PublicRoleOverview,
                value: RoleOverviewView {
                    bootstrap_ready: true,
                    canic_version: "0.110.15".into(),
                    canister_version: 2,
                },
            },
            funding: Observation::Observed {
                observed_at_unix_ms: 1000,
                source: ObservationSource::ProtectedRoleStatus,
                value: RoleFundingView {
                    native_cycles: "987654321000000".into(),
                    automatic_funding_enabled: true,
                    policy_generation: 2,
                    pending_operations: 1,
                },
            },
            application_metrics: Observation::Unavailable {
                observed_at_unix_ms: 1000,
                failure: ObservationFailure::Unsupported,
            },
            costs: None,
            estate: Observation::Unavailable {
                observed_at_unix_ms: 1000,
                failure: ObservationFailure::Rejected { code: 42 },
            },
            store: Observation::Unavailable {
                observed_at_unix_ms: 1000,
                failure: ObservationFailure::Unsupported,
            },
        }],
    }
}

#[test]
fn public_projection_omits_exact_authority_and_html_escapes_profile() {
    let snapshot = snapshot();
    let profile = profile();
    let public = public_view(&snapshot, &profile, 2000).unwrap();
    let json = http_response(&snapshot, &profile, "/snapshot.json", 2000, 8192).unwrap();
    assert_eq!(
        serde_json::from_slice::<PublicObservatoryView>(&json.body).unwrap(),
        public
    );
    let html = http_response(&snapshot, &profile, "/", 2000, 16384).unwrap();
    let text = String::from_utf8(html.body).unwrap();
    assert!(text.contains("&lt;script&gt;private label&lt;/script&gt;"));
    assert!(!text.contains("<script>"));
    for secret in [
        "private-canister",
        "private-parent",
        "private-subnet",
        "private-release",
        "private-module",
        "private-plan",
        "private-app",
        "987654321000000",
    ] {
        assert!(!text.contains(secret));
        assert!(!String::from_utf8_lossy(&json.body).contains(secret));
    }
    assert_eq!(html.cache_control, "no-store");
    assert_eq!(
        http_response(&snapshot, &profile, "/authority", 2000, 64)
            .unwrap()
            .status,
        404
    );
}

#[test]
fn stale_and_future_observations_never_render_ready() {
    for now in [999, 11001] {
        let public = public_view(&snapshot(), &profile(), now).unwrap();
        assert!(!public.authority_available);
        assert!(matches!(
            public.roles[0].overview,
            Observation::Unavailable {
                failure: ObservationFailure::Stale,
                ..
            }
        ));
    }
    assert!(policy::is_fresh(1000, 11000, 10));
}

#[test]
fn rendering_accepts_exact_budgets_and_preserves_serialized_bytes() {
    let value = snapshot();
    let expected = serde_json::to_vec_pretty(&value).unwrap();
    assert_eq!(json_bytes(&value, expected.len()).unwrap(), expected);
    for maximum in [0, expected.len() - 1] {
        assert!(matches!(
            json_bytes(&value, maximum),
            Err(ObservatoryError::Bound("rendered bytes"))
        ));
    }
    let profile = profile();
    let expected = http_response(&value, &profile, "/", 2000, 16384)
        .unwrap()
        .body;
    assert_eq!(
        http_response(&value, &profile, "/", 2000, expected.len())
            .unwrap()
            .body,
        expected
    );
    assert!(matches!(
        http_response(&value, &profile, "/", 2000, expected.len() - 1),
        Err(ObservatoryError::Bound("rendered bytes"))
    ));
}

#[test]
fn rendering_and_collection_budgets_fail_explicitly() {
    assert!(matches!(
        json_bytes(&snapshot(), 16),
        Err(ObservatoryError::Bound("rendered bytes"))
    ));
    assert!(matches!(
        http_response(&snapshot(), &profile(), "/", 1000, 16),
        Err(ObservatoryError::Bound("rendered bytes"))
    ));
    let mut options = options();
    options.maximum_canisters = 0;
    assert!(matches!(
        policy::validate_options(&options),
        Err(ObservatoryError::Bound("canister selection"))
    ));
    let mut bad = profile();
    bad.title.push('\n');
    assert!(matches!(
        policy::validate_profile(&bad),
        Err(ObservatoryError::Profile)
    ));
}

fn options() -> ObservatoryOptions {
    ObservatoryOptions {
        environment: "local".into(),
        fleet: "demo".into(),
        collect_costs: false,
        maximum_canisters: 16,
        maximum_response_bytes: 8192,
        freshness_secs: 30,
        query_timeout_secs: 5,
        maximum_collection_secs: 60,
    }
}

struct NoTransport;
impl ObservatoryTransport for NoTransport {
    fn metric_samples(
        &mut self,
        _: &RegistryEntry,
        _: PublicMetricFamily,
    ) -> Result<MetricSamplesView, ObservationFailure> {
        panic!("no authority, no calls")
    }
    fn cost_window(&mut self, _: &RegistryEntry) -> Result<CostWindowView, ObservationFailure> {
        panic!("no authority, no calls")
    }
    fn overview(&mut self, _: &RegistryEntry) -> Result<RoleOverviewView, ObservationFailure> {
        panic!("no authority, no calls")
    }
    fn funding(&mut self, _: &RegistryEntry) -> Result<RoleFundingView, ObservationFailure> {
        panic!("no authority, no calls")
    }
    fn estate(&mut self, _: &RegistryEntry) -> Result<RootEstateView, ObservationFailure> {
        panic!("no authority, no calls")
    }
    fn store(&mut self, _: &RegistryEntry) -> Result<StoreInventoryView, ObservationFailure> {
        panic!("no authority, no calls")
    }
    fn attempts(&self) -> u64 {
        0
    }
}

#[test]
fn missing_authority_does_not_contact_any_role_or_claim_an_empty_healthy_fleet() {
    let snapshot = ops::collect(
        Err(ObservationFailure::AuthorityUnavailable),
        &options(),
        &mut NoTransport,
    )
    .unwrap();
    assert!(snapshot.roles.is_empty());
    assert_eq!(snapshot.remote_call_attempts, 0);
    assert!(matches!(
        snapshot.authority,
        Observation::Unavailable {
            failure: ObservationFailure::AuthorityUnavailable,
            ..
        }
    ));
}

struct PartialTransport {
    cost_calls: usize,
}

#[test]
fn cost_collection_is_bounded_partial_private_and_marks_restarts() {
    let mut transport = PartialTransport { cost_calls: 0 };
    let entry = RegistryEntry {
        pid: "healthy".into(),
        role: Some("root".into()),
        parent_pid: None,
        module_hash: None,
        protocol_binding: None,
    };
    let role = ops::collect_role(&entry, &mut transport, true);
    assert_eq!(transport.cost_calls, 4);
    let costs = role.costs.as_ref().unwrap();
    assert!(matches!(
        costs.balance,
        Observation::Observed {
            value: MetricSamplesView {
                state: MetricSampleState::Stale,
                ..
            },
            ..
        }
    ));
    assert!(matches!(
        costs.funding_and_callbacks,
        Observation::Unavailable {
            failure: ObservationFailure::TimedOut,
            ..
        }
    ));
    assert!(
        costs
            .limitations
            .contains(&CostEvidenceLimitation::SourceWindowChanged)
    );
    assert!(
        costs
            .limitations
            .contains(&CostEvidenceLimitation::TransferCoverageIncomplete)
    );
    assert!(
        costs
            .limitations
            .contains(&CostEvidenceLimitation::AggregateTimerCallbacksUnqualified)
    );
    let mut private = snapshot();
    private.roles = vec![role];
    let bytes = json_bytes(&private, 16384).unwrap();
    assert_eq!(
        serde_json::from_slice::<ObservatorySnapshotView>(&bytes).unwrap(),
        private
    );
    let public = http_response(&private, &profile(), "/snapshot.json", 2000, 16384).unwrap();
    let text = String::from_utf8(public.body).unwrap();
    for secret in [
        "cost-private-canister",
        "123456789123456789123456789",
        "timer_instructions",
    ] {
        assert!(!text.contains(secret));
    }
}
impl ObservatoryTransport for PartialTransport {
    fn metric_samples(
        &mut self,
        entry: &RegistryEntry,
        family: PublicMetricFamily,
    ) -> Result<MetricSamplesView, ObservationFailure> {
        if family == PublicMetricFamily::Application {
            return if entry.pid == "failed" {
                Err(ObservationFailure::TimedOut)
            } else {
                Ok(application_samples(&entry.pid))
            };
        }
        self.cost_calls += 1;
        if family == PublicMetricFamily::Operations {
            return Err(ObservationFailure::TimedOut);
        }
        Ok(MetricSamplesView {
            state: MetricSampleState::Stale,
            sampled_at_ns: Some(1),
            stale_after_ns: 2,
            truncated: false,
            rows: vec![MetricView {
                name: "balance".into(),
                canister_id: Some("cost-private-canister".into()),
                value: "123456789123456789123456789".into(),
                unit: "cycles".into(),
                observed_at_ns: 1,
                measurement: MetricKind::Gauge,
            }],
        })
    }
    fn cost_window(&mut self, _: &RegistryEntry) -> Result<CostWindowView, ObservationFailure> {
        self.cost_calls += 1;
        Ok(CostWindowView {
            canister_version: 9,
            heap_started_at_ns: Some(3),
        })
    }
    fn overview(&mut self, entry: &RegistryEntry) -> Result<RoleOverviewView, ObservationFailure> {
        if entry.pid == "failed" {
            Err(ObservationFailure::TimedOut)
        } else {
            Ok(RoleOverviewView {
                bootstrap_ready: true,
                canic_version: "0.110.15".into(),
                canister_version: 1,
            })
        }
    }
    fn funding(&mut self, _: &RegistryEntry) -> Result<RoleFundingView, ObservationFailure> {
        Err(ObservationFailure::Rejected { code: 42 })
    }
    fn estate(&mut self, _: &RegistryEntry) -> Result<RootEstateView, ObservationFailure> {
        Err(ObservationFailure::Unsupported)
    }
    fn store(&mut self, _: &RegistryEntry) -> Result<StoreInventoryView, ObservationFailure> {
        Err(ObservationFailure::Unsupported)
    }
    fn attempts(&self) -> u64 {
        0
    }
}

#[test]
fn role_failures_preserve_other_roles_and_independent_fields() {
    let mut transport = PartialTransport { cost_calls: 0 };
    let mut entry = RegistryEntry {
        pid: "failed".into(),
        role: Some("root".into()),
        parent_pid: None,
        module_hash: None,
        protocol_binding: None,
    };
    let failed = ops::collect_role(&entry, &mut transport, false);
    entry.pid = "healthy".into();
    let healthy = ops::collect_role(&entry, &mut transport, false);
    assert_eq!(transport.cost_calls, 0);
    assert!(healthy.costs.is_none());
    assert!(matches!(
        failed.application_metrics,
        Observation::Unavailable {
            failure: ObservationFailure::TimedOut,
            ..
        }
    ));
    assert!(matches!(
        healthy.application_metrics,
        Observation::Observed {
            value: MetricSamplesView {
                state: MetricSampleState::Fresh,
                ..
            },
            ..
        }
    ));
    assert!(matches!(
        failed.overview,
        Observation::Unavailable {
            failure: ObservationFailure::TimedOut,
            ..
        }
    ));
    assert!(matches!(
        healthy.overview,
        Observation::Observed {
            value: RoleOverviewView {
                bootstrap_ready: true,
                ..
            },
            ..
        }
    ));
    assert!(matches!(
        healthy.funding,
        Observation::Unavailable {
            failure: ObservationFailure::Rejected { code: 42 },
            ..
        }
    ));
}

#[test]
fn exhausted_collection_never_launches_a_query_or_version_probe() {
    let icp = crate::icp::IcpCli::new("/does-not-exist", Some("local".into()));
    let mut transport = ops::transport::IcpObservatoryTransport {
        icp: &icp,
        root: std::path::Path::new("/does-not-exist"),
        environment: "local",
        maximum_response_bytes: 1024,
        attempted_queries: 0,
        query_timeout: std::time::Duration::from_secs(1),
        deadline: std::time::Instant::now(),
        compatibility: None,
    };
    let entry = RegistryEntry {
        pid: candid::Principal::anonymous().to_text(),
        role: Some("root".into()),
        parent_pid: None,
        module_hash: None,
        protocol_binding: None,
    };
    assert_eq!(
        transport.overview(&entry),
        Err(ObservationFailure::TimedOut)
    );
    assert_eq!(transport.attempts(), 0);
    assert!(transport.compatibility.is_none());
}

fn application_samples(canister: &str) -> MetricSamplesView {
    MetricSamplesView {
        state: MetricSampleState::Fresh,
        sampled_at_ns: Some(900_000_000),
        stale_after_ns: 2_000_000_000,
        truncated: false,
        rows: vec![MetricView {
            name: "app.<queue>".into(),
            canister_id: Some(canister.into()),
            value: u128::MAX.to_string(),
            unit: "count".into(),
            observed_at_ns: 900_000_000,
            measurement: MetricKind::Gauge,
        }],
    }
}

fn snapshot_with_application() -> ObservatorySnapshotView {
    let mut snapshot = snapshot();
    snapshot.roles[0].application_metrics = Observation::Observed {
        observed_at_unix_ms: 1000,
        source: ObservationSource::PublicMetricCache,
        value: application_samples("private-canister"),
    };
    snapshot
}

#[test]
fn application_public_projection_keeps_exact_values_and_redacts_other_dimensions() {
    let mut snapshot = snapshot_with_application();
    let Observation::Observed { value, .. } = &mut snapshot.roles[0].application_metrics else {
        unreachable!()
    };
    let mut other = value.rows[0].clone();
    other.canister_id = Some("private-tenant".into());
    other.value = "56789".into();
    value.rows.push(other);
    let public = public_view(&snapshot, &profile(), 2000).unwrap();
    let Observation::Observed { value, .. } = &public.roles[0].application_metrics else {
        unreachable!()
    };
    assert!(value.truncated);
    assert_eq!(value.rows.len(), 1);
    assert_eq!(value.rows[0].canister_id, None);
    assert_eq!(value.rows[0].value, u128::MAX.to_string());
    assert_eq!(value.sampled_at_ns, Some(900_000_000));
    let html = http_response(&snapshot, &profile(), "/", 2000, 16384).unwrap();
    let text = String::from_utf8(html.body).unwrap();
    assert!(text.contains("app.&lt;queue&gt;"));
    assert!(text.contains("fresh; partial"));
    assert!(text.contains(&u128::MAX.to_string()));
    for absent in ["<queue>", "private-tenant", "private-canister", "56789"] {
        assert!(!text.contains(absent));
    }
    assert!(matches!(
        http_response(&snapshot, &profile(), "/", 2000, 128),
        Err(ObservatoryError::Bound("rendered bytes"))
    ));
}

#[test]
fn application_source_age_and_future_rows_cannot_be_hidden_by_a_fresh_reply() {
    for (sampled_at, row_at, now) in [
        (900_000_000, 900_000_000, 3000),
        (4_000_000_000, 4_000_000_000, 2000),
        (900_000_000, 4_000_000_000, 2000),
    ] {
        let mut snapshot = snapshot_with_application();
        let Observation::Observed { value, .. } = &mut snapshot.roles[0].application_metrics else {
            unreachable!()
        };
        value.sampled_at_ns = Some(sampled_at);
        value.rows[0].observed_at_ns = row_at;
        let public = public_view(&snapshot, &profile(), now).unwrap();
        let Observation::Observed { value, .. } = &public.roles[0].application_metrics else {
            unreachable!()
        };
        assert_eq!(value.state, MetricSampleState::Stale);
        assert_eq!(value.rows[0].value, u128::MAX.to_string());
    }
}

#[test]
fn disabled_or_missing_application_data_is_not_reported_as_zero() {
    for state in [MetricSampleState::Disabled, MetricSampleState::Unavailable] {
        let mut snapshot = snapshot_with_application();
        let Observation::Observed { value, .. } = &mut snapshot.roles[0].application_metrics else {
            unreachable!()
        };
        value.state = state;
        value.rows.clear();
        value.sampled_at_ns = None;
        let public = public_view(&snapshot, &profile(), 2000).unwrap();
        let Observation::Observed { value, .. } = &public.roles[0].application_metrics else {
            unreachable!()
        };
        assert_eq!(value.state, state);
        assert!(value.rows.is_empty());
        let html = http_response(&snapshot, &profile(), "/", 2000, 16384).unwrap();
        assert!(
            String::from_utf8(html.body)
                .unwrap()
                .contains("<td>unknown</td>")
        );
    }
}

#[test]
fn application_public_redaction_does_not_merge_distinct_metric_identities() {
    let mut snapshot = snapshot_with_application();
    let Observation::Observed { value, .. } = &mut snapshot.roles[0].application_metrics else {
        unreachable!()
    };
    let mut undimensioned = value.rows[0].clone();
    undimensioned.canister_id = None;
    undimensioned.value = "123".into();
    value.rows.push(undimensioned);
    let mut unique = value.rows[0].clone();
    unique.name = "app.unique".into();
    value.rows.push(unique);
    let public = public_view(&snapshot, &profile(), 2000).unwrap();
    let Observation::Observed { value, .. } = &public.roles[0].application_metrics else {
        unreachable!()
    };
    assert!(value.truncated);
    assert_eq!(value.rows.len(), 1);
    assert_eq!(value.rows[0].name, "app.unique");
    let Observation::Observed { value, .. } = &snapshot.roles[0].application_metrics else {
        unreachable!()
    };
    assert_eq!(value.rows.len(), 3);
    assert_eq!(value.rows[1].value, "123");
}
