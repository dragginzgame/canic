use crate::dto::prelude::*;

#[cfg(feature = "external-candid")]
pub use crate::cycles::CyclesConversionError;
pub use crate::values::cycles::{CycleTopupEventStatus, CycleTopupFailureDisposition};
pub use crate::{
    cycles::BC, cycles::Cycles, cycles::CyclesParseError, cycles::KC, cycles::MC, cycles::QC,
    cycles::TC,
};

//
// CycleTrackerEntry
//

#[derive(Deserialize, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct CycleTrackerEntry {
    pub timestamp_secs: u64,
    pub cycles: Cycles,
}
