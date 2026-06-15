use super::*;

pub(in crate::deployment_truth::tests) fn sample_authority_evidence() -> AuthorityDryRunEvidenceV1 {
    sample_authority_evidence_from_check(sample_check(sample_plan(), sample_matching_inventory()))
}

pub(in crate::deployment_truth::tests) fn sample_authority_evidence_from_check(
    check: DeploymentCheckV1,
) -> AuthorityDryRunEvidenceV1 {
    authority_dry_run_evidence_from_check(
        &check,
        "authority-evidence-1",
        "authority-report-1",
        "authority-dry-run-1",
        "2026-05-23T00:00:01Z",
    )
    .expect("build authority evidence")
}
