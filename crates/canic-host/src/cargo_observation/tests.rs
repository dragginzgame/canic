//! Real Cargo failures retain bounded diagnostics and their typed execution cause.

use super::*;
use ic_host_process::tool::ExecutionFailure;
use std::fs;

#[test]
fn metadata_failure_preserves_manifest_stderr_and_execution_cause() {
    let root = crate::test_support::temp_dir("canic-metadata-diagnostics");
    fs::create_dir_all(&root).unwrap();
    let manifest = root.join("Cargo.toml");
    fs::write(&manifest, "[package\n").unwrap();
    let error = cargo_metadata_catalog_for_manifest(
        &manifest,
        true,
        true,
        &CargoFeatureSelection::default(),
    )
    .unwrap_err();
    let diagnostic = error.downcast_ref::<MetadataObservationError>().unwrap();
    assert_eq!(diagnostic.manifest, manifest);
    let execution = diagnostic.source.execution_error().unwrap();
    std::assert_matches!(execution.failure, ExecutionFailure::ExitStatus);
    assert!(!execution.evidence.stderr.is_empty());
    let rendered = diagnostic.to_string();
    assert!(rendered.contains(manifest.to_str().unwrap()));
    assert!(rendered.contains(String::from_utf8_lossy(&execution.evidence.stderr).as_ref()));
    assert!(StdError::source(diagnostic).unwrap().is::<ToolError>());
    fs::remove_dir_all(root).unwrap();
}
