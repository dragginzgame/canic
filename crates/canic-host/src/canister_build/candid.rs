use std::{fs, path::Path, process::Command};

use crate::artifact_io::write_bytes_atomically;

pub(super) fn extract_candid(
    debug_wasm_path: &Path,
    did_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
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

    write_bytes_atomically(did_path, &output.stdout)?;
    Ok(())
}

// Remove stale ICP-generated Candid sidecars so local surface scans match the
// extracted `<role>.did` artifact. Production `ic` builds skip Candid sidecars
// entirely.
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
