//! Apply consumes retained authority and rejects new review selectors.

use super::*;

#[test]
fn bootstrap_requires_explicit_setup_selection_and_apply_has_no_new_inputs() {
    let review = [
        "bootstrap",
        "staging",
        "--app-config",
        "canic.toml",
        "--release-build",
        &"11".repeat(32),
        "--declarations",
        "sources.toml",
    ];
    assert!(command().try_get_matches_from(review).is_err());
    for selection in ["create", "initialize", "ready"] {
        let mut args = review.to_vec();
        args.extend(["--coordinator", selection]);
        assert!(command().try_get_matches_from(args).is_ok());
    }
    let digest = "22".repeat(32);
    assert!(
        command()
            .try_get_matches_from(["bootstrap", "staging", "--apply", &digest])
            .is_ok()
    );
    for flag in [
        "--coordinator",
        "--source",
        "--seed",
        "--declarations",
        "--app-config",
        "--release-build",
    ] {
        assert!(
            command()
                .try_get_matches_from(["bootstrap", "staging", "--apply", &digest, flag, "new"])
                .is_err()
        );
    }
}

#[test]
fn registration_recovery_binds_original_plan_and_separate_approval() {
    let plan = "31".repeat(32);
    let review = "42".repeat(32);
    assert!(
        command()
            .try_get_matches_from(["bootstrap", "staging", "--recover", &plan])
            .is_ok()
    );
    assert!(
        command()
            .try_get_matches_from([
                "bootstrap",
                "staging",
                "--recover",
                &plan,
                "--approve-recovery",
                &review
            ])
            .is_ok()
    );
    assert!(
        command()
            .try_get_matches_from(["bootstrap", "staging", "--approve-recovery", &review])
            .is_err()
    );
    for selector in ["--apply", "--release-build", "--source", "--seed"] {
        assert!(
            command()
                .try_get_matches_from([
                    "bootstrap",
                    "staging",
                    "--recover",
                    &plan,
                    selector,
                    &review
                ])
                .is_err()
        );
    }
}
