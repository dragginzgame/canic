//! Module: model::placement::allocation
//!
//! Responsibility: derive canonical identities for receipt-backed child allocation.
//! Does not own: admission, stable storage, root RPC execution, or registry mutation.
//! Boundary: placement workflows use these values to bind intents and replay receipts.

use crate::{
    cdk::types::Principal,
    ids::{CanisterRole, IntentResourceKey},
    model::{
        intent::PayloadBinding,
        replay::{OperationId, ReplayPayloadHasher},
    },
};
use sha2::{Digest, Sha256};

const ALLOCATION_OPERATION_COMMAND: &str = "placement.allocate_child";
const ALLOCATION_RESOURCE_DOMAIN: &[u8] = b"canic-placement-allocation-resource";
const PLACEMENT_RESOURCE_PREFIX: &str = "canic:placement:";

///
/// PlacementAllocationIdentity
///
/// Canonical operation, payload, and capacity-resource identity for one child allocation.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlacementAllocationIdentity {
    pub operation_id: OperationId,
    pub payload_binding: PayloadBinding,
    pub resource_key: IntentResourceKey,
}

struct PlacementAllocationIdentityParts<'a> {
    owner: Principal,
    placement_kind: &'a str,
    pool: &'a str,
    subject: &'a [u8],
    generation: u64,
    resource_includes_subject: bool,
    canister_role: &'a CanisterRole,
    extra_arg: Option<&'a [u8]>,
}

impl PlacementAllocationIdentity {
    /// Derive a directory allocation identity bound to one owner, pool, and logical key.
    #[must_use]
    pub fn directory(
        owner: Principal,
        pool: &str,
        key_value: &str,
        claim_id: u64,
        canister_role: &CanisterRole,
        extra_arg: Option<&[u8]>,
    ) -> Self {
        Self::derive(PlacementAllocationIdentityParts {
            owner,
            placement_kind: "directory",
            pool,
            subject: key_value.as_bytes(),
            generation: claim_id,
            resource_includes_subject: true,
            canister_role,
            extra_arg,
        })
    }

    /// Derive a scaling allocation identity bound to one owner, pool, and worker slot.
    #[must_use]
    pub fn scaling(
        owner: Principal,
        pool: &str,
        sequence: u64,
        canister_role: &CanisterRole,
        extra_arg: Option<&[u8]>,
    ) -> Self {
        Self::derive(PlacementAllocationIdentityParts {
            owner,
            placement_kind: "scaling",
            pool,
            subject: &[],
            generation: sequence,
            resource_includes_subject: false,
            canister_role,
            extra_arg,
        })
    }

    /// Derive a sharding allocation identity bound to one owner, pool, and shard slot.
    #[cfg(feature = "sharding")]
    #[must_use]
    pub fn sharding(
        owner: Principal,
        pool: &str,
        slot: u32,
        generation: u64,
        canister_role: &CanisterRole,
        extra_arg: Option<&[u8]>,
    ) -> Self {
        let slot_bytes = slot.to_be_bytes();
        Self::derive(PlacementAllocationIdentityParts {
            owner,
            placement_kind: "sharding",
            pool,
            subject: &slot_bytes,
            generation,
            resource_includes_subject: true,
            canister_role,
            extra_arg,
        })
    }

    fn derive(parts: PlacementAllocationIdentityParts<'_>) -> Self {
        let PlacementAllocationIdentityParts {
            owner,
            placement_kind,
            pool,
            subject,
            generation,
            resource_includes_subject,
            canister_role,
            extra_arg,
        } = parts;
        let command = crate::model::replay::CommandKind::new(ALLOCATION_OPERATION_COMMAND)
            .expect("allocation command kind is a valid static label");
        let actor = crate::model::replay::ReplayActor::direct_caller(owner);
        let mut operation_hasher = ReplayPayloadHasher::new(&command, &actor);
        operation_hasher.hash_str(placement_kind);
        operation_hasher.hash_str(pool);
        operation_hasher.hash_bytes(subject);
        operation_hasher.hash_u64(generation);
        let operation_id = OperationId::from_bytes(operation_hasher.finish());

        let mut payload_hasher = ReplayPayloadHasher::new(&command, &actor);
        payload_hasher.hash_bytes(operation_id.as_bytes());
        payload_hasher.hash_role(canister_role);
        payload_hasher.hash_bool(extra_arg.is_some());
        if let Some(extra_arg) = extra_arg {
            payload_hasher.hash_bytes(extra_arg);
        }
        let payload_binding = PayloadBinding::new(payload_hasher.finish());

        let mut resource_hasher = Sha256::new();
        hash_bytes(&mut resource_hasher, ALLOCATION_RESOURCE_DOMAIN);
        hash_bytes(&mut resource_hasher, owner.as_slice());
        hash_bytes(&mut resource_hasher, placement_kind.as_bytes());
        hash_bytes(&mut resource_hasher, pool.as_bytes());
        if resource_includes_subject {
            hash_bytes(&mut resource_hasher, subject);
        }
        let resource_digest: [u8; 32] = resource_hasher.finalize().into();
        let resource_key = IntentResourceKey::new(format!(
            "{PLACEMENT_RESOURCE_PREFIX}{}",
            hex_encode(&resource_digest)
        ));

        Self {
            operation_id,
            payload_binding,
            resource_key,
        }
    }
}

#[must_use]
pub fn is_placement_resource_key(resource_key: &IntentResourceKey) -> bool {
    let Some(digest) = resource_key
        .as_ref()
        .strip_prefix(PLACEMENT_RESOURCE_PREFIX)
    else {
        return false;
    };
    digest.len() == 64
        && digest
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn hash_bytes(hasher: &mut Sha256, bytes: &[u8]) {
    hasher.update(
        u64::try_from(bytes.len())
            .expect("allocation identity input length must fit u64")
            .to_be_bytes(),
    );
    hasher.update(bytes);
}

fn hex_encode(bytes: &[u8]) -> String {
    use std::fmt::Write as _;

    let mut encoded = String::with_capacity(bytes.len().saturating_mul(2));
    for byte in bytes {
        let _ = write!(encoded, "{byte:02x}");
    }
    encoded
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn p(id: u8) -> Principal {
        Principal::from_slice(&[id; 29])
    }

    #[test]
    fn identities_are_stable_and_bind_every_effect_field() {
        let role = CanisterRole::new("worker");
        let expected = PlacementAllocationIdentity::scaling(p(1), "pool", 3, &role, None);

        assert_eq!(
            PlacementAllocationIdentity::scaling(p(1), "pool", 3, &role, None),
            expected
        );
        assert_ne!(
            PlacementAllocationIdentity::scaling(p(2), "pool", 3, &role, None),
            expected
        );
        assert_ne!(
            PlacementAllocationIdentity::scaling(p(1), "other", 3, &role, None),
            expected
        );
        assert_ne!(
            PlacementAllocationIdentity::scaling(p(1), "pool", 4, &role, None),
            expected
        );
        assert_ne!(
            PlacementAllocationIdentity::scaling(
                p(1),
                "pool",
                3,
                &CanisterRole::new("other"),
                None,
            )
            .payload_binding,
            expected.payload_binding
        );
        assert_ne!(
            PlacementAllocationIdentity::scaling(p(1), "pool", 3, &role, Some(&[1])),
            expected
        );
    }

    #[test]
    fn placement_resource_keys_use_only_the_reserved_canonical_shape() {
        let identity = PlacementAllocationIdentity::scaling(
            p(1),
            "pool",
            3,
            &CanisterRole::new("worker"),
            None,
        );

        assert!(is_placement_resource_key(&identity.resource_key));
        assert!(!is_placement_resource_key(&IntentResourceKey::new(
            "placement:test"
        )));
        assert!(!is_placement_resource_key(&IntentResourceKey::new(
            "canic:placement:test"
        )));
        assert!(!is_placement_resource_key(&IntentResourceKey::new(
            format!("canic:placement:{}", "A".repeat(64))
        )));
    }

    #[test]
    fn resource_scope_matches_placement_capacity_authority() {
        let role = CanisterRole::new("worker");
        let scaling_a = PlacementAllocationIdentity::scaling(p(1), "pool", 1, &role, None);
        let scaling_b = PlacementAllocationIdentity::scaling(p(1), "pool", 2, &role, None);
        assert_eq!(scaling_a.resource_key, scaling_b.resource_key);

        let directory_a = PlacementAllocationIdentity::directory(p(1), "pool", "a", 1, &role, None);
        let directory_b = PlacementAllocationIdentity::directory(p(1), "pool", "b", 1, &role, None);
        assert_ne!(directory_a.resource_key, directory_b.resource_key);

        #[cfg(feature = "sharding")]
        {
            let sharding_a = PlacementAllocationIdentity::sharding(p(1), "pool", 1, 0, &role, None);
            let sharding_b = PlacementAllocationIdentity::sharding(p(1), "pool", 2, 0, &role, None);
            assert_ne!(sharding_a.resource_key, sharding_b.resource_key);
        }
    }

    #[cfg(feature = "sharding")]
    #[test]
    fn placement_kinds_cannot_share_operation_or_resource_identity() {
        let role = CanisterRole::new("worker");
        let scaling = PlacementAllocationIdentity::scaling(p(1), "pool", 3, &role, None);
        let sharding = PlacementAllocationIdentity::sharding(p(1), "pool", 3, 0, &role, None);

        assert_ne!(scaling.operation_id, sharding.operation_id);
        assert_ne!(scaling.resource_key, sharding.resource_key);
    }

    #[test]
    fn directory_claims_and_shard_generations_advance_operations_not_capacity_scope() {
        let role = CanisterRole::new("worker");
        let directory_first =
            PlacementAllocationIdentity::directory(p(1), "pool", "key", 1, &role, None);
        let directory_next =
            PlacementAllocationIdentity::directory(p(1), "pool", "key", 2, &role, None);
        assert_ne!(directory_first.operation_id, directory_next.operation_id);
        assert_eq!(directory_first.resource_key, directory_next.resource_key);

        #[cfg(feature = "sharding")]
        {
            let shard_first =
                PlacementAllocationIdentity::sharding(p(1), "pool", 3, 0, &role, None);
            let shard_next = PlacementAllocationIdentity::sharding(p(1), "pool", 3, 1, &role, None);
            assert_ne!(shard_first.operation_id, shard_next.operation_id);
            assert_eq!(shard_first.resource_key, shard_next.resource_key);
        }
    }
}
