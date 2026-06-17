//! Module: manifest::validation
//!
//! Responsibility: validate backup manifest contracts before persistence or restore.
//! Does not own: manifest data definitions, summary rendering, or storage IO.
//! Boundary: exposes manifest validation methods without changing serialized shapes.

mod binding;
mod scalar;
mod sections;

use crate::manifest::{DeploymentBackupManifest, ManifestValidationError};

use binding::{validate_consistency_against_deployment, validate_verification_against_deployment};
use scalar::{validate_manifest_version, validate_nonempty};

const SUPPORTED_MANIFEST_VERSION: u16 = 1;
const SHA256_ALGORITHM: &str = "sha256";

impl DeploymentBackupManifest {
    /// Validate the manifest-level contract before backup finalization or restore planning.
    pub fn validate(&self) -> Result<(), ManifestValidationError> {
        validate_manifest_version(self.manifest_version)?;
        validate_nonempty("backup_id", &self.backup_id)?;
        validate_nonempty("created_at", &self.created_at)?;
        self.tool.validate()?;
        self.source.validate()?;
        self.consistency.validate()?;
        self.deployment.validate()?;
        self.verification.validate()?;
        validate_consistency_against_deployment(&self.consistency, &self.deployment)?;
        validate_verification_against_deployment(&self.verification, &self.deployment)?;
        Ok(())
    }
}
