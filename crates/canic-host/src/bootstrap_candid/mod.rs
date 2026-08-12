//! Module: bootstrap_candid
//!
//! Responsibility: materialize canonical Candid for Canic-owned infrastructure canisters.
//! Does not own: endpoint definitions, Wasm compilation, or App-owned Candid artifacts.
//! Boundary: copies checked-in contracts for ordinary builds and extracts only on explicit refresh
//! or when a generated fallback has no canonical source contract.

use crate::{canister_build::extract_candid_bytes, durable_io::write_bytes};
use std::{fs, path::Path};

/// Materialize one infrastructure canister's Candid artifact.
pub fn materialize_infrastructure_candid(
    role: &str,
    canonical_did_path: Option<&Path>,
    artifact_did_path: &Path,
    refresh_canonical_did: bool,
    debug_wasm_path: &Path,
    build_debug_wasm: impl FnOnce() -> Result<(), Box<dyn std::error::Error>>,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(canonical_did_path) = canonical_did_path {
        if canonical_did_path.is_file() && !refresh_canonical_did {
            fs::copy(canonical_did_path, artifact_did_path)?;
            return Ok(());
        }
        if !refresh_canonical_did {
            return Err(format!(
                "canonical {role} Candid file is missing: {}",
                canonical_did_path.display()
            )
            .into());
        }
    } else if refresh_canonical_did {
        return Err(format!(
            "cannot refresh canonical {role} Candid without the canonical source package"
        )
        .into());
    }

    build_debug_wasm()?;

    let candid = extract_candid_bytes(debug_wasm_path)?;

    if let Some(canonical_did_path) = canonical_did_path {
        write_bytes(canonical_did_path, &candid)?;
    }
    write_bytes(artifact_did_path, &candid)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::temp_dir;

    #[test]
    fn ordinary_build_copies_canonical_did_without_debug_build() {
        let root = temp_dir("canic-bootstrap-candid-copy");
        fs::create_dir_all(&root).expect("create temp dir");
        let canonical = root.join("canonical.did");
        let artifact = root.join("artifact.did");
        fs::write(&canonical, "service : {}\n").expect("write canonical DID");

        materialize_infrastructure_candid(
            "test_role",
            Some(&canonical),
            &artifact,
            false,
            &root.join("missing.wasm"),
            || panic!("ordinary canonical build must not compile debug Wasm"),
        )
        .expect("copy canonical DID");

        assert_eq!(
            fs::read_to_string(artifact).expect("read artifact DID"),
            "service : {}\n"
        );
        fs::remove_dir_all(root).expect("clean temp dir");
    }

    #[test]
    fn missing_canonical_did_fails_instead_of_becoming_an_implicit_refresh() {
        let root = temp_dir("canic-bootstrap-candid-missing");
        fs::create_dir_all(&root).expect("create temp dir");
        let canonical = root.join("missing.did");
        let artifact = root.join("artifact.did");

        let error = materialize_infrastructure_candid(
            "test_role",
            Some(&canonical),
            &artifact,
            false,
            &root.join("missing.wasm"),
            || panic!("missing canonical DID must fail before a debug build"),
        )
        .expect_err("reject missing canonical DID");

        assert!(
            error
                .to_string()
                .contains("canonical test_role Candid file is missing")
        );
        fs::remove_dir_all(root).expect("clean temp dir");
    }
}
