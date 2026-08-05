//! Module: config::component_group::label
//!
//! Responsibility: define bounded canonical deployment-label primitives.
//! Does not own: inheritance, typed purpose, placement, authorization, or runtime state.
//! Boundary: source and decoded label text is validated before group compilation.

use std::{borrow::Borrow, fmt};

use candid::CandidType;
use serde::{Deserialize, Deserializer, Serialize, de};
use thiserror::Error as ThisError;

/// Maximum bytes in one canonical deployment-label key.
pub const MAX_COMPONENT_DEPLOYMENT_LABEL_KEY_BYTES: usize = 40;
/// Maximum bytes in one canonical deployment-label value.
pub const MAX_COMPONENT_DEPLOYMENT_LABEL_VALUE_BYTES: usize = 128;
/// Maximum effective deployment labels on one flattened Component occurrence.
pub const MAX_COMPONENT_DEPLOYMENT_LABELS: usize = 32;

macro_rules! bounded_component_deployment_label_text {
    ($name:ident, $kind:literal, $maximum:expr, $validate:ident) => {
        #[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            #[must_use]
            pub const fn as_str(&self) -> &str {
                self.0.as_str()
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(self.as_str())
            }
        }

        impl AsRef<str> for $name {
            fn as_ref(&self) -> &str {
                self.as_str()
            }
        }

        impl Borrow<str> for $name {
            fn borrow(&self) -> &str {
                self.as_str()
            }
        }

        impl CandidType for $name {
            fn _ty() -> candid::types::Type {
                candid::types::TypeInner::Text.into()
            }

            fn idl_serialize<S>(&self, serializer: S) -> Result<(), S::Error>
            where
                S: candid::types::Serializer,
            {
                serializer.serialize_text(self.as_str())
            }
        }

        impl TryFrom<String> for $name {
            type Error = ComponentDeploymentLabelParseError;

            fn try_from(value: String) -> Result<Self, Self::Error> {
                $validate(&value, $kind, $maximum)?;
                Ok(Self(value))
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                let value = String::deserialize(deserializer)?;
                Self::try_from(value).map_err(de::Error::custom)
            }
        }
    };
}

bounded_component_deployment_label_text!(
    ComponentDeploymentLabelKey,
    "Component deployment label key",
    MAX_COMPONENT_DEPLOYMENT_LABEL_KEY_BYTES,
    validate_label_key
);

bounded_component_deployment_label_text!(
    ComponentDeploymentLabelValue,
    "Component deployment label value",
    MAX_COMPONENT_DEPLOYMENT_LABEL_VALUE_BYTES,
    validate_label_value
);

/// One bounded inert metadata label on a flattened Component occurrence.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ComponentDeploymentLabel {
    pub key: ComponentDeploymentLabelKey,
    pub value: ComponentDeploymentLabelValue,
}

/// Typed rejection for malformed Component deployment label text.
#[derive(Clone, Debug, Eq, PartialEq, ThisError)]
pub enum ComponentDeploymentLabelParseError {
    #[error("{kind} must not be empty")]
    Empty { kind: &'static str },

    #[error("{kind} must not exceed {maximum} bytes, got {actual}")]
    TooLong {
        kind: &'static str,
        actual: usize,
        maximum: usize,
    },

    #[error("Component deployment label key must use only ASCII letters, numbers, '-' or '_'")]
    InvalidKeyCharacters,

    #[error("Component deployment label value must not contain control characters")]
    InvalidValueCharacters,
}

fn validate_label_key(
    value: &str,
    kind: &'static str,
    maximum: usize,
) -> Result<(), ComponentDeploymentLabelParseError> {
    validate_label_text_length(value, kind, maximum)?;
    if !value
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        return Err(ComponentDeploymentLabelParseError::InvalidKeyCharacters);
    }
    Ok(())
}

fn validate_label_value(
    value: &str,
    kind: &'static str,
    maximum: usize,
) -> Result<(), ComponentDeploymentLabelParseError> {
    validate_label_text_length(value, kind, maximum)?;
    if value.chars().any(char::is_control) {
        return Err(ComponentDeploymentLabelParseError::InvalidValueCharacters);
    }
    Ok(())
}

const fn validate_label_text_length(
    value: &str,
    kind: &'static str,
    maximum: usize,
) -> Result<(), ComponentDeploymentLabelParseError> {
    if value.is_empty() {
        return Err(ComponentDeploymentLabelParseError::Empty { kind });
    }
    if value.len() > maximum {
        return Err(ComponentDeploymentLabelParseError::TooLong {
            kind,
            actual: value.len(),
            maximum,
        });
    }
    Ok(())
}
