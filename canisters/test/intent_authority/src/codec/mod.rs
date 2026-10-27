//! Explicit method-bound Wire hooks for standalone receipt tests.

macro_rules! codec {
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
codec!(begin, BEGIN, crate::protocol::BeginRequest);
codec!(load, LOAD, u8);
codec!(settle, SETTLE, crate::protocol::SettleRequest);
