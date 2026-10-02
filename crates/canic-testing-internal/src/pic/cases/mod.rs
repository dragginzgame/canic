//! Module: pic::cases
//!
//! Responsibility: bind registered journeys to their compiled libtest identities.
//! Boundary: discovery only; this module never starts a canister or a build.

use std::{collections::BTreeSet, process::Command};

/// One journey's label, callable function and canonical compiled test identity.
#[derive(Clone, Copy, Debug)]
pub(super) struct GovernedTestCase {
    pub(super) name: &'static str,
    pub(super) test: fn(),
    pub(super) rust_path: &'static str,
}

// Resolve the function item's canonical name, including through re-exports.
// The discovery check verifies it against this compiler's actual libtest names.
macro_rules! registered {
    ($(($name:expr, $test:expr $(,)?)),* $(,)?) => {
        vec![$($crate::pic::cases::GovernedTestCase {
            name: $name,
            test: $test,
            rust_path: std::any::type_name_of_val(&$test)
                .strip_prefix("canic_testing_internal::")
                .expect("crate-local governed function"),
        }),*]
    };
}
pub(super) use registered;

// These native modules are also compiled and executed by ordinary discovery.
const ORDINARY_NATIVE_MODULES: &[&str] = &[
    "embedded_root::tests::",
    "pic::artifacts::tests::",
    "pic::journey_policy::tests::",
    "pic::lifecycle::fast_tests::",
    "pic::progress::tests::",
    "pic::root::baseline::tests::",
    "pic::timing::tests::",
];

// Intentional opt-in proofs and runner entry points must remain explicitly
// ignored. A new ignored case needs classification, just like a new normal case.
const EXPLICIT_SELECTIONS: &[&str] = &[
    "embedded_root::tests::embedded_peer_reproduces_across_paths_and_release_versions",
    "pic::artifacts::tests::infrastructure_direct_builds_match_examples_and_cached_coordinator",
    "pic::fleet_registry::baseline::tests::completed_reset::incident_estate_reset_recovers_and_replays",
    "pic::fleet_registry::baseline::tests::frontend_handoff_public_cli_and_sdk_preserve_admission_and_local_trust",
    "pic::fleet_registry::baseline::tests::persistent_local_fleet_converges_two_roots_through_public_host",
    "pic::fleet_registry::baseline::tests::packaged_consumer::installed_package_build_deploy_recover_and_replay",
    "pic::fleet_registry::baseline::tests::pipelined_release_artifacts_match_serial_builds",
    "pic::fleet_registry::baseline::tests::release_artifacts::tests::batched_fixture_role_evidence_matches_isolated_validation",
    "pic::governed_suite::governed_internal_pocketic_suite",
    "pic::governed_suite::governed_parallel_funding_proof",
];

pub(super) fn assert_discovered_inventory(cases: &[GovernedTestCase]) {
    let discovered = discover(false);
    let ignored = discover(true);
    let expected_ignored = EXPLICIT_SELECTIONS
        .iter()
        .map(|name| (*name).to_owned())
        .collect::<BTreeSet<_>>();
    assert_eq!(ignored, expected_ignored, "classify every opt-in proof");

    let registered = cases
        .iter()
        .map(|case| case.rust_path)
        .collect::<BTreeSet<_>>();
    assert_eq!(
        registered.len(),
        cases.len(),
        "one owner per governed function"
    );
    for name in &registered {
        assert!(
            discovered.contains(*name),
            "registered case is not a compiled test: {name}"
        );
        assert!(
            !ignored.contains(*name),
            "ordinary governed case unexpectedly ignored: {name}"
        );
    }
    let selection = Command::new("bash")
        .arg(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../scripts/ci/list-internal-native-tests.sh"
        ))
        .output()
        .expect("list native test selectors");
    assert!(selection.status.success());
    let selection = String::from_utf8(selection.stdout).expect("UTF-8 native selectors");
    let selectors = selection.lines().collect::<Vec<_>>();
    assert!(!selectors.is_empty());
    assert_eq!(
        selectors.iter().collect::<BTreeSet<_>>().len(),
        selectors.len()
    );
    for selector in &selectors {
        assert!(!selector.is_empty());
        assert!(
            discovered
                .iter()
                .any(|name| name.contains(selector) && !ignored.contains(name)),
            "empty native selector: {selector}"
        );
    }
    for name in discovered.difference(&ignored) {
        let native = ORDINARY_NATIVE_MODULES
            .iter()
            .any(|module| name.starts_with(module))
            || selectors.iter().any(|selector| name.contains(selector));
        assert_ne!(
            native,
            registered.contains(name.as_str()),
            "test must have exactly one native or PocketIC owner: {name}"
        );
    }
}

fn discover(ignored: bool) -> BTreeSet<String> {
    let mut command = Command::new(std::env::current_exe().expect("compiled test executable"));
    command.arg("--list");
    if ignored {
        command.arg("--ignored");
    }
    let output = command.output().expect("list compiled test identities");
    assert!(output.status.success());
    let names = String::from_utf8(output.stdout)
        .expect("UTF-8 test identities")
        .lines()
        .filter_map(|line| line.strip_suffix(": test").map(str::to_owned))
        .collect::<BTreeSet<_>>();
    assert!(!names.is_empty());
    names
}
