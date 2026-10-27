//
// CapabilityVersion
//

use crate::dto::{
    prelude::*,
    rpc::{CyclesRequest, CyclesResponse, Request, Response},
};

pub const CAPABILITY_VERSION_V1: u16 = 1;

//
// CapabilityService
//

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub enum CapabilityService {
    Root,
}

//
// CapabilityRequestMetadata
//

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct CapabilityRequestMetadata {
    pub request_id: [u8; 32],
    pub issued_at_ns: u64,
    pub ttl_ns: u64,
}

//
// CapabilityProof
//

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub enum CapabilityProof {
    Structural,
}

//
// RootCapabilityEnvelopeV1
//

#[derive(Clone, Debug)]
pub struct RootCapabilityEnvelopeV1 {
    pub service: CapabilityService,
    pub capability_version: u16,
    pub capability: Request,
    pub proof: CapabilityProof,
    pub metadata: CapabilityRequestMetadata,
}

//
// RootCapabilityResponseV1
//

#[derive(Clone, Debug)]
pub struct RootCapabilityResponseV1 {
    pub response: Response,
}

//
// NonrootCyclesCapabilityEnvelopeV1
//

#[derive(Clone, Debug, Deserialize, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct NonrootCyclesCapabilityEnvelopeV1 {
    pub service: CapabilityService,
    pub capability_version: u16,
    pub capability: CyclesRequest,
    pub proof: CapabilityProof,
    pub metadata: CapabilityRequestMetadata,
}

//
// NonrootCyclesCapabilityResponseV1
//

#[derive(Clone, Debug, Deserialize, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct NonrootCyclesCapabilityResponseV1 {
    pub response: CyclesResponse,
}
