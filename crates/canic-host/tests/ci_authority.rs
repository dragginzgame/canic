//! Validate CI authority from YAML records without binding shell implementation or layout.

use serde_json::Value;
use std::{collections::BTreeSet, fs, path::Path};

fn action_is_pinned(action: &str) -> bool {
    action.starts_with("./")
        || action.rsplit_once('@').is_some_and(|(_, revision)| {
            revision.len() == 40
                && revision
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        })
}

fn inspect(source: &str) -> Result<Value, Vec<String>> {
    let workflow: Value = serde_saphyr::from_str(source).unwrap();
    let mut failures = Vec::new();
    if workflow.get("permissions").is_none_or(Value::is_null) {
        failures.push("permissions".into());
    }
    let jobs = workflow["jobs"].as_object().unwrap();
    for (name, job) in jobs {
        if let Some(action) = job["uses"].as_str() {
            if !action_is_pinned(action) {
                failures.push(format!("{name}: action"));
            }
            continue;
        }
        if job["timeout-minutes"]
            .as_u64()
            .is_none_or(|timeout| timeout == 0)
        {
            failures.push(format!("{name}: timeout"));
        }
        for step in job["steps"].as_array().into_iter().flatten() {
            if let Some(action) = step["uses"].as_str() {
                if !action_is_pinned(action) {
                    failures.push(format!("{name}: action"));
                }
                if action.starts_with("actions/checkout@")
                    && !matches!(&step["with"]["persist-credentials"], Value::Bool(false))
                    && step["with"]["persist-credentials"].as_str() != Some("false")
                {
                    failures.push(format!("{name}: credentials"));
                }
            }
        }
    }
    if failures.is_empty() {
        Ok(workflow)
    } else {
        Err(failures)
    }
}

fn ancestors(workflow: &Value, job: &str, seen: &mut BTreeSet<String>) {
    let needs = &workflow["jobs"][job]["needs"];
    let dependencies = needs.as_array().map_or_else(
        || needs.as_str().into_iter().collect::<Vec<_>>(),
        |values| values.iter().filter_map(Value::as_str).collect(),
    );
    for dependency in dependencies {
        if seen.insert(dependency.into()) {
            ancestors(workflow, dependency, seen);
        }
    }
}

#[test]
fn maintained_ci_preserves_pins_credentials_and_validation_dependencies() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let mut inspected = 0;
    for entry in fs::read_dir(root.join(".github/workflows")).unwrap() {
        let path = entry.unwrap().path();
        if !path
            .extension()
            .is_some_and(|extension| extension == "yml" || extension == "yaml")
        {
            continue;
        }
        inspect(&fs::read_to_string(path).unwrap()).unwrap();
        inspected += 1;
    }
    assert!(inspected > 0);
    let workflow =
        inspect(&fs::read_to_string(root.join(".github/workflows/ci.yml")).unwrap()).unwrap();
    for job in [
        "checks",
        "macos-host",
        "msrv",
        "tests-ordinary",
        "tests-pocketic",
        "release-build",
    ] {
        assert!(workflow["jobs"].get(job).is_some());
        let mut dependencies = BTreeSet::new();
        ancestors(&workflow, job, &mut dependencies);
        assert!(dependencies.contains("preflight") && dependencies.contains("security"));
        if matches!(job, "tests-ordinary" | "tests-pocketic" | "release-build") {
            assert!(dependencies.contains("checks"));
        }
    }
}

#[test]
fn equivalent_yaml_layout_preserves_authority_and_real_changes_reject() {
    let pin = "a".repeat(40);
    let source = format!(
        "permissions: {{contents: read}}\njobs:\n  verify:\n    timeout-minutes: 10\n    steps:\n      - {{uses: 'actions/checkout@{pin}', with: {{persist-credentials: false}}}}\n"
    );
    assert!(inspect(&source).is_ok());
    for (from, to, expected) in [
        (
            "permissions: {contents: read}",
            "description: explanatory",
            "permissions",
        ),
        (
            "timeout-minutes: 10",
            "timeout-minutes: 0",
            "verify: timeout",
        ),
        (
            "persist-credentials: false",
            "persist-credentials: true",
            "verify: credentials",
        ),
        (pin.as_str(), "main", "verify: action"),
    ] {
        assert!(
            inspect(&source.replace(from, to))
                .unwrap_err()
                .contains(&expected.to_string())
        );
    }
}
