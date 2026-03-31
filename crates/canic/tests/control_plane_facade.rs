#![cfg(feature = "control-plane")]

use canic::ids::{TemplateChunkingMode, TemplateManifestState};

// Confirms the public `canic` facade exposes the full control-plane enum surface.
#[test]
fn control_plane_facade_reexports_template_manifest_enums() {
    let _ = TemplateChunkingMode::Chunked;
    let _ = TemplateManifestState::Approved;
}
