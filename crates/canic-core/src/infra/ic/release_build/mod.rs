//! Module: infra::ic::release_build
//!
//! Responsibility: expose the release-build identity embedded at compile time.
//! Does not own: release-build planning, artifact hashing, or install admission.
//! Boundary: the leaf Canister supplies its compile-time value to the runtime lifecycle adapter.

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
    pub fn embedded_release_build_id(
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
            ReleaseBuildInfra::embedded_release_build_id(None),
            Err(EmbeddedReleaseBuildError::Missing)
        ));
        assert!(matches!(
            ReleaseBuildInfra::embedded_release_build_id(Some("AB")),
            Err(EmbeddedReleaseBuildError::Invalid(_))
        ));

        let text = "ab".repeat(32);
        assert_eq!(
            ReleaseBuildInfra::embedded_release_build_id(Some(&text))
                .expect("canonical release-build ID")
                .to_string(),
            text
        );
    }
}
