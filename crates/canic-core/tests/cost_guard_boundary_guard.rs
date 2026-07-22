// Category C - System-level artifact test (no embedded config).

use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

#[test]
fn production_cost_guard_call_sites_match_reviewed_inventory() {
    let workflow_root = source_root().join("workflow");
    let mut actual = BTreeMap::new();

    scan_rust_files(&workflow_root, &mut |path, contents| {
        if path.file_name().is_some_and(|name| name == "tests.rs")
            || path.components().any(|part| part.as_os_str() == "tests")
        {
            return;
        }
        let production = contents.split("\n#[cfg(test)]").next().unwrap_or(contents);
        let counts = (
            production.matches("CostGuardOps::reserve(").count(),
            production.matches("CostGuardOps::complete(").count(),
            production.matches("CostGuardOps::recover(").count(),
            production
                .matches("CostGuardOps::recover_after_failure(")
                .count(),
        );
        if counts != (0, 0, 0, 0) {
            actual.insert(display(path), counts);
        }
    });

    let expected = BTreeMap::from([("src/workflow/cost_guard/mod.rs".to_string(), (1, 1, 1, 0))]);

    assert_eq!(
        actual, expected,
        "only CostGuardWorkflow may couple cost-guard storage mutation to intent-expiry scheduling"
    );
}

#[test]
fn control_plane_cost_guard_mutation_uses_core_workflow_authority() {
    let control_plane_workflow = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace crates directory")
        .join("canic-control-plane/src/workflow");
    let mut violations = Vec::new();

    scan_rust_files(&control_plane_workflow, &mut |path, contents| {
        if path.file_name().is_some_and(|name| name == "tests.rs")
            || path.components().any(|part| part.as_os_str() == "tests")
        {
            return;
        }
        let production = contents.split("\n#[cfg(test)]").next().unwrap_or(contents);
        if production.contains("CostGuardOps::") {
            violations.push(display(path));
        }
    });

    assert!(
        violations.is_empty(),
        "control-plane workflows must use CostGuardWorkflow so finite reservations cannot bypass expiry scheduling: {violations:?}"
    );
}

#[test]
fn cost_guard_permit_construction_stays_private() {
    let source_root = source_root();
    let mut violations = Vec::new();

    scan_rust_files(&source_root, &mut |path, contents| {
        if path.ends_with("src/ops/cost_guard.rs") {
            return;
        }

        if contents.contains("CostGuardPermit {") {
            violations.push(format!(
                "{} constructs CostGuardPermit outside the cost-guard module",
                display(path)
            ));
        }
    });

    assert!(
        violations.is_empty(),
        "cost guard permit construction boundary changed: {violations:?}"
    );
}

#[test]
fn icp_refill_value_transfer_adapters_require_cost_guard_permit() {
    let refill_ops = source_root().join("ops/ic/icp_refill.rs");
    let contents = fs::read_to_string(&refill_ops).expect("read ICP refill ops");
    let permit_args = contents.matches("_permit: &CostGuardPermit").count();

    assert_eq!(
        permit_args, 2,
        "both ICP refill value-transfer adapters must require CostGuardPermit"
    );
}

#[test]
fn management_deployment_adapters_require_cost_guard_permit() {
    let lifecycle_ops = source_root().join("ops/ic/mgmt/lifecycle.rs");
    let lifecycle = fs::read_to_string(&lifecycle_ops).expect("read management lifecycle ops");
    let lifecycle_permit_args = lifecycle.matches("_permit: &CostGuardPermit").count();

    assert_eq!(
        lifecycle_permit_args, 3,
        "create_canister and install_code management adapters must have permit-required wrappers"
    );

    let cycles_ops = source_root().join("ops/ic/mgmt/cycles.rs");
    let cycles = fs::read_to_string(&cycles_ops).expect("read management cycles ops");
    assert!(
        cycles.contains("_permit: &CostGuardPermit"),
        "deposit_cycles value-transfer wrapper must require CostGuardPermit"
    );
}

#[test]
fn provisioning_workflow_uses_management_permit_wrappers() {
    let workflow_root = source_root().join("workflow");
    let mut violations = Vec::new();

    scan_rust_files(&workflow_root, &mut |path, contents| {
        for forbidden in [
            "MgmtOps::create_canister(",
            "MgmtOps::deposit_cycles(",
            "ModuleInstallWorkflow::install_with_payload(",
            "ModuleInstallWorkflow::install_code(",
        ] {
            if contents.contains(forbidden) {
                violations.push(format!(
                    "{} calls unpermitted management deployment helper `{forbidden}`",
                    display(path)
                ));
            }
        }
    });

    assert!(
        violations.is_empty(),
        "management deployment permit boundary changed: {violations:?}"
    );
}

fn source_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src")
}

fn scan_rust_files(root: &Path, visitor: &mut impl FnMut(&Path, &str)) {
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            scan_rust_files(&path, visitor);
            continue;
        }

        if path.extension().is_none_or(|ext| ext != "rs") {
            continue;
        }

        let Ok(contents) = fs::read_to_string(&path) else {
            continue;
        };
        visitor(&path, &contents);
    }
}

fn display(path: &Path) -> String {
    path.strip_prefix(env!("CARGO_MANIFEST_DIR"))
        .unwrap_or(path)
        .display()
        .to_string()
}
