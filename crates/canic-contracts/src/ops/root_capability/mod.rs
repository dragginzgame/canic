//! Lossless capability projections and exact operation-to-frame correlation.
use crate::{
    dto::{
        capability::{RootCapabilityEnvelopeV1, RootCapabilityResponseV1},
        error::Error,
        root_capability as native, rpc as stored,
    },
    protocol::root_capability as wire,
};
impl From<stored::CreateCanisterRequest> for native::CreateCanisterRequest {
    fn from(value: stored::CreateCanisterRequest) -> Self {
        Self {
            canister_role: value.canister_role,
            parent: value.parent.into(),
            extra_arg: value.extra_arg,
            metadata: value.metadata,
        }
    }
}
impl From<native::CreateCanisterRequest> for stored::CreateCanisterRequest {
    fn from(value: native::CreateCanisterRequest) -> Self {
        Self {
            canister_role: value.canister_role,
            parent: value.parent.into(),
            extra_arg: value.extra_arg,
            metadata: value.metadata,
        }
    }
}
impl From<stored::RecycleCanisterRequest> for native::RecycleCanisterRequest {
    fn from(value: stored::RecycleCanisterRequest) -> Self {
        Self {
            allocation_operation_id: value.allocation_operation_id,
            canister_pid: value.canister_pid.into(),
            metadata: value.metadata,
        }
    }
}
impl From<native::RecycleCanisterRequest> for stored::RecycleCanisterRequest {
    fn from(value: native::RecycleCanisterRequest) -> Self {
        Self {
            allocation_operation_id: value.allocation_operation_id,
            canister_pid: value.canister_pid.into(),
            metadata: value.metadata,
        }
    }
}
impl From<stored::CreateCanisterResponse> for native::CreateCanisterResponse {
    fn from(value: stored::CreateCanisterResponse) -> Self {
        Self {
            new_canister_pid: value.new_canister_pid.into(),
        }
    }
}
impl From<native::CreateCanisterResponse> for stored::CreateCanisterResponse {
    fn from(value: native::CreateCanisterResponse) -> Self {
        Self {
            new_canister_pid: value.new_canister_pid.into(),
        }
    }
}
impl From<stored::CreateCanisterParent> for native::CreateCanisterParent {
    fn from(value: stored::CreateCanisterParent) -> Self {
        match value {
            stored::CreateCanisterParent::Root => Self::Root,
            stored::CreateCanisterParent::ThisCanister => Self::ThisCanister,
            stored::CreateCanisterParent::Parent => Self::Parent,
            stored::CreateCanisterParent::Canister(pid) => Self::Canister(pid.into()),
            stored::CreateCanisterParent::Directory(role) => Self::Directory(role),
        }
    }
}
impl From<native::CreateCanisterParent> for stored::CreateCanisterParent {
    fn from(value: native::CreateCanisterParent) -> Self {
        match value {
            native::CreateCanisterParent::Root => Self::Root,
            native::CreateCanisterParent::ThisCanister => Self::ThisCanister,
            native::CreateCanisterParent::Parent => Self::Parent,
            native::CreateCanisterParent::Canister(pid) => Self::Canister(pid.into()),
            native::CreateCanisterParent::Directory(role) => Self::Directory(role),
        }
    }
}
/// Marshal a typed endpoint input into the existing canonical workflow request.
pub fn into_root_request<T>(
    value: native::CapabilityRequest<T>,
    convert: impl FnOnce(T) -> stored::Request,
) -> RootCapabilityEnvelopeV1 {
    RootCapabilityEnvelopeV1 {
        service: value.service,
        capability_version: value.capability_version,
        capability: convert(value.capability),
        proof: value.proof,
        metadata: value.metadata,
    }
}
/// Exact method and framed request; no caller authority is created by encoding.
pub struct EncodedCall {
    pub method: &'static str,
    pub method_id: u64,
    pub bytes: Vec<u8>,
}
/// Select the operation's maintained native method, retaining all original metadata.
pub fn encode_request(value: RootCapabilityEnvelopeV1) -> Result<EncodedCall, ic_wire::Error> {
    let RootCapabilityEnvelopeV1 {
        service,
        capability_version,
        capability,
        proof,
        metadata,
    } = value;
    match capability {
        stored::Request::AcknowledgePlacementReceipt(value) => {
            let request = native::CapabilityRequest {
                service,
                capability_version,
                capability: value,
                proof,
                metadata,
            };
            encode_call(
                wire::acknowledge::METHOD,
                wire::acknowledge::METHOD_ID,
                &request,
            )
        }
        stored::Request::AllocatePlacementChild(value) => {
            let request = native::CapabilityRequest {
                service,
                capability_version,
                capability: native::CreateCanisterRequest::from(value),
                proof,
                metadata,
            };
            encode_call(wire::allocate::METHOD, wire::allocate::METHOD_ID, &request)
        }
        stored::Request::CreateCanister(value) => {
            let request = native::CapabilityRequest {
                service,
                capability_version,
                capability: native::CreateCanisterRequest::from(value),
                proof,
                metadata,
            };
            encode_call(wire::create::METHOD, wire::create::METHOD_ID, &request)
        }
        stored::Request::RecycleCanister(value) => {
            let request = native::CapabilityRequest {
                service,
                capability_version,
                capability: native::RecycleCanisterRequest::from(value),
                proof,
                metadata,
            };
            encode_call(wire::recycle::METHOD, wire::recycle::METHOD_ID, &request)
        }
        stored::Request::Cycles(value) => {
            let request = native::CapabilityRequest {
                service,
                capability_version,
                capability: value,
                proof,
                metadata,
            };
            encode_call(wire::cycles::METHOD, wire::cycles::METHOD_ID, &request)
        }
    }
}
/// Preserve exact response correlation at the endpoint boundary.
pub const fn acknowledge_response(response: RootCapabilityResponseV1) -> Result<(), Error> {
    match response.response {
        stored::Response::AcknowledgePlacementReceipt => Ok(()),
        _ => Err(Error::from_registered(
            crate::diagnostics::codes::STATE_INVALID,
        )),
    }
}
/// Preserve exact response correlation at the endpoint boundary.
pub fn create_response(
    response: RootCapabilityResponseV1,
) -> Result<native::CreateCanisterResponse, Error> {
    match response.response {
        stored::Response::CreateCanister(value) => Ok(value.into()),
        _ => Err(Error::from_registered(
            crate::diagnostics::codes::STATE_INVALID,
        )),
    }
}
/// Preserve exact response correlation at the endpoint boundary.
pub const fn recycle_response(response: RootCapabilityResponseV1) -> Result<(), Error> {
    match response.response {
        stored::Response::RecycleCanister => Ok(()),
        _ => Err(Error::from_registered(
            crate::diagnostics::codes::STATE_INVALID,
        )),
    }
}
/// Preserve exact response correlation at the endpoint boundary.
pub const fn cycles_response(
    response: RootCapabilityResponseV1,
) -> Result<stored::CyclesResponse, Error> {
    match response.response {
        stored::Response::Cycles(value) => Ok(value),
        _ => Err(Error::from_registered(
            crate::diagnostics::codes::STATE_INVALID,
        )),
    }
}
/// Decode only the selected operation's response and preserve application errors.
pub fn decode_response(
    id: u64,
    bytes: &[u8],
) -> Result<Result<RootCapabilityResponseV1, Error>, ic_wire::Error> {
    match id {
        wire::acknowledge::METHOD_ID => {
            let reply: Result<(), Error> =
                ic_wire::native::decode_response(id, bytes, wire::RESPONSE_LIMITS)?;
            Ok(reply.map(|()| RootCapabilityResponseV1 {
                response: stored::Response::AcknowledgePlacementReceipt,
            }))
        }
        wire::allocate::METHOD_ID => {
            let reply: Result<native::CreateCanisterResponse, Error> =
                ic_wire::native::decode_response(id, bytes, wire::RESPONSE_LIMITS)?;
            Ok(reply.map(|value| RootCapabilityResponseV1 {
                response: stored::Response::CreateCanister(value.into()),
            }))
        }
        wire::create::METHOD_ID => {
            let reply: Result<native::CreateCanisterResponse, Error> =
                ic_wire::native::decode_response(id, bytes, wire::RESPONSE_LIMITS)?;
            Ok(reply.map(|value| RootCapabilityResponseV1 {
                response: stored::Response::CreateCanister(value.into()),
            }))
        }
        wire::recycle::METHOD_ID => {
            let reply: Result<(), Error> =
                ic_wire::native::decode_response(id, bytes, wire::RESPONSE_LIMITS)?;
            Ok(reply.map(|()| RootCapabilityResponseV1 {
                response: stored::Response::RecycleCanister,
            }))
        }
        wire::cycles::METHOD_ID => {
            let reply: Result<stored::CyclesResponse, Error> =
                ic_wire::native::decode_response(id, bytes, wire::RESPONSE_LIMITS)?;
            Ok(reply.map(|value| RootCapabilityResponseV1 {
                response: stored::Response::Cycles(value),
            }))
        }
        _ => Err(ic_wire::Error::Identity),
    }
}

fn encode_call<T: serde::Serialize>(
    method: &'static str,
    method_id: u64,
    request: &T,
) -> Result<EncodedCall, ic_wire::Error> {
    Ok(EncodedCall {
        method,
        method_id,
        bytes: ic_wire::native::encode_request(method_id, request, wire::REQUEST_LIMITS)?,
    })
}

#[cfg(test)]
mod tests;
