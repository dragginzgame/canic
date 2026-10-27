//! Host-only schemas for validated identities, matching their existing Serde text contracts.

use super::{
    COMPONENT_GROUP_MEMBER_PATH_MAX_SEGMENTS, CanonicalNetworkId, ComponentGroupDeploymentId,
    ComponentGroupMemberId, ComponentGroupMemberPath, ComponentGroupSpecId, ComponentInstanceId,
    ComponentSpecId, FleetId, FleetServiceId, ReleaseBuildId,
};
use ic_wire::typescript::{GenerationError, SchemaContext, Wire};
use serde_json::{Value, json};

/// Trusted client predicates for the canonical identity schemas.
pub const TYPESCRIPT_VALIDATOR: &str = include_str!("identities.ts");

/// Compose owner-supplied checks with Canic identity predicates; transport stays separate.
pub const TYPESCRIPT_ADAPTER: &str = include_str!("adapter.ts");

impl Wire for CanonicalNetworkId {
    fn native_schema(_: &mut SchemaContext) -> Result<Value, GenerationError> {
        Ok(json!({"k":"text", "max":64, "check":"canicNetworkId"}))
    }
}

impl Wire for FleetId {
    fn native_schema(_: &mut SchemaContext) -> Result<Value, GenerationError> {
        Ok(json!({"k":"text", "max":64, "check":"canicFleetId"}))
    }
}

impl Wire for ComponentInstanceId {
    fn native_schema(_: &mut SchemaContext) -> Result<Value, GenerationError> {
        Ok(json!({"k":"text", "max":64, "check":"canicComponentInstanceId"}))
    }
}

impl Wire for ComponentSpecId {
    fn native_schema(_: &mut SchemaContext) -> Result<Value, GenerationError> {
        Ok(json!({"k":"text", "max":40, "check":"canicComponentSpecId"}))
    }
}

impl Wire for ReleaseBuildId {
    fn native_schema(_: &mut SchemaContext) -> Result<Value, GenerationError> {
        Ok(json!({"k":"text", "max":64, "check":"canicReleaseBuildId"}))
    }
}

impl Wire for ComponentGroupSpecId {
    fn native_schema(_: &mut SchemaContext) -> Result<Value, GenerationError> {
        Ok(json!({"k":"text", "max":40, "check":"canicDeploymentName"}))
    }
}
impl Wire for ComponentGroupDeploymentId {
    fn native_schema(_: &mut SchemaContext) -> Result<Value, GenerationError> {
        Ok(json!({"k":"text", "max":40, "check":"canicDeploymentName"}))
    }
}
impl Wire for ComponentGroupMemberId {
    fn native_schema(_: &mut SchemaContext) -> Result<Value, GenerationError> {
        Ok(json!({"k":"text", "max":40, "check":"canicDeploymentName"}))
    }
}
impl Wire for FleetServiceId {
    fn native_schema(_: &mut SchemaContext) -> Result<Value, GenerationError> {
        Ok(json!({"k":"text", "max":40, "check":"canicDeploymentName"}))
    }
}
impl Wire for ComponentGroupMemberPath {
    fn native_schema(context: &mut SchemaContext) -> Result<Value, GenerationError> {
        Ok(
            json!({"k":"list", "max":COMPONENT_GROUP_MEMBER_PATH_MAX_SEGMENTS,
            "item":context.describe::<ComponentGroupMemberId>()?,
            "check":"canicMemberPath"}),
        )
    }
}

impl Wire for crate::deployment::ComponentDeploymentLabelKey {
    fn native_schema(_: &mut SchemaContext) -> Result<Value, GenerationError> {
        Ok(json!({"k":"text", "max":40, "check":"canicLabelKey"}))
    }
}
impl Wire for crate::deployment::ComponentDeploymentLabelValue {
    fn native_schema(_: &mut SchemaContext) -> Result<Value, GenerationError> {
        Ok(json!({"k":"text", "max":128, "check":"canicLabelValue"}))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ic_wire::{Limits, native};

    #[test]
    fn release_build_schema_preserves_canonical_text_and_rejects_invalid_ids() {
        let limits = Limits {
            bytes: 256,
            allocation: 4096,
            nodes: 16,
            depth: 8,
        };
        let text = "ab".repeat(32);
        let value: ReleaseBuildId = text.parse().unwrap();
        let bytes = native::encode(&value, limits).unwrap();
        assert_eq!(bytes, native::encode(&text, limits).unwrap());
        assert_eq!(
            native::decode::<ReleaseBuildId>(&bytes, limits).unwrap(),
            value
        );
        assert_eq!(
            ic_wire::typescript::native_schema::<ReleaseBuildId>().unwrap(),
            json!({"k":"text", "max":64, "check":"canicReleaseBuildId"})
        );
        for invalid in [
            "AB".repeat(32),
            "a".repeat(63),
            "a".repeat(65),
            "gg".repeat(32),
        ] {
            let bytes = native::encode(&invalid, limits).unwrap();
            assert!(native::decode::<ReleaseBuildId>(&bytes, limits).is_err());
        }
    }

    #[test]
    fn component_spec_schema_matches_exact_name_boundaries() {
        let limits = Limits {
            bytes: 256,
            allocation: 4096,
            nodes: 16,
            depth: 8,
        };
        for text in ["a".to_owned(), "A_0-".repeat(10)] {
            let identity: ComponentSpecId = text.parse().unwrap();
            let encoded = native::encode(&identity, limits).unwrap();
            assert_eq!(encoded, native::encode(&text, limits).unwrap());
            assert_eq!(
                native::decode::<ComponentSpecId>(&encoded, limits).unwrap(),
                identity
            );
        }
        assert_eq!(
            ic_wire::typescript::native_schema::<ComponentSpecId>().unwrap(),
            json!({"k":"text", "max":40, "check":"canicComponentSpecId"})
        );
    }

    #[test]
    fn network_schema_preserves_text_bytes_and_rejects_noncanonical_values() {
        let limits = Limits {
            bytes: 256,
            allocation: 4096,
            nodes: 16,
            depth: 8,
        };
        let text = "ab".repeat(32);
        let identity: CanonicalNetworkId = text.parse().unwrap();
        let encoded = native::encode(&identity, limits).unwrap();
        assert_eq!(encoded, native::encode(&text, limits).unwrap());
        assert_eq!(
            native::decode::<CanonicalNetworkId>(&encoded, limits).unwrap(),
            identity
        );
        assert_eq!(
            ic_wire::typescript::native_schema::<CanonicalNetworkId>().unwrap(),
            json!({"k":"text", "max":64, "check":"canicNetworkId"})
        );
        for invalid in [
            "AB".repeat(32),
            "a".repeat(63),
            "a".repeat(65),
            "gg".repeat(32),
        ] {
            let bytes = native::encode(&invalid, limits).unwrap();
            assert!(native::decode::<CanonicalNetworkId>(&bytes, limits).is_err());
        }
    }
    #[test]
    fn member_path_schema_preserves_validated_sequence_encoding() {
        let limits = Limits {
            bytes: 4096,
            allocation: 32768,
            nodes: 512,
            depth: 16,
        };
        let segments = vec!["A_0-".repeat(10); COMPONENT_GROUP_MEMBER_PATH_MAX_SEGMENTS];
        let path = ComponentGroupMemberPath::try_from(
            segments
                .iter()
                .map(|s| s.parse().unwrap())
                .collect::<Vec<_>>(),
        )
        .unwrap();
        let bytes = native::encode(&path, limits).unwrap();
        assert_eq!(bytes, native::encode(&segments, limits).unwrap());
        assert_eq!(
            native::decode::<ComponentGroupMemberPath>(&bytes, limits).unwrap(),
            path
        );
        for invalid in [
            vec![],
            vec!["x".to_owned(); 17],
            vec![String::new()],
            vec!["x".repeat(41)],
            vec!["x/y".to_owned()],
        ] {
            let bytes = native::encode(&invalid, limits).unwrap();
            assert!(native::decode::<ComponentGroupMemberPath>(&bytes, limits).is_err());
        }
        let schema = ic_wire::typescript::native_schema::<ComponentGroupMemberPath>().unwrap();
        assert_eq!(schema["max"], 16);
        assert_eq!(schema["check"], "canicMemberPath");
        assert_eq!(schema["item"]["check"], "canicDeploymentName");
    }
}
