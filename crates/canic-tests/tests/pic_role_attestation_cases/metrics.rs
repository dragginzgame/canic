use crate::pic_role_attestation_support::*;

// Verify rejected guard checks are observable through the public metrics endpoint.
#[test]
fn issuer_guard_denial_records_access_metric() {
    let setup = install_test_root_cached();
    let pic = setup.pic.pic();
    let issuer_id = setup.issuer_id;
    let labels = ["access", "issuer_guard_is_root", "auth", "caller_is_root"];

    let before = metric_count_for_labels(pic, issuer_id, MetricsKind::Security, &labels);

    let denied: Result<(), Error> = pic.update_call_as_or_panic(
        issuer_id,
        Principal::anonymous(),
        "issuer_guard_is_root",
        (),
    );
    let err = denied.expect_err("anonymous caller must fail the root guard");
    assert_eq!(err.code, ErrorCode::Unauthorized);

    let after = metric_count_for_labels(pic, issuer_id, MetricsKind::Security, &labels);
    assert_eq!(
        after,
        before.saturating_add(1),
        "expected exactly one new access-denial metric row"
    );
}

// Verify successful endpoint dispatch records endpoint perf metrics end to end.
#[test]
fn issuer_guard_success_records_perf_metric() {
    let setup = install_test_root_cached();
    let pic = setup.pic.pic();
    let root_id = setup.root_id;
    let issuer_id = setup.issuer_id;
    let labels = ["perf", "endpoint", "update", "issuer_guard_is_root"];

    let before = metric_count_for_labels(pic, issuer_id, MetricsKind::Runtime, &labels);

    let allowed: Result<(), Error> =
        pic.update_call_as_or_panic(issuer_id, root_id, "issuer_guard_is_root", ());
    allowed.expect("root caller should satisfy the root guard");

    let after = metric_count_for_labels(pic, issuer_id, MetricsKind::Runtime, &labels);
    assert_eq!(
        after,
        before.saturating_add(1),
        "expected exactly one new endpoint perf metric row"
    );

    let row = query_metric_entries(pic, issuer_id, MetricsKind::Runtime)
        .into_iter()
        .find(|entry| {
            entry.labels.len() == labels.len()
                && entry
                    .labels
                    .iter()
                    .zip(labels.iter())
                    .all(|(actual, expected)| actual == expected)
        })
        .expect("missing endpoint perf metric row");

    assert!(row.principal.is_none());
    std::assert_matches!(row.value, MetricValue::CountAndU64 { .. });
}
