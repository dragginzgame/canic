//! Import parsing keeps exact approval separate from new destructive review authority.

use super::*;

#[test]
fn apply_accepts_only_retained_authority() {
    let digest = "01".repeat(32);
    let valid = command()
        .try_get_matches_from(["import", "staging", "--apply", &digest])
        .unwrap();
    assert_eq!(string_option(&valid, "apply"), Some(digest.clone()));
    for (option, value) in [
        ("--root", "aaaaa-aa"),
        ("--canister", "aaaaa-aa"),
        ("--declarations", "source.toml"),
        ("--source", "policy.toml"),
        ("--seed", "seed.toml"),
        ("--maximum-source-debit", "1T"),
        ("--maximum-root-debit", "2T"),
        ("--maximum-root-paid-calls", "36"),
    ] {
        let failure = command()
            .try_get_matches_from(["import", "staging", "--apply", &digest, option, value])
            .unwrap_err();
        assert_eq!(failure.kind(), clap::error::ErrorKind::ArgumentConflict);
    }
}

#[test]
fn review_requires_sources_disposition_and_explicit_allowances() {
    let args = [
        "import",
        "staging",
        "--canister",
        "aaaaa-aa",
        "--declarations",
        "import.toml",
        "--maximum-source-debit",
        "1T",
        "--maximum-root-debit",
        "2T",
        "--maximum-root-paid-calls",
        "36",
    ];
    let parsed = command().try_get_matches_from(args).unwrap();
    let request = request(&parsed, "staging", "staging").unwrap();
    assert_eq!(request.maximum_source_debit_cycles, 1_000_000_000_000);
    assert_eq!(request.maximum_root_debit_cycles, 2_000_000_000_000);
    assert_eq!(request.maximum_root_paid_calls, 36);
    assert_eq!(request.canisters, [Principal::management_canister()]);
    assert_eq!(
        request.seed,
        PathBuf::from("deployments/staging.estate.toml")
    );
    for index in [2, 4, 6, 8, 10] {
        let incomplete = args
            .iter()
            .enumerate()
            .filter_map(|(i, value)| (i != index && i != index + 1).then_some(*value))
            .collect::<Vec<_>>();
        assert_eq!(
            command()
                .try_get_matches_from(incomplete)
                .unwrap_err()
                .kind(),
            clap::error::ErrorKind::MissingRequiredArgument
        );
    }
}
