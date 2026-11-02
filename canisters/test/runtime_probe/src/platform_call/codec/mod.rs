//! Admit the target's native request before the uninstrumented SDK entrypoint runs.

/// Decode only the owned target request; all replies are selected by that target.
pub fn target(bytes: Vec<u8>) -> super::protocol::TargetRequest {
    ic_wire::native::decode_request(super::protocol::TARGET, &bytes, super::protocol::LIMITS)
        .unwrap_or_else(|error| ic_platform::cdk::api::trap(error.to_string()))
}
