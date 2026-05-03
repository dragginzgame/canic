use crate::pic_role_attestation_support::*;

// Verify rejected guard checks are observable through the public metrics endpoint.
#[test]
fn signer_guard_denial_records_access_metric() {
    let setup = install_test_root_cached();
    let pic = PicBorrow(setup.pic.pic());
    let signer_id = setup.signer_id;
    let labels = ["signer_guard_is_root", "auth", "caller_is_root"];

    let before = metric_count_for_labels(&pic, signer_id, MetricsKind::Access, &labels);

    let denied: Result<(), Error> = update_call_as(
        &pic,
        signer_id,
        Principal::anonymous(),
        "signer_guard_is_root",
        (),
    );
    let err = denied.expect_err("anonymous caller must fail the root guard");
    assert_eq!(err.code, ErrorCode::Unauthorized);

    let after = metric_count_for_labels(&pic, signer_id, MetricsKind::Access, &labels);
    assert_eq!(
        after,
        before.saturating_add(1),
        "expected exactly one new access-denial metric row"
    );
}

// Verify successful endpoint dispatch records endpoint perf metrics end to end.
#[test]
fn signer_guard_success_records_perf_metric() {
    let setup = install_test_root_cached();
    let pic = PicBorrow(setup.pic.pic());
    let root_id = setup.root_id;
    let signer_id = setup.signer_id;
    let labels = ["endpoint", "signer_guard_is_root"];

    let before = metric_count_for_labels(&pic, signer_id, MetricsKind::Perf, &labels);

    let allowed: Result<(), Error> =
        update_call_as(&pic, signer_id, root_id, "signer_guard_is_root", ());
    allowed.expect("root caller should satisfy the root guard");

    let after = metric_count_for_labels(&pic, signer_id, MetricsKind::Perf, &labels);
    assert_eq!(
        after,
        before.saturating_add(1),
        "expected exactly one new endpoint perf metric row"
    );

    let row = query_metric_entries(&pic, signer_id, MetricsKind::Perf)
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
    assert!(matches!(row.value, MetricValue::CountAndU64 { .. }));
}
