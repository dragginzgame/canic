// Category C - System-level artifact test (no embedded config).

use std::{
    fs,
    path::{Path, PathBuf},
};

#[test]
fn auth_certified_data_has_expected_owners() {
    let source_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut owners = Vec::new();

    collect_certified_data_owners(&source_root, &mut owners);
    owners.sort();

    assert_eq!(
        owners,
        vec![
            "src/ops/auth/issuer_canister_sig.rs".to_string(),
            "src/ops/auth/root_canister_sig.rs".to_string(),
        ],
        "0.65 auth certified data must be owned by the root and issuer canister-signature maps"
    );
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

fn display(path: &Path) -> String {
    path.strip_prefix(env!("CARGO_MANIFEST_DIR"))
        .unwrap_or(path)
        .display()
        .to_string()
}
