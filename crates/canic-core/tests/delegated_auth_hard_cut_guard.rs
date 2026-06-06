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
    ]
    .into()
}

fn display(path: &Path) -> String {
    path.strip_prefix(env!("CARGO_MANIFEST_DIR"))
        .unwrap_or(path)
        .display()
        .to_string()
}
