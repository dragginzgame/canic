// Category C - System-level artifact test (no embedded config).

use std::{
    fs,
    path::{Path, PathBuf},
};

#[test]
fn delegated_auth_has_no_live_token_use_replay_store() {
    let source_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut violations = Vec::new();

    scan_dir(&source_root, &mut violations);

    assert!(
        violations.is_empty(),
        "delegated-token update consumption must stay hard-cut from live runtime code: {violations:?}"
    );
}

#[test]
fn root_auth_certified_data_has_single_owner() {
    let source_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut owners = Vec::new();

    collect_certified_data_owners(&source_root, &mut owners);
    owners.sort();

    assert_eq!(
        owners,
        vec!["src/ops/auth/root_canister_sig.rs".to_string()],
        "0.65 root auth certified data must be owned only by root_canister_sig.rs"
    );
}

fn scan_dir(root: &Path, violations: &mut Vec<String>) {
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            scan_dir(&path, violations);
            continue;
        }

        if path.extension().is_none_or(|ext| ext != "rs") {
            continue;
        }

        if path.file_name().is_some_and(|name| name == "token_uses.rs") {
            violations.push(format!(
                "removed stable token-use module at {}",
                display(&path)
            ));
        }

        let Ok(contents) = fs::read_to_string(&path) else {
            continue;
        };

        for term in forbidden_terms() {
            if contents.contains(&term) {
                violations.push(format!(
                    "{} contains removed symbol `{term}`",
                    display(&path)
                ));
            }
        }
    }
}

fn collect_certified_data_owners(root: &Path, owners: &mut Vec<String>) {
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_certified_data_owners(&path, owners);
            continue;
        }

        if path.extension().is_none_or(|ext| ext != "rs") {
            continue;
        }

        let Ok(contents) = fs::read_to_string(&path) else {
            continue;
        };

        if contents.contains("certified_data_set(") || contents.contains("set_certified_data(") {
            owners.push(display(&path));
        }
    }
}

fn forbidden_terms() -> Vec<String> {
    [
        ["consume_update", "_token_once"].concat(),
        ["consume_delegated", "_token_use"].concat(),
        ["Delegated", "TokenUse"].concat(),
        ["Delegated", "TokenUseConsumeResult"].concat(),
        ["Delegated", "TokenUseRecord"].concat(),
        ["delegated", "_token_uses"].concat(),
        ["DELEGATED", "_TOKEN_USE_CAPACITY"].concat(),
        ["mod ", "token_uses"].concat(),
        ["prepare_role", "_attestation_signature"].concat(),
        ["sign_prepared_role", "_attestation"].concat(),
        ["prepare_internal_invocation", "_proof_signature"].concat(),
        ["sign_prepared_internal", "_invocation_proof"].concat(),
        ["role_attestation", "_signing_effect"].concat(),
        ["internal_invocation_proof", "_signing_effect"].concat(),
        ["reserve_auth_material", "_signing_cost_guard"].concat(),
    ]
    .into()
}

fn display(path: &Path) -> String {
    path.strip_prefix(env!("CARGO_MANIFEST_DIR"))
        .unwrap_or(path)
        .display()
        .to_string()
}
