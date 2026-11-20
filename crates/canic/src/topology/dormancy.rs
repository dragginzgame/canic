//! Installation evidence for a retired allocation held by its source Root.

use crate::method::Contract;
use ic_platform_types::Principal;
use serde::{Deserialize, Serialize};

///
/// Receipt
/// Binds the retired allocation and its proof deadline to the source Root and child.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Receipt {
    /// Actual installing Root and intended custodian.
    pub root: Principal,
    /// Canister whose previous application was retired.
    pub subject: Principal,
    /// Generation of the drained allocation.
    pub generation: ic_wire::ByteArray<32>,
    /// Original proof deadline retained on closure, in IC nanoseconds.
    pub proof_deadline_ns: u64,
}

impl Receipt {
    const FRAME: ic_wire::Encoding = ic_wire::Encoding::Frame {
        method: ic_wire::method_id("canic.dormant.install.v1"),
        direction: ic_wire::Direction::Request,
    };

    /// Encode the reviewed installation arguments.
    ///
    /// # Errors
    /// Returns any bounded encoding error.
    pub fn encode(&self) -> Result<Vec<u8>, ic_wire::Error> {
        ic_wire::encode(self, Self::FRAME, crate::method::LIMITS)
    }

    /// Decode the exact versioned retirement binding.
    ///
    /// # Errors
    /// Refuses malformed or oversized installation arguments.
    pub fn decode(bytes: &[u8]) -> Result<Self, ic_wire::Error> {
        ic_wire::decode(bytes, Self::FRAME, crate::method::LIMITS)
    }
}

///
/// DormancyReceipt
/// Wire contract for reading the installation receipt through a replicated call.
///

pub struct DormancyReceipt;

impl Contract for DormancyReceipt {
    const NAME: &'static str = "dormancy_receipt";
    const ID: u64 = ic_wire::method_id(Self::NAME);
    type Input = ();
    type Output = Receipt;
}
