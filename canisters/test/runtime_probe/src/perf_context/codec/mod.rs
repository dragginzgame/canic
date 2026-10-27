//! Method-bound native codecs for uninstrumented performance observation queries.
//! The bare CDK adapters preserve measurement isolation; Wire owns frame admission.

macro_rules! observation_codec {
    ($module:ident, $identity:ident) => {
        pub mod $module {
            /// Admit the exact method's unit request before reading observation state.
            pub fn decode_args(bytes: Vec<u8>) {
                ic_wire::native::decode_request::<()>(
                    crate::timer_wire::$identity,
                    &bytes,
                    crate::timer_wire::REQUEST_LIMITS,
                )
                .unwrap_or_else(|error| ic_platform::cdk::api::trap(error.to_string()));
            }
            /// Frame only this observation's reply without recording another sample.
            pub fn encode_reply<T: serde::Serialize>(value: T) -> Vec<u8> {
                ic_wire::native::encode_response(
                    crate::timer_wire::$identity,
                    &value,
                    crate::timer_wire::RESPONSE_LIMITS,
                )
                .unwrap_or_else(|error| ic_platform::cdk::api::trap(error.to_string()))
            }
        }
    };
}

observation_codec!(entered, PERF_ENTERED);
observation_codec!(completed, PERF_COMPLETED);
observation_codec!(metrics, PERF_METRICS);
