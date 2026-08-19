//! Module: model::auth::application_authorization::scope
//!
//! Responsibility: validate and canonicalize application authorization scopes.
//! Does not own: grant issuance, endpoint mapping, or session policy.
//! Boundary: all layers reuse these owned and borrowed validated values.

use crate::model::auth::application_authorization::{
    MAX_APPLICATION_SCOPE_BYTES, MAX_APPLICATION_SESSION_SCOPE_BYTES,
    MAX_APPLICATION_SESSION_SCOPES, MAX_VERIFIED_APPLICATION_SCOPES,
};
use std::{
    fmt::{self, Display},
    str::FromStr,
};
use thiserror::Error;

/// A model-owned, validated application authorization scope.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ApplicationScope(String);

impl ApplicationScope {
    /// Parse one owned application scope using the current canonical grammar.
    pub fn parse(value: impl Into<String>) -> Result<Self, ApplicationScopeError> {
        let value = value.into();
        validate_application_scope(&value)?;
        Ok(Self(value))
    }

    /// Borrow this scope without losing its validation invariant.
    #[must_use]
    pub fn as_scope_ref(&self) -> ApplicationScopeRef<'_> {
        ApplicationScopeRef(&self.0)
    }

    /// Return the canonical scope text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for ApplicationScope {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl Display for ApplicationScope {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for ApplicationScope {
    type Err = ApplicationScopeError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::parse(value)
    }
}

/// A borrowed application scope whose canonical grammar has been validated.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ApplicationScopeRef<'a>(&'a str);

impl<'a> ApplicationScopeRef<'a> {
    /// Validate and borrow a dynamic application scope.
    pub fn parse(value: &'a str) -> Result<Self, ApplicationScopeError> {
        validate_application_scope(value)?;
        Ok(Self(value))
    }

    /// Validate a static application scope during constant evaluation.
    ///
    /// # Panics
    ///
    /// Panics when `value` does not satisfy the canonical application-scope grammar.
    #[must_use]
    pub const fn from_static(value: &'static str) -> Self {
        assert!(
            is_valid_application_scope(value),
            "invalid canonical application scope"
        );
        Self(value)
    }

    /// Return the canonical scope text.
    #[must_use]
    pub const fn as_str(self) -> &'a str {
        self.0
    }
}

impl AsRef<str> for ApplicationScopeRef<'_> {
    fn as_ref(&self) -> &str {
        self.0
    }
}

impl Display for ApplicationScopeRef<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.0)
    }
}

/// One sorted, unique and bounded set of canonical application scopes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CanonicalApplicationScopes(Vec<ApplicationScope>);

impl CanonicalApplicationScopes {
    /// Canonicalize scopes granted by one verified delegated-token role grant.
    pub fn for_verified_grant(
        scopes: Vec<ApplicationScope>,
    ) -> Result<Self, ApplicationScopeError> {
        canonicalize(scopes, MAX_VERIFIED_APPLICATION_SCOPES, None)
    }

    /// Canonicalize the non-empty scope subset retained by one local session.
    pub fn for_session(scopes: Vec<ApplicationScope>) -> Result<Self, ApplicationScopeError> {
        canonicalize(
            scopes,
            MAX_APPLICATION_SESSION_SCOPES,
            Some(MAX_APPLICATION_SESSION_SCOPE_BYTES),
        )
    }

    /// Return whether this set contains the validated scope.
    #[must_use]
    pub fn contains(&self, scope: ApplicationScopeRef<'_>) -> bool {
        self.0
            .binary_search_by(|candidate| candidate.as_str().cmp(scope.as_str()))
            .is_ok()
    }

    /// Return the sorted canonical scopes.
    #[must_use]
    pub fn as_slice(&self) -> &[ApplicationScope] {
        &self.0
    }
}

/// Typed canonical application-scope validation failure.
#[derive(Clone, Debug, Eq, Error, PartialEq)]
pub enum ApplicationScopeError {
    #[error("application scope aggregate is {bytes} bytes and exceeds {max} bytes")]
    AggregateTooLarge { bytes: usize, max: usize },

    #[error("application scope contains an empty namespace segment")]
    EmptySegment,

    #[error("application scope is empty")]
    Empty,

    #[error("application scope set contains duplicate '{scope}'")]
    ExactDuplicate { scope: String },

    #[error("application scope contains invalid byte at offset {offset}")]
    InvalidByte { offset: usize },

    #[error("application scope namespace segment starts with an invalid byte at offset {offset}")]
    InvalidSegmentStart { offset: usize },

    #[error("application scope set is empty")]
    SetEmpty,

    #[error("application scope set has {count} entries and exceeds {max}")]
    TooMany { count: usize, max: usize },

    #[error("application scope is {bytes} bytes and exceeds {max} bytes")]
    TooLong { bytes: usize, max: usize },
}

fn canonicalize(
    mut scopes: Vec<ApplicationScope>,
    max_count: usize,
    max_aggregate_bytes: Option<usize>,
) -> Result<CanonicalApplicationScopes, ApplicationScopeError> {
    if scopes.is_empty() {
        return Err(ApplicationScopeError::SetEmpty);
    }
    if scopes.len() > max_count {
        return Err(ApplicationScopeError::TooMany {
            count: scopes.len(),
            max: max_count,
        });
    }

    let aggregate_bytes = scopes.iter().map(|scope| scope.as_str().len()).sum();
    if let Some(max) = max_aggregate_bytes
        && aggregate_bytes > max
    {
        return Err(ApplicationScopeError::AggregateTooLarge {
            bytes: aggregate_bytes,
            max,
        });
    }

    scopes.sort_unstable();
    for pair in scopes.windows(2) {
        if pair[0] == pair[1] {
            return Err(ApplicationScopeError::ExactDuplicate {
                scope: pair[0].to_string(),
            });
        }
    }
    Ok(CanonicalApplicationScopes(scopes))
}

fn validate_application_scope(value: &str) -> Result<(), ApplicationScopeError> {
    if value.is_empty() {
        return Err(ApplicationScopeError::Empty);
    }
    if value.len() > MAX_APPLICATION_SCOPE_BYTES {
        return Err(ApplicationScopeError::TooLong {
            bytes: value.len(),
            max: MAX_APPLICATION_SCOPE_BYTES,
        });
    }

    let mut segment_start = true;
    for (offset, byte) in value.bytes().enumerate() {
        if byte == b':' {
            if segment_start {
                return Err(ApplicationScopeError::EmptySegment);
            }
            segment_start = true;
            continue;
        }
        if segment_start {
            if !is_lower_alpha_numeric(byte) {
                return Err(ApplicationScopeError::InvalidSegmentStart { offset });
            }
            segment_start = false;
        } else if !is_lower_alpha_numeric(byte) && byte != b'_' && byte != b'-' {
            return Err(ApplicationScopeError::InvalidByte { offset });
        }
    }
    if segment_start {
        return Err(ApplicationScopeError::EmptySegment);
    }
    Ok(())
}

const fn is_valid_application_scope(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.is_empty() || bytes.len() > MAX_APPLICATION_SCOPE_BYTES {
        return false;
    }

    let mut offset = 0;
    let mut segment_start = true;
    while offset < bytes.len() {
        let byte = bytes[offset];
        if byte == b':' {
            if segment_start {
                return false;
            }
            segment_start = true;
        } else if segment_start {
            if !is_lower_alpha_numeric(byte) {
                return false;
            }
            segment_start = false;
        } else if !is_lower_alpha_numeric(byte) && byte != b'_' && byte != b'-' {
            return false;
        }
        offset += 1;
    }
    !segment_start
}

const fn is_lower_alpha_numeric(byte: u8) -> bool {
    byte.is_ascii_lowercase() || byte.is_ascii_digit()
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_scope_accepts_every_grammar_edge() {
        const STATIC_SCOPE: ApplicationScopeRef<'static> =
            ApplicationScopeRef::from_static("my_app:sql_read");

        for value in ["a", "0", "my_app:sql-read", "a:b_c-d0"] {
            assert_eq!(ApplicationScope::parse(value).unwrap().as_str(), value);
            assert_eq!(ApplicationScopeRef::parse(value).unwrap().as_str(), value);
        }
        assert_eq!(STATIC_SCOPE.as_str(), "my_app:sql_read");
    }

    #[test]
    fn canonical_scope_rejects_noncanonical_grammar() {
        for value in [
            "",
            ":read",
            "app:",
            "app::read",
            "App:read",
            "app.read",
            "app:réad",
            "app: read",
            "app:_read",
            "app:-read",
        ] {
            assert!(
                ApplicationScope::parse(value).is_err(),
                "accepted {value:?}"
            );
        }
        assert!(ApplicationScope::parse("a".repeat(64)).is_ok());
        assert_eq!(
            ApplicationScope::parse("a".repeat(65)),
            Err(ApplicationScopeError::TooLong { bytes: 65, max: 64 })
        );
    }

    #[test]
    fn session_scope_set_sorts_once_and_rejects_duplicates() {
        let set = CanonicalApplicationScopes::for_session(vec![
            ApplicationScope::parse("app:write").unwrap(),
            ApplicationScope::parse("app:read").unwrap(),
        ])
        .unwrap();
        assert_eq!(
            set.as_slice()
                .iter()
                .map(ApplicationScope::as_str)
                .collect::<Vec<_>>(),
            vec!["app:read", "app:write"]
        );
        assert!(set.contains(ApplicationScopeRef::parse("app:read").unwrap()));

        let duplicate = ApplicationScope::parse("app:read").unwrap();
        assert_eq!(
            CanonicalApplicationScopes::for_session(vec![duplicate.clone(), duplicate]),
            Err(ApplicationScopeError::ExactDuplicate {
                scope: "app:read".to_string()
            })
        );
    }

    #[test]
    fn scope_set_enforces_verified_and_session_count_bounds() {
        let scopes = |count: usize| {
            (0..count)
                .map(|index| ApplicationScope::parse(format!("app:s{index}")))
                .collect::<Result<Vec<_>, _>>()
                .unwrap()
        };

        assert!(CanonicalApplicationScopes::for_verified_grant(scopes(32)).is_ok());
        assert_eq!(
            CanonicalApplicationScopes::for_verified_grant(scopes(33)),
            Err(ApplicationScopeError::TooMany { count: 33, max: 32 })
        );
        assert!(CanonicalApplicationScopes::for_session(scopes(16)).is_ok());
        assert_eq!(
            CanonicalApplicationScopes::for_session(scopes(17)),
            Err(ApplicationScopeError::TooMany { count: 17, max: 16 })
        );
    }
}
