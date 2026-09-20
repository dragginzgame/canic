use super::*;

fn pid(id: u8) -> String {
    candid::Principal::from_slice(&[id]).to_text()
}

fn observed<T>(value: T) -> Observation<T> {
    Observation::Observed {
        observed_at_unix_ms: 1,
        source: ObservationSource::PublicMetricCache,
        value,
    }
}

fn unavailable<T>() -> Observation<T> {
    Observation::Unavailable {
        observed_at_unix_ms: 1,
        failure: ObservationFailure::Unsupported,
    }
}

fn row(name: &str, principal: Option<String>, amount: u128, at: u64) -> CostMetricView {
    CostMetricView {
        name: name.into(),
        canister_id: principal,
        value: amount.to_string(),
        unit: "cycles".into(),
        observed_at_ns: at,
        measurement: CostMetricKind::Counter {
            window_id: 0,
            saturated: false,
        },
    }
}

fn frame(at: u64, rows: Vec<CostMetricView>) -> Observation<CostSamplesView> {
    observed(CostSamplesView {
        state: CostSampleState::Fresh,
        sampled_at_ns: Some(at),
        stale_after_ns: 500,
        truncated: false,
        rows,
    })
}

fn role(id: u8, at: u64, balance: u128, grants: u128) -> ObservatoryRoleView {
    let mut balance = row("balance", Some(pid(id)), balance, at);
    balance.measurement = CostMetricKind::Gauge;
    let mut funding = vec![row("cycles_funding.cycles_granted_total", None, grants, at)];
    if id == 1 {
        funding.push(row(
            "cycles_funding.cycles_granted_to_child",
            Some(pid(2)),
            grants,
            at,
        ));
    }
    ObservatoryRoleView {
        role: "root".into(),
        canister_id: pid(id),
        parent_canister_id: (id == 2).then(|| pid(1)),
        subnet_id: Some(pid(9)),
        release_identity: Some("release-a".into()),
        expected_module_sha256: Some("a".repeat(64)),
        overview: unavailable(),
        funding: unavailable(),
        estate: unavailable(),
        store: unavailable(),
        costs: Some(RoleCostEvidenceView {
            balance: frame(at, vec![balance]),
            funding_and_callbacks: frame(at, funding),
            timer_instructions: unavailable(),
            window: observed(CostWindowView {
                canister_version: 1,
                heap_started_at_ns: Some(50),
            }),
            limitations: vec![
                CostEvidenceLimitation::TransferCoverageIncomplete,
                CostEvidenceLimitation::TimerRegistrationResetUnobservable,
            ],
        }),
    }
}

fn snapshots() -> (ObservatorySnapshotView, ObservatorySnapshotView) {
    let snapshot = |at, balance, incoming, outgoing| ObservatorySnapshotView {
        schema_version: 1,
        environment: "local".into(),
        fleet: "demo".into(),
        collected_at_unix_ms: at,
        freshness_secs: 30,
        collection_elapsed_ms: 0,
        remote_call_attempts: 0,
        authority: Observation::Observed {
            observed_at_unix_ms: at,
            source: ObservationSource::RetainedTerminalReview,
            value: ObservatoryAuthorityView {
                app: "app".into(),
                canonical_network_id: "network".into(),
                plan_sha256: "b".repeat(64),
                registry_revision: 1,
                admission_principals: 1,
            },
        },
        operation: unavailable(),
        roles: vec![
            role(1, at, 10_000, incoming),
            role(2, at, balance, outgoing),
        ],
    };
    (snapshot(100, 1_000, 100, 20), snapshot(200, 500, 100, 520))
}

fn child(report: &ObservatoryComparisonView) -> &RoleCostComparisonView {
    report
        .roles
        .iter()
        .find(|role| role.canister_id == pid(2))
        .unwrap()
}

fn cycles(value: &CostComparisonResult<CycleMovementView>) -> &str {
    let CostComparisonResult::Available { value } = value else {
        panic!("expected available movement: {value:?}");
    };
    assert_eq!(value.elapsed_ns, value.end_ns - value.start_ns);
    &value.cycles
}

fn costs(snapshot: &mut ObservatorySnapshotView, role: usize) -> &mut RoleCostEvidenceView {
    snapshot.roles[role].costs.as_mut().unwrap()
}

fn data<T>(observation: &mut Observation<T>) -> &mut T {
    let Observation::Observed { value, .. } = observation else {
        panic!("expected fixture observation");
    };
    value
}

#[test]
fn known_outgoing_grants_explain_a_balance_drop_without_claiming_consumption() {
    let (before, after) = snapshots();
    let report = compare(&before, &after).unwrap();
    let result = child(&report);
    assert_eq!(cycles(&result.balance_change), "-500");
    assert_eq!(cycles(&result.incoming_grants), "0");
    assert_eq!(cycles(&result.outgoing_grants), "500");
    assert_eq!(cycles(&result.known_grant_adjusted_decrease), "0");
    assert!(
        result
            .limitations
            .contains(&CostEvidenceLimitation::TransferCoverageIncomplete)
    );
    assert!(
        result
            .limitations
            .contains(&CostEvidenceLimitation::UnattributedExecutionMessageStorage)
    );
    let bytes = serde_json::to_vec(&report).unwrap();
    assert_eq!(
        serde_json::from_slice::<ObservatoryComparisonView>(&bytes).unwrap(),
        report
    );
}

#[test]
fn incoming_grants_adjust_growth_and_negative_residual_stays_signed() {
    let (before, mut after) = snapshots();
    let funding = data(&mut costs(&mut after, 0).funding_and_callbacks);
    funding.rows[0].value = "600".into();
    funding.rows[1].value = "600".into();
    data(&mut costs(&mut after, 1).funding_and_callbacks).rows[0].value = "20".into();
    data(&mut costs(&mut after, 1).balance).rows[0].value = "1450".into();
    assert_eq!(
        cycles(&child(&compare(&before, &after).unwrap()).known_grant_adjusted_decrease),
        "50"
    );
    data(&mut costs(&mut after, 1).balance).rows[0].value = "1600".into();
    assert_eq!(
        cycles(&child(&compare(&before, &after).unwrap()).known_grant_adjusted_decrease),
        "-100"
    );
}

#[test]
fn malformed_or_changed_bindings_reject_the_pair() {
    let (before, after) = snapshots();
    for change in [
        |s: &mut ObservatorySnapshotView| s.fleet.push('x'),
        |s: &mut ObservatorySnapshotView| s.environment.push('x'),
        |s: &mut ObservatorySnapshotView| data(&mut s.authority).plan_sha256.push('x'),
        |s: &mut ObservatorySnapshotView| data(&mut s.authority).registry_revision += 1,
        |s: &mut ObservatorySnapshotView| s.roles[1].parent_canister_id = None,
        |s: &mut ObservatorySnapshotView| s.roles[1].expected_module_sha256 = Some("c".repeat(64)),
        |s: &mut ObservatorySnapshotView| s.roles[1].release_identity = Some("release-b".into()),
        |s: &mut ObservatorySnapshotView| {
            s.roles.pop();
        },
    ] {
        let mut changed = after.clone();
        change(&mut changed);
        assert!(matches!(
            compare(&before, &changed),
            Err(ObservatoryError::Comparison(
                CostComparisonFailure::BindingChanged
            ))
        ));
    }
    let mut duplicate = after.clone();
    duplicate.roles.push(duplicate.roles[0].clone());
    assert!(matches!(
        compare(&before, &duplicate),
        Err(ObservatoryError::Comparison(
            CostComparisonFailure::InvalidSnapshot
        ))
    ));
    let mut unsupported = after.clone();
    unsupported.schema_version = 2;
    assert!(matches!(
        compare(&before, &unsupported),
        Err(ObservatoryError::Comparison(
            CostComparisonFailure::UnsupportedSchema
        ))
    ));
    assert!(matches!(
        compare(&after, &before),
        Err(ObservatoryError::Comparison(
            CostComparisonFailure::NonAdvancingWindow
        ))
    ));
}

#[test]
fn unqualified_counters_do_not_erase_valid_balance_movement() {
    let (before, after) = snapshots();
    for (change, expected) in [
        (
            (|s: &mut CostSamplesView| s.state = CostSampleState::Stale)
                as fn(&mut CostSamplesView),
            CostComparisonFailure::StaleSample,
        ),
        (
            |s: &mut CostSamplesView| s.truncated = true,
            CostComparisonFailure::TruncatedSample,
        ),
        (
            |s: &mut CostSamplesView| s.rows.clear(),
            CostComparisonFailure::MissingMetric,
        ),
        (
            |s: &mut CostSamplesView| s.rows[0].value = "19".into(),
            CostComparisonFailure::CounterDecreased,
        ),
        (
            |s: &mut CostSamplesView| {
                s.rows[0].measurement = CostMetricKind::Counter {
                    window_id: 1,
                    saturated: false,
                }
            },
            CostComparisonFailure::CounterWindowChanged,
        ),
        (
            |s: &mut CostSamplesView| {
                s.rows[0].measurement = CostMetricKind::Counter {
                    window_id: 0,
                    saturated: true,
                }
            },
            CostComparisonFailure::SaturatedCounter,
        ),
        (
            |s: &mut CostSamplesView| s.rows[0].value = "-1".into(),
            CostComparisonFailure::InvalidMetric,
        ),
        (
            |s: &mut CostSamplesView| s.rows[0].value = "0520".into(),
            CostComparisonFailure::InvalidMetric,
        ),
        (
            |s: &mut CostSamplesView| s.rows[0].unit = "instructions".into(),
            CostComparisonFailure::InvalidMetric,
        ),
        (
            |s: &mut CostSamplesView| s.rows.push(s.rows[0].clone()),
            CostComparisonFailure::InvalidMetric,
        ),
    ] {
        let mut changed = after.clone();
        change(data(&mut costs(&mut changed, 1).funding_and_callbacks));
        let report = compare(&before, &changed).unwrap();
        assert_eq!(cycles(&child(&report).balance_change), "-500");
        assert_eq!(
            child(&report).known_grant_adjusted_decrease,
            CostComparisonResult::Unavailable { reason: expected }
        );
    }
}

#[test]
fn parent_windows_must_align_with_balances_and_child_grants() {
    let (before, mut after) = snapshots();
    let parent = data(&mut costs(&mut after, 0).funding_and_callbacks);
    parent.sampled_at_ns = Some(201);
    for row in &mut parent.rows {
        row.observed_at_ns = 201;
    }
    let report = compare(&before, &after).unwrap();
    assert_eq!(cycles(&child(&report).incoming_grants), "0");
    assert_eq!(
        child(&report).known_grant_adjusted_decrease,
        CostComparisonResult::Unavailable {
            reason: CostComparisonFailure::TimeWindowsDiffer
        }
    );
}

#[test]
fn heap_resets_reject_regrown_values_and_missing_anchors_never_become_continuity() {
    let (before, after) = snapshots();
    for (change, expected) in [
        (
            (|w: &mut CostWindowView| w.heap_started_at_ns = Some(150)) as fn(&mut CostWindowView),
            CostComparisonFailure::SourceWindowChanged,
        ),
        (
            |w: &mut CostWindowView| w.canister_version += 1,
            CostComparisonFailure::SourceWindowChanged,
        ),
        (
            |w: &mut CostWindowView| w.heap_started_at_ns = None,
            CostComparisonFailure::SourceWindowUnavailable,
        ),
    ] {
        let mut changed = after.clone();
        change(data(&mut costs(&mut changed, 1).window));
        assert_eq!(
            child(&compare(&before, &changed).unwrap()).balance_change,
            CostComparisonResult::Unavailable { reason: expected }
        );
    }
    let mut missing = after;
    missing.roles[1].costs = None;
    assert_eq!(
        child(&compare(&before, &missing).unwrap()).balance_change,
        CostComparisonResult::Unavailable {
            reason: CostComparisonFailure::MissingCosts
        }
    );
}

#[test]
fn unmatched_sample_times_and_repeated_cache_reads_are_unavailable() {
    let (before, mut after) = snapshots();
    data(&mut costs(&mut after, 1).balance).rows[0].observed_at_ns = 199;
    assert_eq!(
        child(&compare(&before, &after).unwrap()).balance_change,
        CostComparisonResult::Unavailable {
            reason: CostComparisonFailure::InvalidMetric
        }
    );
    let balance = data(&mut costs(&mut after, 1).balance);
    balance.sampled_at_ns = Some(100);
    balance.rows[0].observed_at_ns = 100;
    assert_eq!(
        child(&compare(&before, &after).unwrap()).balance_change,
        CostComparisonResult::Unavailable {
            reason: CostComparisonFailure::NonAdvancingWindow
        }
    );
}

#[test]
fn file_input_is_bounded_and_workflow_needs_no_workspace_or_transport() {
    let (before, after) = snapshots();
    let dir = std::env::temp_dir().join(format!("canic-cost-comparison-{}", std::process::id()));
    std::fs::create_dir(&dir).unwrap();
    let old = dir.join("before.json");
    let new = dir.join("after.json");
    std::fs::write(&old, serde_json::to_vec(&before).unwrap()).unwrap();
    std::fs::write(&new, serde_json::to_vec(&after).unwrap()).unwrap();
    assert_eq!(
        crate::observatory::workflow::compare_files(&old, &new, 16384).unwrap(),
        compare(&before, &after).unwrap()
    );
    assert!(matches!(
        read_snapshot(&old, 1024),
        Err(ObservatoryError::Bound("snapshot bytes"))
    ));
    std::fs::write(&new, b"{}").unwrap();
    assert!(matches!(
        read_snapshot(&new, 16384),
        Err(ObservatoryError::Json(_))
    ));
    std::fs::remove_dir_all(dir).unwrap();
}
