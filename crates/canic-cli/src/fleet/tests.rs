use super::*;
use crate::test_support::temp_dir;

#[test]
fn fleet_commands_are_current_generation_and_lexicographically_ordered() {
    let command = fleet_command();
    let names = command
        .get_subcommands()
        .map(clap::Command::get_name)
        .collect::<Vec<_>>();
    assert_eq!(names, ["ensure", "generate"]);
}

#[test]
fn generate_defaults_to_policy_seed_and_desired_paths() {
    let release = "01".repeat(32);
    let options = GenerateOptions::parse([
        OsString::from("generate"),
        OsString::from("staging"),
        OsString::from("--app-config"),
        OsString::from("apps/demo/canic.toml"),
        OsString::from("--release-build"),
        OsString::from(release),
    ])
    .expect("parse generation");

    assert_eq!(options.source, PathBuf::from("deployments/staging.toml"));
    assert_eq!(
        options.seed,
        PathBuf::from("deployments/staging.estate.toml")
    );
    assert_eq!(options.output, PathBuf::from("fleets/staging.toml"));
    assert_eq!(options.replace, None);
}

#[test]
fn ensure_defaults_to_current_fleet_document() {
    let options = EnsureOptions::parse([OsString::from("ensure"), OsString::from("staging")])
        .expect("parse ensure");
    assert_eq!(options.desired, PathBuf::from("fleets/staging.toml"));
    assert_eq!(options.apply, None);
}

#[test]
fn ensure_requires_canonical_apply_digest() {
    let error = EnsureOptions::parse([
        OsString::from("ensure"),
        OsString::from("staging"),
        OsString::from("--apply"),
        OsString::from("not-a-digest"),
    ])
    .expect_err("reject invalid digest");
    assert!(matches!(error, FleetCommandError::Usage(_)));
}

#[test]
fn generated_desired_output_requires_exact_digest_for_replacement() {
    let root = temp_dir("canic-fleet-generated-output");
    let path = root.join("fleets/staging.toml");

    publish_generated(&path, b"first", None).expect("create generated output");
    publish_generated(&path, b"first", None).expect("repeat exact output");
    assert!(matches!(
        publish_generated(&path, b"second", None),
        Err(FleetCommandError::OutputConflict(conflict)) if conflict == path
    ));
    assert!(matches!(
        publish_generated(&path, b"second", Some(&"00".repeat(32))),
        Err(FleetCommandError::OutputDigestMismatch { path: conflict, .. }) if conflict == path
    ));
    publish_generated(&path, b"second", Some(&sha256_hex(b"first")))
        .expect("replace exact reviewed output");
    assert_eq!(fs::read(&path).expect("read replaced output"), b"second");

    let missing = root.join("fleets/missing.toml");
    assert!(matches!(
        publish_generated(&missing, b"first", Some(&sha256_hex(b"absent"))),
        Err(FleetCommandError::OutputMissingForReplacement(path)) if path == missing
    ));

    fs::remove_dir_all(root).expect("remove test directory");
}

#[test]
fn generate_replace_requires_canonical_digest() {
    let release = "01".repeat(32);
    let error = GenerateOptions::parse([
        OsString::from("generate"),
        OsString::from("staging"),
        OsString::from("--app-config"),
        OsString::from("apps/demo/canic.toml"),
        OsString::from("--release-build"),
        OsString::from(release),
        OsString::from("--replace"),
        OsString::from("not-a-digest"),
    ])
    .expect_err("reject invalid replacement digest");
    assert!(matches!(error, FleetCommandError::Usage(_)));
}
