use crate::{
    dto::{
        metrics::{
            AccessMetricEntry, AuthMetricEntry, AuthRolloutMetricEntry, CyclesFundingMetricEntry,
            DelegationMetricEntry, EndpointHealth, HttpMetricEntry, IccMetricEntry, MetricsKind,
            MetricsRequest, MetricsResponse, RootCapabilityMetricEntry, SystemMetricEntry,
            TimerMetricEntry,
        },
        page::{Page, PageRequest},
    },
    ops::{
        perf::PerfOps,
        runtime::metrics::{
            MetricsOps,
            auth::typed_auth_metric_records,
            mapper::{
                AccessMetricEntryMapper, AuthMetricEntryMapper, AuthRolloutMetricEntryMapper,
                CyclesFundingMetricEntryMapper, DelegationMetricEntryMapper, EndpointHealthMapper,
                HttpMetricEntryMapper, IccMetricEntryMapper, RootCapabilityMetricEntryMapper,
                SystemMetricEntryMapper, TimerMetricEntryMapper,
            },
        },
    },
    perf::PerfEntry,
    workflow::view::paginate::paginate_vec,
};

///
/// MetricsQuery
///
/// Read-only query façade over metric snapshots.
/// Responsible for mapping, sorting, and pagination only.
///

pub struct MetricsQuery;

impl MetricsQuery {
    #[must_use]
    pub fn dispatch(req: MetricsRequest) -> MetricsResponse {
        match req.kind {
            MetricsKind::System => MetricsResponse::System(Self::system_snapshot()),
            MetricsKind::Icc => MetricsResponse::Icc(Self::icc_page(req.page)),
            MetricsKind::Http => MetricsResponse::Http(Self::http_page(req.page)),
            MetricsKind::Timer => MetricsResponse::Timer(Self::timer_page(req.page)),
            MetricsKind::Access => MetricsResponse::Access(Self::access_page(req.page)),
            MetricsKind::Auth => MetricsResponse::Auth(Self::auth_page(req.page)),
            MetricsKind::AuthRollout => {
                MetricsResponse::AuthRollout(Self::auth_rollout_page(req.page))
            }
            MetricsKind::Delegation => MetricsResponse::Delegation(Self::delegation_page(req.page)),
            MetricsKind::RootCapability => {
                MetricsResponse::RootCapability(Self::root_capability_page(req.page))
            }
            MetricsKind::CyclesFunding => {
                MetricsResponse::CyclesFunding(Self::cycles_funding_page(req.page))
            }
            MetricsKind::Perf => MetricsResponse::Perf(Self::perf_page(req.page)),
            MetricsKind::EndpointHealth => MetricsResponse::EndpointHealth(
                Self::endpoint_health_page(req.page, Some(crate::protocol::CANIC_METRICS)),
            ),
        }
    }

    #[must_use]
    pub fn system_snapshot() -> Vec<SystemMetricEntry> {
        let snapshot = MetricsOps::system_snapshot();
        let mut entries = SystemMetricEntryMapper::record_to_view(snapshot.entries);

        entries.sort_by(|a, b| a.kind.cmp(&b.kind));

        entries
    }

    #[must_use]
    pub fn icc_page(page: PageRequest) -> Page<IccMetricEntry> {
        let snapshot = MetricsOps::icc_snapshot();
        let mut entries = IccMetricEntryMapper::record_to_view(snapshot.entries);

        entries.sort_by(|a, b| {
            a.target
                .as_slice()
                .cmp(b.target.as_slice())
                .then_with(|| a.method.cmp(&b.method))
        });

        paginate_vec(entries, page)
    }

    #[must_use]
    pub fn http_page(page: PageRequest) -> Page<HttpMetricEntry> {
        let snapshot = MetricsOps::http_snapshot();
        let mut entries = HttpMetricEntryMapper::record_to_view(snapshot.entries);

        entries.sort_by(|a, b| a.method.cmp(&b.method).then_with(|| a.label.cmp(&b.label)));

        paginate_vec(entries, page)
    }

    #[must_use]
    pub fn timer_page(page: PageRequest) -> Page<TimerMetricEntry> {
        let snapshot = MetricsOps::timer_snapshot();
        let mut entries = TimerMetricEntryMapper::record_to_view(snapshot.entries);

        entries.sort_by(|a, b| {
            a.mode
                .cmp(&b.mode)
                .then_with(|| a.delay_ms.cmp(&b.delay_ms))
                .then_with(|| a.label.cmp(&b.label))
        });

        paginate_vec(entries, page)
    }

    #[must_use]
    pub fn access_page(page: PageRequest) -> Page<AccessMetricEntry> {
        let snapshot = MetricsOps::access_snapshot();
        let mut entries = AccessMetricEntryMapper::record_to_view(snapshot.entries);

        entries.sort_by(|a, b| {
            a.endpoint
                .cmp(&b.endpoint)
                .then_with(|| a.kind.cmp(&b.kind))
                .then_with(|| a.predicate.cmp(&b.predicate))
        });

        paginate_vec(entries, page)
    }

    #[must_use]
    pub fn auth_page(page: PageRequest) -> Page<AuthMetricEntry> {
        let snapshot = MetricsOps::access_snapshot();
        let mut entries =
            AuthMetricEntryMapper::record_to_view(typed_auth_metric_records(snapshot.entries));

        entries.sort_by(|a, b| {
            a.endpoint
                .cmp(&b.endpoint)
                .then_with(|| a.predicate.cmp(&b.predicate))
        });

        paginate_vec(entries, page)
    }

    #[must_use]
    pub fn auth_rollout_page(page: PageRequest) -> Page<AuthRolloutMetricEntry> {
        let snapshot = MetricsOps::access_snapshot();
        let entries = AuthRolloutMetricEntryMapper::record_to_view(typed_auth_metric_records(
            snapshot.entries,
        ));

        paginate_vec(entries, page)
    }

    #[must_use]
    pub fn delegation_page(page: PageRequest) -> Page<DelegationMetricEntry> {
        let snapshot = MetricsOps::delegation_snapshot();
        let mut entries = DelegationMetricEntryMapper::record_to_view(snapshot.entries);

        entries.sort_by(|a, b| a.authority.as_slice().cmp(b.authority.as_slice()));

        paginate_vec(entries, page)
    }

    #[must_use]
    pub fn root_capability_page(page: PageRequest) -> Page<RootCapabilityMetricEntry> {
        let snapshot = MetricsOps::root_capability_snapshot();
        let mut entries = RootCapabilityMetricEntryMapper::record_to_view(snapshot.entries);

        entries.sort_by(|a, b| {
            a.capability
                .cmp(&b.capability)
                .then_with(|| a.event_type.cmp(&b.event_type))
                .then_with(|| a.outcome.cmp(&b.outcome))
                .then_with(|| a.proof_mode.cmp(&b.proof_mode))
        });

        paginate_vec(entries, page)
    }

    #[must_use]
    pub fn cycles_funding_page(page: PageRequest) -> Page<CyclesFundingMetricEntry> {
        let snapshot = MetricsOps::cycles_funding_snapshot();
        let mut entries = CyclesFundingMetricEntryMapper::record_to_view(snapshot.entries);

        entries.sort_by(|a, b| {
            a.metric
                .cmp(&b.metric)
                .then_with(|| a.child_principal.cmp(&b.child_principal))
                .then_with(|| a.reason.cmp(&b.reason))
        });

        paginate_vec(entries, page)
    }

    #[must_use]
    pub fn perf_page(page: PageRequest) -> Page<PerfEntry> {
        let snapshot = PerfOps::snapshot();
        paginate_vec(snapshot.entries, page)
    }

    #[must_use]
    pub fn endpoint_health_page(
        page: PageRequest,
        exclude_endpoint: Option<&str>,
    ) -> Page<EndpointHealth> {
        let snapshot = MetricsOps::endpoint_health_snapshot();
        let mut entries = EndpointHealthMapper::record_to_view(
            snapshot.attempts,
            snapshot.results,
            snapshot.access,
            exclude_endpoint,
        );

        entries.sort_by(|a, b| a.endpoint.cmp(&b.endpoint));

        paginate_vec(entries, page)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        dto::metrics::AuthRolloutMetricClass,
        ids::AccessMetricKind,
        ops::runtime::metrics::{
            access::AccessMetrics,
            auth::{
                AuthMetricPredicate, AuthProofCacheUtilizationBucket,
                DelegationInstallValidationFailureReason, DelegationProvisionRole,
                VerifierProofCacheEvictionClass,
            },
        },
    };

    #[test]
    fn auth_page_filters_non_auth_metrics() {
        AccessMetrics::reset();

        AccessMetrics::increment(
            "auth_verifier",
            AccessMetricKind::Auth,
            AuthMetricPredicate::ProofMiss.as_str().as_ref(),
        );
        AccessMetrics::increment(
            "auth_verifier",
            AccessMetricKind::Auth,
            AuthMetricPredicate::ProofMiss.as_str().as_ref(),
        );
        AccessMetrics::increment(
            "canic_metrics",
            AccessMetricKind::Guard,
            "caller_is_controller",
        );

        let page = MetricsQuery::auth_page(PageRequest {
            offset: 0,
            limit: 10,
        });

        assert_eq!(page.entries.len(), 1);
        assert_eq!(page.entries[0].endpoint, "auth_verifier");
        assert_eq!(
            page.entries[0].predicate,
            AuthMetricPredicate::ProofMiss.as_str().as_ref()
        );
        assert_eq!(page.entries[0].count, 2);
    }

    #[test]
    fn auth_rollout_page_groups_gate_and_operational_signals() {
        AccessMetrics::reset();

        AccessMetrics::increment(
            "auth_verifier",
            AccessMetricKind::Auth,
            AuthMetricPredicate::ProofMiss.as_str().as_ref(),
        );
        AccessMetrics::increment(
            "auth_verifier",
            AccessMetricKind::Auth,
            AuthMetricPredicate::ProofMismatch.as_str().as_ref(),
        );
        AccessMetrics::increment(
            "auth_verifier",
            AccessMetricKind::Auth,
            AuthMetricPredicate::ProofCacheEviction {
                class: VerifierProofCacheEvictionClass::Active,
            }
            .as_str()
            .as_ref(),
        );
        AccessMetrics::increment(
            "auth_verifier",
            AccessMetricKind::Auth,
            AuthMetricPredicate::ProofCacheUtilization {
                bucket: AuthProofCacheUtilizationBucket::NinetyFiveToOneHundred,
            }
            .as_str()
            .as_ref(),
        );
        AccessMetrics::increment(
            "auth_signer",
            AccessMetricKind::Auth,
            AuthMetricPredicate::DelegationPushFailed {
                role: DelegationProvisionRole::Verifier,
                intent: crate::dto::auth::DelegationProofInstallIntent::Repair,
            }
            .as_str()
            .as_ref(),
        );
        AccessMetrics::increment(
            "auth_signer",
            AccessMetricKind::Auth,
            AuthMetricPredicate::DelegationInstallValidationFailed {
                intent: crate::dto::auth::DelegationProofInstallIntent::Prewarm,
                reason: DelegationInstallValidationFailureReason::VerifyProof,
            }
            .as_str()
            .as_ref(),
        );
        AccessMetrics::increment(
            "auth_verifier",
            AccessMetricKind::Auth,
            AuthMetricPredicate::ProofCacheEviction {
                class: VerifierProofCacheEvictionClass::Cold,
            }
            .as_str()
            .as_ref(),
        );
        AccessMetrics::increment(
            "canic_metrics",
            AccessMetricKind::Guard,
            "caller_is_controller",
        );

        let page = MetricsQuery::auth_rollout_page(PageRequest {
            offset: 0,
            limit: 20,
        });

        assert_eq!(page.entries.len(), 7);
        assert_eq!(
            rollout_entry(&page, "proof_miss"),
            Some((AuthRolloutMetricClass::HardGate, 1))
        );
        assert_eq!(
            rollout_entry(&page, "proof_mismatch"),
            Some((AuthRolloutMetricClass::HardGate, 1))
        );
        assert_eq!(
            rollout_entry(&page, "active_proof_eviction"),
            Some((AuthRolloutMetricClass::HardGate, 1))
        );
        assert_eq!(
            rollout_entry(&page, "repair_failure"),
            Some((AuthRolloutMetricClass::HardGate, 1))
        );
        assert_eq!(
            rollout_entry(&page, "cache_saturation"),
            Some((AuthRolloutMetricClass::HardGate, 1))
        );
        assert_eq!(
            rollout_entry(&page, "cold_proof_eviction"),
            Some((AuthRolloutMetricClass::Operational, 1))
        );
        assert_eq!(
            rollout_entry(&page, "prewarm_failure"),
            Some((AuthRolloutMetricClass::Operational, 1))
        );
    }

    fn rollout_entry(
        page: &Page<AuthRolloutMetricEntry>,
        signal: &str,
    ) -> Option<(AuthRolloutMetricClass, u64)> {
        page.entries
            .iter()
            .find_map(|entry| (entry.signal == signal).then_some((entry.class, entry.count)))
    }
}
