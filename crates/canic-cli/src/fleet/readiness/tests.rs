use super::*;

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
