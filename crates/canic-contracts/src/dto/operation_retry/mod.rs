//! Controller-owned observation and exact resumption of retained business work.
use crate::ids::ComponentInstanceId;
use serde::{Deserialize, Serialize};

/// Exact operation owner; no selector creates a new business operation.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub enum Operation {
    CoordinatorAdmission {
        operation_id: [u8; 32],
    },
    CoordinatorFunding {
        operation_id: [u8; 32],
    },
    CoordinatorProvisioning {
        operation_id: [u8; 32],
    },
    CoordinatorRootRemoval {
        operation_id: [u8; 32],
    },
    RootAllocation {
        operation_id: [u8; 32],
    },
    ComponentRemoval {
        component: ComponentInstanceId,
        operation_id: [u8; 32],
    },
    SubtreeRemoval {
        component: ComponentInstanceId,
        operation_id: [u8; 32],
    },
}

/// Retry budget state, independent of the retained effect journal's phase.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub enum State {
    Active,
    ReviewRequired,
    BudgetExhausted,
}

/// Durable retry visibility for one exact operation.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct Status {
    pub operation: Operation,
    pub generation: u64,
    pub attempts_without_progress: u32,
    pub maximum_attempts_without_progress: u32,
    pub retry_at_ns: u64,
    pub diagnostic_code: Option<u16>,
    pub state: State,
}

/// Renew only the observed suspended window; replay cannot renew it twice.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct ResumeRequest {
    pub operation: Operation,
    pub expected_generation: u64,
}
