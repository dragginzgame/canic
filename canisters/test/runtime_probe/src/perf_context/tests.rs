//! Qualify the probe's bounded integer report against production aggregate state.

use super::*;

#[test]
fn projects_exact_samples_and_omits_totals_outside_probe_width() {
    for (label, value) in [
        ("zero", 0),
        ("cap", u64::MAX),
        ("wide", u64::MAX),
        ("wide", 1),
    ] {
        perf::record_checkpoint("runtime_probe::perf_context", label, value);
    }
    perf::record_checkpoint("other_scope", "unrelated", 1);

    let rows = perf_context_metrics();
    assert_eq!(rows.len(), 2);
    let zero = rows.iter().find(|row| row.name == "zero").expect("zero");
    assert_eq!((zero.count, zero.instructions), (1, 0));
    let cap = rows
        .iter()
        .find(|row| row.name == "cap")
        .expect("exact cap");
    assert_eq!((cap.count, cap.instructions), (1, u64::MAX));
}
