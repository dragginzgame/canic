use super::*;

#[test]
fn unpaid_review_explains_unknown_funding_and_preserves_exact_review_identity() {
    let operation_id = "ab".repeat(32);
    let plan_sha256 = "cd".repeat(32);
    let reason = InfrastructureFundingUnavailable::RetainedInfrastructureReview {
        operation_id: operation_id.clone(),
        plan_sha256: plan_sha256.clone(),
    };
    let message = infrastructure_unavailable_message(&reason);
    assert!(message.contains(&operation_id));
    assert!(message.contains(&plan_sha256));
    assert!(message.contains("unpaid infrastructure review"));
    assert!(message.contains("preserve retained authority"));
    assert_eq!(
        serde_json::to_value(reason).unwrap(),
        serde_json::json!({
            "retained_infrastructure_review": { "operation_id": operation_id, "plan_sha256": plan_sha256 }
        })
    );
}

#[test]
fn generation_inputs_require_both_paths_and_exclude_retained_desired() {
    let base = ["readiness", "staging", "--operator", "aaaaa-aa"];
    assert!(command().try_get_matches_from(base).is_ok());
    for extra in [
        vec!["--source", "policy.toml"],
        vec!["--seed", "estate.toml"],
    ] {
        let error = command()
            .try_get_matches_from(base.into_iter().chain(extra))
            .unwrap_err();
        assert_eq!(
            error.kind(),
            clap::error::ErrorKind::MissingRequiredArgument
        );
    }
    let paths = ["--source", "policy.toml", "--seed", "estate.toml"];
    assert!(
        command()
            .try_get_matches_from(base.into_iter().chain(paths))
            .is_ok()
    );
    let error = command()
        .try_get_matches_from(
            base.into_iter()
                .chain(paths)
                .chain(["--desired", "old.toml"]),
        )
        .unwrap_err();
    assert_eq!(error.kind(), clap::error::ErrorKind::ArgumentConflict);
}
