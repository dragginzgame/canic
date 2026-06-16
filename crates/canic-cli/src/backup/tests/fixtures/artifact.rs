//! Module: backup::tests::fixtures::artifact
//!
//! Responsibility: write artifact files for backup tests.
//! Does not own: journal construction or backup verification implementation.
//! Boundary: filesystem artifact helper for deterministic checksums.

use canic_backup::artifacts::ArtifactChecksum;
use std::{fs, path::Path};

// Write one artifact at the layout-relative path used by test journals.
pub(in crate::backup::tests) fn write_artifact(root: &Path, bytes: &[u8]) -> ArtifactChecksum {
    let path = root.join("artifacts/root");
    fs::create_dir_all(path.parent().expect("artifact has parent")).expect("create artifacts");
    fs::write(&path, bytes).expect("write artifact");
    ArtifactChecksum::from_bytes(bytes)
}
