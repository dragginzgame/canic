//! Module: ids::component_deployment
//!
//! Responsibility: identify declared Component composition and concrete group placements.
//! Does not own: group compilation, placement policy, service authority, or runtime parentage.
//! Boundary: source identities and member paths are bounded canonical values at decode time.

use crate::impl_storable_bounded;
use candid::CandidType;
use serde::{Deserialize, Deserializer, Serialize, de};
use std::{borrow::Borrow, fmt, str::FromStr};
use thiserror::Error as ThisError;

const COMPONENT_DEPLOYMENT_NAME_MAX_BYTES: usize = 40;
/// Maximum member segments in one canonical flattened Component Group path.
pub const COMPONENT_GROUP_MEMBER_PATH_MAX_SEGMENTS: usize = 16;
const COMPONENT_GROUP_MEMBER_PATH_MAX_BYTES: usize =
    8 + COMPONENT_GROUP_MEMBER_PATH_MAX_SEGMENTS * (8 + COMPONENT_DEPLOYMENT_NAME_MAX_BYTES);

macro_rules! bounded_deployment_name {
    ($(#[$meta:meta])* $name:ident, $kind:literal) => {
        $(#[$meta])*
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

        impl FromStr for $name {
            type Err = ComponentDeploymentIdParseError;

            fn from_str(value: &str) -> Result<Self, Self::Err> {
                validate_component_deployment_name(value, $kind)?;
                Ok(Self(value.to_string()))
            }
        }

        impl TryFrom<String> for $name {
            type Error = ComponentDeploymentIdParseError;

            fn try_from(value: String) -> Result<Self, Self::Error> {
                validate_component_deployment_name(&value, $kind)?;
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

        impl_storable_bounded!($name, 64, false);
    };
}

bounded_deployment_name!(
    /// App-scoped identity of one reusable Component Group declaration.
    ComponentGroupSpecId,
    "Component Group Spec ID"
);

bounded_deployment_name!(
    /// App-scoped identity of one independently scalable Component Group deployment.
    ComponentGroupDeploymentId,
    "Component Group deployment ID"
);

bounded_deployment_name!(
    /// Member name unique within one Component Group declaration.
    ComponentGroupMemberId,
    "Component Group member ID"
);

bounded_deployment_name!(
    /// App-scoped identity of one declared Fleet service endpoint set.
    FleetServiceId,
    "Fleet Service ID"
);

/// SHA-256 identity of one canonical Component deployment configuration.
#[derive(
    CandidType, Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize,
)]
#[serde(transparent)]
pub struct ComponentDeploymentConfigurationDigest([u8; 32]);

impl ComponentDeploymentConfigurationDigest {
    #[must_use]
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    #[must_use]
    pub const fn into_bytes(self) -> [u8; 32] {
        self.0
    }
}

impl fmt::Display for ComponentDeploymentConfigurationDigest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in self.0 {
            write!(formatter, "{byte:02x}")?;
        }
        Ok(())
    }
}

impl_storable_bounded!(ComponentDeploymentConfigurationDigest, 128, false);

/// Durable Fleet-scoped identity of one materialized Component Group deployment copy.
#[derive(
    CandidType, Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize,
)]
#[serde(deny_unknown_fields)]
pub struct ComponentGroupPlacementId {
    pub deployment: ComponentGroupDeploymentId,
    pub ordinal: u32,
}

impl_storable_bounded!(ComponentGroupPlacementId, 128, false);

/// Canonical inclusion path identifying one flattened Component occurrence.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct ComponentGroupMemberPath(Vec<ComponentGroupMemberId>);

impl ComponentGroupMemberPath {
    #[must_use]
    pub fn as_slice(&self) -> &[ComponentGroupMemberId] {
        &self.0
    }

    #[must_use]
    pub const fn len(&self) -> usize {
        self.0.len()
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl TryFrom<Vec<ComponentGroupMemberId>> for ComponentGroupMemberPath {
    type Error = ComponentGroupMemberPathError;

    fn try_from(value: Vec<ComponentGroupMemberId>) -> Result<Self, Self::Error> {
        validate_component_group_member_path(&value)?;
        Ok(Self(value))
    }
}

impl CandidType for ComponentGroupMemberPath {
    fn _ty() -> candid::types::Type {
        <Vec<ComponentGroupMemberId> as CandidType>::_ty()
    }

    fn idl_serialize<S>(&self, serializer: S) -> Result<(), S::Error>
    where
        S: candid::types::Serializer,
    {
        self.0.idl_serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for ComponentGroupMemberPath {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = Vec::<ComponentGroupMemberId>::deserialize(deserializer)?;
        Self::try_from(value).map_err(de::Error::custom)
    }
}

impl_storable_bounded!(ComponentGroupMemberPath, 1_024, false);

/// Typed rejection for an invalid Component deployment identifier.
#[derive(Clone, Debug, Eq, PartialEq, ThisError)]
pub enum ComponentDeploymentIdParseError {
    #[error("{kind} must not be empty")]
    Empty { kind: &'static str },

    #[error("{kind} must not exceed {max_bytes} bytes, got {actual_bytes}")]
    TooLong {
        kind: &'static str,
        max_bytes: usize,
        actual_bytes: usize,
    },

    #[error("{kind} must use only ASCII letters, numbers, '-' or '_'")]
    InvalidCharacters { kind: &'static str },
}

/// Typed rejection for an invalid flattened Component Group member path.
#[derive(Clone, Debug, Eq, PartialEq, ThisError)]
pub enum ComponentGroupMemberPathError {
    #[error("Component Group member path must not be empty")]
    Empty,

    #[error("Component Group member path must not exceed {max} segments, got {actual}")]
    TooDeep { max: usize, actual: usize },

    #[error("Component Group member path must not exceed {max} canonical bytes, got {actual}")]
    TooLong { max: usize, actual: usize },
}

fn validate_component_deployment_name(
    value: &str,
    kind: &'static str,
) -> Result<(), ComponentDeploymentIdParseError> {
    if value.is_empty() {
        return Err(ComponentDeploymentIdParseError::Empty { kind });
    }
    if value.len() > COMPONENT_DEPLOYMENT_NAME_MAX_BYTES {
        return Err(ComponentDeploymentIdParseError::TooLong {
            kind,
            max_bytes: COMPONENT_DEPLOYMENT_NAME_MAX_BYTES,
            actual_bytes: value.len(),
        });
    }
    if !value
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        return Err(ComponentDeploymentIdParseError::InvalidCharacters { kind });
    }
    Ok(())
}

fn validate_component_group_member_path(
    value: &[ComponentGroupMemberId],
) -> Result<(), ComponentGroupMemberPathError> {
    if value.is_empty() {
        return Err(ComponentGroupMemberPathError::Empty);
    }
    if value.len() > COMPONENT_GROUP_MEMBER_PATH_MAX_SEGMENTS {
        return Err(ComponentGroupMemberPathError::TooDeep {
            max: COMPONENT_GROUP_MEMBER_PATH_MAX_SEGMENTS,
            actual: value.len(),
        });
    }
    let encoded_bytes = value
        .iter()
        .fold(8_usize, |bytes, member| bytes + 8 + member.as_str().len());
    if encoded_bytes > COMPONENT_GROUP_MEMBER_PATH_MAX_BYTES {
        return Err(ComponentGroupMemberPathError::TooLong {
            max: COMPONENT_GROUP_MEMBER_PATH_MAX_BYTES,
            actual: encoded_bytes,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cdk::structures::storable::Storable;

    #[test]
    fn deployment_names_are_bounded_canonical_identifiers() {
        let group = "project_data-cell"
            .parse::<ComponentGroupSpecId>()
            .expect("Component Group Spec ID");
        let deployment = "project_data_cells"
            .parse::<ComponentGroupDeploymentId>()
            .expect("Component Group deployment ID");
        let member = "project_hub"
            .parse::<ComponentGroupMemberId>()
            .expect("Component Group member ID");
        let service = "project-hubs"
            .parse::<FleetServiceId>()
            .expect("Fleet Service ID");

        assert_eq!(group.as_str(), "project_data-cell");
        assert_eq!(deployment.as_str(), "project_data_cells");
        assert_eq!(member.as_str(), "project_hub");
        assert_eq!(service.as_str(), "project-hubs");
        assert!("".parse::<ComponentGroupSpecId>().is_err());
        assert!("bad/name".parse::<ComponentGroupDeploymentId>().is_err());
        assert!("service.name".parse::<FleetServiceId>().is_err());
        assert!(
            "a".repeat(COMPONENT_DEPLOYMENT_NAME_MAX_BYTES + 1)
                .parse::<ComponentGroupMemberId>()
                .is_err()
        );
    }

    #[test]
    fn deployment_names_validate_serde_and_candid_input() {
        let invalid_candid = candid::encode_one("bad/name").expect("encode invalid service ID");
        let invalid_cbor = {
            let mut bytes = Vec::new();
            ciborium::ser::into_writer("bad/name", &mut bytes).expect("encode invalid service ID");
            bytes
        };

        assert!(candid::decode_one::<FleetServiceId>(&invalid_candid).is_err());
        assert!(ciborium::de::from_reader::<FleetServiceId, _>(invalid_cbor.as_slice()).is_err());
    }

    #[test]
    fn configuration_digest_preserves_exact_bytes_and_hex_boundary() {
        let digest = ComponentDeploymentConfigurationDigest::from_bytes([0xab; 32]);
        let encoded = candid::encode_one(digest).expect("encode configuration digest");
        let decoded: ComponentDeploymentConfigurationDigest =
            candid::decode_one(&encoded).expect("decode configuration digest");

        assert_eq!(decoded, digest);
        assert_eq!(digest.as_bytes(), &[0xab; 32]);
        assert_eq!(digest.to_string(), "ab".repeat(32));
        assert!(digest.to_bytes().len() <= 128);
    }

    #[test]
    fn placement_identity_binds_deployment_and_ordinal() {
        let placement = ComponentGroupPlacementId {
            deployment: "project_data_cells"
                .parse()
                .expect("Component Group deployment ID"),
            ordinal: 7,
        };
        let candid = candid::encode_one(&placement).expect("encode placement identity");
        let decoded: ComponentGroupPlacementId =
            candid::decode_one(&candid).expect("decode placement identity");

        assert_eq!(decoded, placement);
        assert!(placement.to_bytes().len() <= 128);
    }

    #[test]
    fn member_paths_preserve_occurrence_order_and_reject_invalid_depth() {
        let path = ComponentGroupMemberPath::try_from(vec![
            "databases".parse().expect("group member"),
            "database_a".parse().expect("Component member"),
        ])
        .expect("member path");
        let candid = candid::encode_one(&path).expect("encode member path");
        let decoded: ComponentGroupMemberPath =
            candid::decode_one(&candid).expect("decode member path");

        assert_eq!(decoded, path);
        assert_eq!(path.len(), 2);
        assert_eq!(path.as_slice()[0].as_str(), "databases");
        assert_eq!(path.as_slice()[1].as_str(), "database_a");
        assert!(ComponentGroupMemberPath::try_from(Vec::new()).is_err());
        assert!(
            ComponentGroupMemberPath::try_from(
                (0..=COMPONENT_GROUP_MEMBER_PATH_MAX_SEGMENTS)
                    .map(|index| format!("member_{index}").parse().expect("group member"))
                    .collect::<Vec<_>>()
            )
            .is_err()
        );
    }

    #[test]
    fn maximum_member_path_fits_its_stable_bound() {
        let member = "a"
            .repeat(COMPONENT_DEPLOYMENT_NAME_MAX_BYTES)
            .parse::<ComponentGroupMemberId>()
            .expect("maximum group member");
        let path = ComponentGroupMemberPath::try_from(vec![
            member;
            COMPONENT_GROUP_MEMBER_PATH_MAX_SEGMENTS
        ])
        .expect("maximum member path");
        let bytes = path.to_bytes();

        assert!(bytes.len() <= 1_024);
        assert_eq!(ComponentGroupMemberPath::from_bytes(bytes), path);
    }
}
