//! Store-backed authority; legacy inline approvals keep their original wire representation.
use crate::children;
use canic_wasm_store_protocol::{Manifest, Pin};
use serde::{Deserialize, Serialize};

/// Admit Store-backed child creation.
pub const STAGE: u64 = 41;
/// Reconcile creation with its exact Store binding.
pub const RECONCILE: u64 = 42;
/// Admit a Store-backed child upgrade.
pub const STAGE_UPGRADE: u64 = 43;
/// Reconcile an upgrade with its exact Store binding.
pub const RECONCILE_UPGRADE: u64 = 44;
/// Complete journals and artifact retention evidence.
pub const CUSTODY: u64 = 45;
/// Bounded metadata for two children and their eight retained upgrades each.
pub const LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 65_536,
    allocation: 4_194_304,
    nodes: 262_144,
    depth: 24,
};

/// Exact published content and its enduring supplier.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Reference {
    /// Canonical Store canister principal, on the owner's subnet.
    pub store: String,
    /// Immutable release and ordered content commitments; never “latest”.
    pub manifest: Manifest,
}

/// Approval and artifact supplier form one immutable operation.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Stage<A> {
    /// Existing effect authority, unchanged in its legacy encoding.
    pub approval: A,
    /// Retained before any effect is dispatched.
    pub reference: Reference,
}
impl<A: Serialize> Stage<A> {
    /// Domain-separated identity for the caller-owned Store pin.
    /// # Panics
    /// Panics if the caller supplies metadata outside the bounded contract.
    #[must_use]
    pub fn pin(&self, method: u64, slot: children::Slot) -> Pin {
        let bytes = ic_wire::encode(&(method, slot, self), ic_wire::Encoding::Payload, LIMITS)
            .expect("bounded Store operation");
        Pin {
            operation: crate::digest(&bytes),
            manifest: self.reference.manifest.clone(),
        }
    }
}

/// Whether durable retention evidence has committed at the effect owner.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub enum PinState {
    /// No retention request dispatched.
    #[default]
    Pending,
    /// Request committed; reply may have been lost.
    Issued,
    /// Exact Store receipt retained or reconstructed through replicated observation.
    Retained,
}
/// Bounded artifact delivery evidence retained independently of the management journal.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Delivery {
    /// Exact supplier and content.
    pub reference: Reference,
    /// Digest of the complete operation; the manifest has one retained owner above.
    pub operation: [u8; 32],
    /// Retention progress; an uncertain request is never blindly reissued.
    pub state: PinState,
    /// Original cache preparation deadline, retained with the outgoing call.
    pub cache_deadline: Option<u64>,
}
impl Delivery {
    /// Reconstruct the exact Store request from retained operation and manifest.
    #[must_use]
    pub fn pin(&self) -> Pin {
        Pin {
            operation: self.operation,
            manifest: self.reference.manifest.clone(),
        }
    }
}
/// One child's Store evidence, including all admitted upgrade sources.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Sources {
    /// Creation source.
    pub creation: Delivery,
    /// Upgrade sources in the same order as the complete upgrade journal.
    pub upgrades: Vec<Delivery>,
}
/// Complete maintenance evidence. Legacy custody methods refuse Store-backed children.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Custody {
    /// Complete creation and upgrade journals.
    pub children: children::Custody,
    /// None denotes an inline child; no raw bytes are exported.
    pub sources: [Option<Sources>; 2],
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Approval, CREATION_CYCLES, Status, custody, upgrade};
    use canic_wasm_store_protocol::{CHUNK_BYTES, MAX_CHUNKS, Release};

    #[test]
    fn complete_store_custody_fits_and_operation_identity_binds_source_slot_and_action() {
        let manifest = Manifest {
            release: Release {
                name: "a".repeat(64),
                version: "v".repeat(64),
            },
            hash: [255; 32],
            bytes: (CHUNK_BYTES * MAX_CHUNKS) as u64,
            chunks: vec![[255; 32]; MAX_CHUNKS],
        };
        let stage = Stage {
            approval: Approval {
                manager: "a".repeat(63),
                module_hash: manifest.hash,
                creation_cycles: CREATION_CYCLES,
                execute_at_ns: u64::MAX - 1,
                expires_at_ns: u64::MAX,
            },
            reference: Reference {
                store: "b".repeat(63),
                manifest,
            },
        };
        let operation = stage.pin(STAGE, children::Slot::First).operation;
        assert_ne!(
            operation,
            stage.pin(STAGE, children::Slot::Second).operation
        );
        assert_ne!(
            operation,
            stage.pin(STAGE_UPGRADE, children::Slot::First).operation
        );
        let mut changed = stage.clone();
        changed.reference.store.push('c');
        assert_ne!(
            operation,
            changed.pin(STAGE, children::Slot::First).operation
        );
        changed = stage.clone();
        changed.reference.manifest.release.version.push('x');
        assert_ne!(
            operation,
            changed.pin(STAGE, children::Slot::First).operation
        );
        let delivery = Delivery {
            reference: stage.reference.clone(),
            operation,
            state: PinState::Retained,
            cache_deadline: Some(u64::MAX),
        };
        let sources = Sources {
            creation: delivery.clone(),
            upgrades: vec![delivery; upgrade::MAX_UPGRADES],
        };
        let journal = custody::Custody {
            creation: Status {
                approval: Some(stage.approval),
                worker: Some("c".repeat(63)),
                attempts: [1; 3],
                requests: [Some([255; 32]); 3],
                creation_refund: Some(u128::MAX),
                observed_module: Some(vec![255; 32]),
                ..Default::default()
            },
            upgrades: vec![
                upgrade::Status {
                    approval: Some(upgrade::Approval {
                        manager: "a".repeat(63),
                        child: "c".repeat(63),
                        previous_module: [255; 32],
                        module_hash: [254; 32],
                        execute_at_ns: u64::MAX - 1,
                        expires_at_ns: u64::MAX
                    }),
                    attempts: [1; 2],
                    requests: [Some([255; 32]); 2],
                    previous_version: Some(u64::MAX - 1),
                    observed_version: Some(u64::MAX),
                    ..Default::default()
                };
                upgrade::MAX_UPGRADES
            ],
        };
        let custody = Custody {
            children: [journal.clone(), journal],
            sources: [Some(sources.clone()), Some(sources)],
        };
        let bytes = ic_wire::encode(
            &custody,
            ic_wire::Encoding::Frame {
                method: CUSTODY,
                direction: ic_wire::Direction::Response,
            },
            LIMITS,
        )
        .unwrap();
        assert_eq!(
            ic_wire::decode::<Custody>(
                &bytes,
                ic_wire::Encoding::Frame {
                    method: CUSTODY,
                    direction: ic_wire::Direction::Response
                },
                LIMITS
            )
            .unwrap(),
            custody
        );
    }
}
