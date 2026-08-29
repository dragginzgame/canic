//! Module: bootstrap_candid
//!
//! Responsibility: resolve canonical Candid for Canic-owned infrastructure canisters.
//! Does not own: endpoint definitions, Wasm compilation, or App-owned Candid artifacts.
//! Boundary: copies checked-in contracts for ordinary builds and extracts only on explicit refresh
//! or when a generated fallback has no canonical source contract.

use crate::{canister_build::extract_candid_bytes, durable_io::write_bytes};
use std::{fs, path::Path};

/// Resolve one infrastructure canister's Candid bytes before artifact publication.
pub fn resolve_infrastructure_candid(
    role: &str,
    canonical_did_path: Option<&Path>,
    refresh_canonical_did: bool,
    debug_wasm_path: &Path,
    build_debug_wasm: impl FnOnce() -> Result<(), Box<dyn std::error::Error>>,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    if let Some(canonical_did_path) = canonical_did_path {
        if canonical_did_path.is_file() && !refresh_canonical_did {
            return Ok(fs::read(canonical_did_path)?);
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
    Ok(candid)
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
        fs::write(&canonical, "service : {}\n").expect("write canonical DID");

        let candid = resolve_infrastructure_candid(
            "test_role",
            Some(&canonical),
            false,
            &root.join("missing.wasm"),
            || panic!("ordinary canonical build must not compile debug Wasm"),
        )
        .expect("resolve canonical DID");

        assert_eq!(candid, b"service : {}\n");
        fs::remove_dir_all(root).expect("clean temp dir");
    }

    #[test]
    fn missing_canonical_did_fails_instead_of_becoming_an_implicit_refresh() {
        let root = temp_dir("canic-bootstrap-candid-missing");
        fs::create_dir_all(&root).expect("create temp dir");
        let canonical = root.join("missing.did");

        let error = resolve_infrastructure_candid(
            "test_role",
            Some(&canonical),
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
