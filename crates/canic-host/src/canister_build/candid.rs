//! Module: canister_build::candid
//!
//! Responsibility: bounded compiled declaration extraction and normalization.
//! Does not own: tool admission, cache identity or artifact publication.
//! Boundary: the shared process engine captures the Canic-selected extractor.

use std::{fs, path::Path, time::Duration};

use ic_host_process::tool::{OutputLimit, OutputLimits, capture_command};

const EXTRACTOR_STDERR_BYTES: usize = 64 * 1024;

const EXTRACTION_LIMITS: OutputLimits = OutputLimits {
    stdout: OutputLimit::Terminate(crate::MAX_DOCUMENT_READ_BYTES),
    stderr: OutputLimit::Terminate(EXTRACTOR_STDERR_BYTES),
    timeout: Some(Duration::from_secs(120)),
};

/// Extract the compiled declaration through the canonical tool and normalize whitespace.
pub fn extract_candid_bytes(debug_wasm_path: &Path) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    extract_candid_with_tool(debug_wasm_path, Path::new("candid-extractor"))
}

pub(super) fn extract_candid_with_tool(
    debug_wasm_path: &Path,
    extractor: &Path,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut command = crate::build_environment::command(extractor);
    command.arg(debug_wasm_path);
    let output = capture_command(&mut command, EXTRACTION_LIMITS)?.require_complete()?;

    // Normalization can add one terminal newline, but never expands the
    // admitted capture beyond that. Process admission remains Canic-owned.
    let limit = crate::MAX_DOCUMENT_READ_BYTES + 1;
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
    use super::*;
    use crate::test_support::temp_dir;
    use ic_host_process::tool::{ExecutionFailure, OutputStream, ToolError};
    use ic_host_tools::candid::normalize;

    #[test]
    fn extracted_candid_has_one_terminal_newline_and_no_trailing_whitespace() {
        assert_eq!(
            normalize(b"//  \nservice : {  \n  method : () -> ();\t\n}", 1024).unwrap(),
            "//\nservice : {\n  method : () -> ();\n}\n"
        );
    }

    #[cfg(unix)]
    fn run_extractor_script(script: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let root = temp_dir("bounded-candid-extractor");
        fs::create_dir_all(&root).unwrap();
        // The shell is the substituted extractor; avoid executing newly written fixture bytes.
        let wasm = root.join("compiled.wasm");
        let input = format!("{script}\n");
        fs::write(&wasm, &input).unwrap();
        let result = extract_candid_with_tool(&wasm, Path::new("/bin/sh"));
        assert_eq!(fs::read(&wasm).unwrap(), input.as_bytes());
        fs::remove_dir_all(root).unwrap();
        result
    }

    #[cfg(unix)]
    #[test]
    fn extractor_failure_retains_shared_status_and_diagnostics() {
        let error =
            run_extractor_script("printf 'partial declaration'; printf 'diagnostic' >&2; exit 23")
                .unwrap_err();
        let error = error.downcast::<ToolError>().unwrap();
        let failure = error.execution_error().unwrap();
        assert!(
            matches!(failure.failure, ExecutionFailure::ExitStatus),
            "{failure:?}"
        );
        assert_eq!(failure.evidence.status.unwrap().code(), Some(23));
        assert_eq!(failure.evidence.stdout, b"partial declaration");
        assert_eq!(failure.evidence.stderr, b"diagnostic");
    }

    #[cfg(unix)]
    #[test]
    fn extractor_capture_refuses_oversized_output_without_returning_partial_candid() {
        for (stream, limit, redirect) in [
            (OutputStream::Stdout, crate::MAX_DOCUMENT_READ_BYTES, ""),
            (OutputStream::Stderr, EXTRACTOR_STDERR_BYTES, ">&2"),
        ] {
            let error = run_extractor_script(&format!(
                "perl -e 'print \"x\" x $ARGV[0]' {} {redirect}",
                limit + 1
            ))
            .unwrap_err();
            let error = error.downcast::<ToolError>().unwrap();
            assert!(matches!(
                error.execution_error().unwrap().failure,
                ExecutionFailure::OutputLimit { stream: actual } if actual == stream
            ));
        }
    }

    #[cfg(unix)]
    #[test]
    fn extractor_capture_preserves_normalization_and_refuses_non_utf8() {
        assert_eq!(
            run_extractor_script("printf 'service : {}  '").unwrap(),
            b"service : {}\n"
        );
        let error = run_extractor_script("printf '\\377'").unwrap_err();
        assert!(matches!(
            error.downcast_ref::<ic_host_tools::candid::NormalizationError>(),
            Some(ic_host_tools::candid::NormalizationError::Utf8(_))
        ));
    }
}
