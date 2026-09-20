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
    fn cost_samples(
        &mut self,
        _: &RegistryEntry,
        _: PublicMetricFamily,
    ) -> Result<CostSamplesView, ObservationFailure> {
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
            value: CostSamplesView {
                state: CostSampleState::Stale,
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
    fn cost_samples(
        &mut self,
        _: &RegistryEntry,
        family: PublicMetricFamily,
    ) -> Result<CostSamplesView, ObservationFailure> {
        self.cost_calls += 1;
        if family == PublicMetricFamily::Operations {
            return Err(ObservationFailure::TimedOut);
        }
        Ok(CostSamplesView {
            state: CostSampleState::Stale,
            sampled_at_ns: Some(1),
            stale_after_ns: 2,
            truncated: false,
            rows: vec![CostMetricView {
                name: "balance".into(),
                canister_id: Some("cost-private-canister".into()),
                value: "123456789123456789123456789".into(),
                unit: "cycles".into(),
                observed_at_ns: 1,
                measurement: CostMetricKind::Gauge,
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
