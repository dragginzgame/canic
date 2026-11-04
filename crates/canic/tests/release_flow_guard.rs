//! Consumer-owned release policy: adapter isolation, failure custody and delivery.

use std::{path::Path, process::Command};

fn run_fixture(script: &str) {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("workspace root");
    let output = Command::new("bash")
        .arg(root.join(script))
        .output()
        .expect("execute release policy fixture");
    assert!(
        output.status.success(),
        "{script} failed\n{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn adapter_groups_preserve_unselected_contracts_and_release_custody() {
    run_fixture("scripts/release/test-adapter-groups.sh");
}

#[test]
fn fast_patch_requires_current_group_validation_and_rejects_behavior_changes() {
    run_fixture("scripts/ci/test-fast-patch-eligibility.sh");
}

#[test]
fn release_recipes_keep_the_complete_gate_and_fail_before_delivery() {
    run_fixture("scripts/ci/test-release-recipes.sh");
}
