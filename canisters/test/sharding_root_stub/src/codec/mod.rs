//! Native capability and fixture control codecs.

/// Exact acknowledge capability framing shared with the production caller.
pub mod acknowledge {
    use canic::protocol::root_capability as wire;
    pub fn decode(
        bytes: Vec<u8>,
    ) -> canic::dto::root_capability::CapabilityRequest<
        canic::dto::rpc::AcknowledgePlacementReceiptRequest,
    > {
        ic_wire::native::decode_request(wire::acknowledge::METHOD_ID, &bytes, wire::REQUEST_LIMITS)
            .unwrap_or_else(|error| ic_platform::cdk::api::trap(error.to_string()))
    }
    pub fn encode(value: Result<(), canic::Error>) -> Vec<u8> {
        ic_wire::native::encode_response(
            wire::acknowledge::METHOD_ID,
            &value,
            wire::RESPONSE_LIMITS,
        )
        .unwrap_or_else(|error| ic_platform::cdk::api::trap(error.to_string()))
    }
}

/// Exact allocate capability framing shared with the production caller.
pub mod allocate {
    use canic::protocol::root_capability as wire;
    pub fn decode(
        bytes: Vec<u8>,
    ) -> canic::dto::root_capability::CapabilityRequest<
        canic::dto::root_capability::CreateCanisterRequest,
    > {
        ic_wire::native::decode_request(wire::allocate::METHOD_ID, &bytes, wire::REQUEST_LIMITS)
            .unwrap_or_else(|error| ic_platform::cdk::api::trap(error.to_string()))
    }
    pub fn encode(
        value: Result<canic::dto::root_capability::CreateCanisterResponse, canic::Error>,
    ) -> Vec<u8> {
        ic_wire::native::encode_response(wire::allocate::METHOD_ID, &value, wire::RESPONSE_LIMITS)
            .unwrap_or_else(|error| ic_platform::cdk::api::trap(error.to_string()))
    }
}

/// Exact create capability framing shared with the production caller.
pub mod create {
    use canic::protocol::root_capability as wire;
    pub fn decode(
        bytes: Vec<u8>,
    ) -> canic::dto::root_capability::CapabilityRequest<
        canic::dto::root_capability::CreateCanisterRequest,
    > {
        ic_wire::native::decode_request(wire::create::METHOD_ID, &bytes, wire::REQUEST_LIMITS)
            .unwrap_or_else(|error| ic_platform::cdk::api::trap(error.to_string()))
    }
    pub fn encode(
        value: Result<canic::dto::root_capability::CreateCanisterResponse, canic::Error>,
    ) -> Vec<u8> {
        ic_wire::native::encode_response(wire::create::METHOD_ID, &value, wire::RESPONSE_LIMITS)
            .unwrap_or_else(|error| ic_platform::cdk::api::trap(error.to_string()))
    }
}

/// Exact recycle capability framing shared with the production caller.
pub mod recycle {
    use canic::protocol::root_capability as wire;
    pub fn decode(
        bytes: Vec<u8>,
    ) -> canic::dto::root_capability::CapabilityRequest<
        canic::dto::root_capability::RecycleCanisterRequest,
    > {
        ic_wire::native::decode_request(wire::recycle::METHOD_ID, &bytes, wire::REQUEST_LIMITS)
            .unwrap_or_else(|error| ic_platform::cdk::api::trap(error.to_string()))
    }
    pub fn encode(value: Result<(), canic::Error>) -> Vec<u8> {
        ic_wire::native::encode_response(wire::recycle::METHOD_ID, &value, wire::RESPONSE_LIMITS)
            .unwrap_or_else(|error| ic_platform::cdk::api::trap(error.to_string()))
    }
}

/// Exact cycles capability framing shared with the production caller.
pub mod cycles {
    use canic::protocol::root_capability as wire;
    pub fn decode(
        bytes: Vec<u8>,
    ) -> canic::dto::root_capability::CapabilityRequest<canic::dto::rpc::CyclesRequest> {
        ic_wire::native::decode_request(wire::cycles::METHOD_ID, &bytes, wire::REQUEST_LIMITS)
            .unwrap_or_else(|error| ic_platform::cdk::api::trap(error.to_string()))
    }
    pub fn encode(value: Result<canic::dto::rpc::CyclesResponse, canic::Error>) -> Vec<u8> {
        ic_wire::native::encode_response(wire::cycles::METHOD_ID, &value, wire::RESPONSE_LIMITS)
            .unwrap_or_else(|error| ic_platform::cdk::api::trap(error.to_string()))
    }
}

macro_rules! control_codec {
    ($module:ident, $id:ident, $request:ty) => {
        pub mod $module {
            pub fn decode(bytes: Vec<u8>) -> $request {
                ic_wire::native::decode_request(
                    crate::protocol::$id,
                    &bytes,
                    crate::protocol::LIMITS,
                )
                .unwrap_or_else(|error| ic_platform::cdk::api::trap(error.to_string()))
            }
            pub fn encode<T: serde::Serialize>(value: T) -> Vec<u8> {
                ic_wire::native::encode_response(
                    crate::protocol::$id,
                    &value,
                    crate::protocol::LIMITS,
                )
                .unwrap_or_else(|error| ic_platform::cdk::api::trap(error.to_string()))
            }
        }
    };
}
control_codec!(allocations, ALLOCATIONS, ());
control_codec!(is_settled, IS_SETTLED, [u8; 32]);
control_codec!(settle, SETTLE, [u8; 32]);
