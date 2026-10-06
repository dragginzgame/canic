use std::{fs, path::Path};

/// Extract the compiled declaration through the canonical tool and normalize whitespace.
pub fn extract_candid_bytes(debug_wasm_path: &Path) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    extract_candid_with_tool(debug_wasm_path, Path::new("candid-extractor"))
}

pub(super) fn extract_candid_with_tool(
    debug_wasm_path: &Path,
    extractor: &Path,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let output = crate::build_environment::command(extractor)
        .arg(debug_wasm_path)
        .output()
        .map_err(|err| {
            format!(
                "failed to run candid-extractor for {}: {err}",
                debug_wasm_path.display()
            )
        })?;

    if !output.status.success() {
        return Err(format!(
            "candid-extractor failed for {}: {}",
            debug_wasm_path.display(),
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }

    // Normalization can add one terminal newline, but never expands the
    // captured declaration beyond that. Process admission remains Canic-owned.
    let limit = output.stdout.len().saturating_add(1);
    Ok(ic_host_tools::candid::normalize(&output.stdout, limit)?.into_bytes())
}

// Remove stale ICP-generated Candid sidecars so surface scans match the exact
// selected `<role>.did` artifact.
pub(super) fn remove_stale_icp_candid_sidecars(artifact_root: &Path) -> std::io::Result<()> {
    for relative in [
        "constructor.did",
        "service.did",
        "service.did.d.ts",
        "service.did.js",
    ] {
        let path = artifact_root.join(relative);
        match fs::remove_file(path) {
            Ok(()) => {}
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
            Err(err) => return Err(err),
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use ic_host_tools::candid::normalize;

    #[test]
    fn extracted_candid_has_one_terminal_newline_and_no_trailing_whitespace() {
        assert_eq!(
            normalize(b"//  \nservice : {  \n  method : () -> ();\t\n}", 1024).unwrap(),
            "//\nservice : {\n  method : () -> ();\n}\n"
        );
    }
}
