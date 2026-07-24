//! Module: infra::ic::release_build
//!
//! Responsibility: expose the release-build identity embedded at compile time.
//! Does not own: release-build planning, artifact hashing, or install admission.
//! Boundary: qualified release builds supply one canonical ID through the build script.

#![expect(
    dead_code,
    reason = "the embedded identity is staged with ID-21 admission before root lifecycle wiring"
)]

use crate::ids::{ReleaseBuildId, ReleaseBuildIdParseError};
use thiserror::Error as ThisError;

///
/// EmbeddedReleaseBuildError
///

#[derive(Debug, ThisError)]
pub enum EmbeddedReleaseBuildError {
    #[error("Wasm has no embedded release-build identity")]
    Missing,

    #[error("Wasm contains an invalid embedded release-build identity: {0}")]
    Invalid(#[from] ReleaseBuildIdParseError),
}

///
/// ReleaseBuildInfra
///

pub struct ReleaseBuildInfra;

impl ReleaseBuildInfra {
    pub fn embedded_release_build_id() -> Result<ReleaseBuildId, EmbeddedReleaseBuildError> {
        Self::release_build_id_from_env(option_env!("CANIC_RELEASE_BUILD_ID"))
    }

    fn release_build_id_from_env(
        value: Option<&str>,
    ) -> Result<ReleaseBuildId, EmbeddedReleaseBuildError> {
        value
            .ok_or(EmbeddedReleaseBuildError::Missing)?
            .parse()
            .map_err(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_release_build_identity_requires_canonical_text() {
        assert!(matches!(
            ReleaseBuildInfra::release_build_id_from_env(None),
            Err(EmbeddedReleaseBuildError::Missing)
        ));
        assert!(matches!(
            ReleaseBuildInfra::release_build_id_from_env(Some("AB")),
            Err(EmbeddedReleaseBuildError::Invalid(_))
        ));

        let text = "ab".repeat(32);
        assert_eq!(
            ReleaseBuildInfra::release_build_id_from_env(Some(&text))
                .expect("canonical release-build ID")
                .to_string(),
            text
        );
    }
}
