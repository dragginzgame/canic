//! Stable source, Cargo, and artifact provenance for build outputs.

mod artifacts;
mod cargo;
mod envelope;
mod inputs;
mod model;
mod source;

pub use envelope::{build_provenance_envelope, build_provenance_schema};
pub use model::{
    ArtifactProvenanceKindV1, ArtifactProvenanceV1, BUILD_PROVENANCE_SCHEMA_ID,
    BuildProvenanceRequest, BuildProvenanceStatusV1, BuildProvenanceV1, BuildScriptInputStateV1,
    CargoProvenanceV1, SourceDirtyPolicyV1, SourceProvenanceV1, SourceVcsV1,
};

#[cfg(test)]
use artifacts::artifact_provenance;
#[cfg(test)]
use source::source_provenance;

#[cfg(test)]
mod tests;
