//! Module: storage::stable::fleet_admission_projection
//!
//! Responsibility: persist the sole target-local Fleet admission projection.
//! Does not own: projection compilation, phase policy, endpoint authorization, or distribution.
//! Boundary: ops converts complete model state to and from this memory-ID-61 record.

use crate::model::fleet_admission_projection::FleetAdmissionTargetTransitionPhaseModel;
use crate::{
    cdk::structures::{
        DefaultMemoryImpl, btreemap::BTreeMap as StableBtreeMap, memory::VirtualMemory,
    },
    ids::{FleetAdmissionProjection, MAX_FLEET_ADMISSION_PROJECTION_RECORD_BYTES},
    role_contract::allocation::memory::fleet_admission_projection::FLEET_ADMISSION_PROJECTION_ID,
    storage::prelude::*,
};
use std::cell::RefCell;

const FLEET_ADMISSION_PROJECTION_RECORD_KEY: u8 = 0;

eager_static! {
    static FLEET_ADMISSION_PROJECTION: RefCell<
        StableBtreeMap<u8, FleetAdmissionProjectionRecord, VirtualMemory<DefaultMemoryImpl>>,
    > = RefCell::new(StableBtreeMap::init(crate::ic_memory_key!(
        authority = CANIC_CORE_MEMORY_AUTHORITY,
        key = "canic.core.fleet_admission.projection.v1",
        ty = FleetAdmissionProjectionStore,
        id = FLEET_ADMISSION_PROJECTION_ID,
    )));
}

/// Stable local fence/open phase.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum FleetAdmissionProjectionPhaseRecord {
    Fenced,
    Open,
}

/// Stable retained exact-retry receipt.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FleetAdmissionProjectionReceiptRecord {
    pub operation_id: [u8; 32],
    pub phase: FleetAdmissionTargetTransitionPhaseRecord,
    pub request_hash: [u8; 32],
    pub receipt_hash: [u8; 32],
}

/// Stable closed target transition phase.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum FleetAdmissionTargetTransitionPhaseRecord {
    Prepare,
    Activate,
    Open,
}

impl From<FleetAdmissionTargetTransitionPhaseModel> for FleetAdmissionTargetTransitionPhaseRecord {
    fn from(value: FleetAdmissionTargetTransitionPhaseModel) -> Self {
        match value {
            FleetAdmissionTargetTransitionPhaseModel::Prepare => Self::Prepare,
            FleetAdmissionTargetTransitionPhaseModel::Activate => Self::Activate,
            FleetAdmissionTargetTransitionPhaseModel::Open => Self::Open,
        }
    }
}

impl From<FleetAdmissionTargetTransitionPhaseRecord> for FleetAdmissionTargetTransitionPhaseModel {
    fn from(value: FleetAdmissionTargetTransitionPhaseRecord) -> Self {
        match value {
            FleetAdmissionTargetTransitionPhaseRecord::Prepare => Self::Prepare,
            FleetAdmissionTargetTransitionPhaseRecord::Activate => Self::Activate,
            FleetAdmissionTargetTransitionPhaseRecord::Open => Self::Open,
        }
    }
}

/// Canonical schema-1 local Fleet admission authority.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FleetAdmissionProjectionRecord {
    pub schema_version: u16,
    pub active: FleetAdmissionProjection,
    pub prepared: Option<FleetAdmissionProjection>,
    pub phase: FleetAdmissionProjectionPhaseRecord,
    pub last_receipt: Option<FleetAdmissionProjectionReceiptRecord>,
}

impl FleetAdmissionProjectionRecord {
    pub const STATE_CONTRACT_NAME: &'static str = "FleetAdmissionProjectionRecord";
}

impl_storable_bounded!(
    FleetAdmissionProjectionRecord,
    MAX_FLEET_ADMISSION_PROJECTION_RECORD_BYTES,
    false
);

/// Test/audit snapshot of the optional fresh-install record.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct FleetAdmissionProjectionData {
    pub record: Option<FleetAdmissionProjectionRecord>,
}

impl FleetAdmissionProjectionData {
    pub const STATE_CONTRACT_NAME: &'static str = "FleetAdmissionProjectionData";
}

/// Single-record stable owner.
pub struct FleetAdmissionProjectionStore;

impl FleetAdmissionProjectionStore {
    #[must_use]
    pub(crate) fn get() -> Option<FleetAdmissionProjectionRecord> {
        FLEET_ADMISSION_PROJECTION
            .with_borrow(|store| store.get(&FLEET_ADMISSION_PROJECTION_RECORD_KEY))
    }

    pub(crate) fn initialize(record: FleetAdmissionProjectionRecord) -> bool {
        FLEET_ADMISSION_PROJECTION.with_borrow_mut(|store| {
            if let Some(existing) = store.get(&FLEET_ADMISSION_PROJECTION_RECORD_KEY) {
                return existing == record;
            }
            store.insert(FLEET_ADMISSION_PROJECTION_RECORD_KEY, record);
            true
        })
    }

    pub(crate) fn replace(record: FleetAdmissionProjectionRecord) -> bool {
        FLEET_ADMISSION_PROJECTION.with_borrow_mut(|store| {
            if store.get(&FLEET_ADMISSION_PROJECTION_RECORD_KEY).is_none() {
                return false;
            }
            store.insert(FLEET_ADMISSION_PROJECTION_RECORD_KEY, record);
            true
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maximum_active_and_prepared_projection_fit_the_frozen_bound() {
        let principals = (0..256).map(principal).collect::<Vec<_>>();
        let mut active = crate::test::support::fleet_admission_projection(
            crate::test::support::managed_component_binding(),
        );
        active.generation = u64::MAX - 1;
        active.policy_digest = [0xfa; 32];
        active.projection_digest = [0xfb; 32];
        active.principals.clone_from(&principals);
        let mut prepared = active.clone();
        prepared.generation = u64::MAX;
        prepared.policy_digest = [0xfc; 32];
        prepared.projection_digest = [0xfd; 32];
        let record = FleetAdmissionProjectionRecord {
            schema_version: 1,
            active,
            prepared: Some(prepared),
            phase: FleetAdmissionProjectionPhaseRecord::Fenced,
            last_receipt: Some(FleetAdmissionProjectionReceiptRecord {
                operation_id: [0xf1; 32],
                phase: FleetAdmissionTargetTransitionPhaseRecord::Prepare,
                request_hash: [0xf2; 32],
                receipt_hash: [0xf3; 32],
            }),
        };

        let stable_bytes = crate::cdk::serialize::serialize(&record).expect("projection CBOR");
        assert!(
            stable_bytes.len() <= MAX_FLEET_ADMISSION_PROJECTION_RECORD_BYTES as usize,
            "maximum projection record uses {} bytes",
            stable_bytes.len()
        );
    }

    fn principal(index: usize) -> candid::Principal {
        let mut bytes = [0_u8; 29];
        bytes[..8].copy_from_slice(
            &u64::try_from(index)
                .expect("fixture index fits u64")
                .to_be_bytes(),
        );
        bytes[8..].fill(u8::try_from(index % 251).expect("bounded fixture byte"));
        candid::Principal::from_slice(&bytes)
    }
}
