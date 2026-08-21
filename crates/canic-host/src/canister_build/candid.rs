use std::{fs, path::Path, process::Command};

pub fn extract_candid_bytes(debug_wasm_path: &Path) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let output = Command::new("candid-extractor")
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

    let candid = String::from_utf8(output.stdout)
        .map_err(|err| format!("candid-extractor emitted non-UTF-8 output: {err}"))?;
    Ok(normalize_candid(&candid).into_bytes())
}

fn normalize_candid(candid: &str) -> String {
    let mut normalized = String::with_capacity(candid.len());
    for line in candid.lines() {
        normalized.push_str(line.trim_end());
        normalized.push('\n');
    }
    normalized
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
    use super::normalize_candid;

    #[test]
    fn extracted_candid_has_one_terminal_newline_and_no_trailing_whitespace() {
        assert_eq!(
            normalize_candid("//  \nservice : {  \n  method : () -> ();\t\n}"),
            "//\nservice : {\n  method : () -> ();\n}\n"
        );
    }
}
