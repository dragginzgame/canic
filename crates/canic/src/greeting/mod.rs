//! Fixed public greeting mount and IC ingress adapter.
#[cfg(target_arch = "wasm32")]
mod endpoint;

use crate::{GREET, Greeting, LIMITS};

#[cfg(target_arch = "wasm32")]
pub use endpoint::{bounded, decode, fail, runtime};

/// Encode the fixed response; Platform traps any encoding failure.
/// # Errors
/// Refuses an unencodable or over-budget response.
#[allow(clippy::needless_pass_by_value)]
pub fn encode(value: Greeting) -> Result<Vec<u8>, ic_wire::Error> {
    ic_wire::encode(
        &value,
        ic_wire::Encoding::Frame {
            method: GREET,
            direction: ic_wire::Direction::Response,
        },
        LIMITS,
    )
}

/// Mount the demo's public `greet` query and replicated `probe` update.
/// The explicit synchronous handler accepts `&str` and returns `canic::Greeting`.
/// Mounting starts no storage, timers or lifecycle and grants no authority.
/// This fixed greeting probe does not establish readiness for other applications.
// Rustfmt 1.100 repeatedly reindents multiline attributes in this macro body.
#[rustfmt::skip]
#[macro_export]
macro_rules! mount_greeting_endpoints {
    ($handler:path) => {
        const _: fn(&str) -> $crate::Greeting = $handler;

        #[cfg(target_arch = "wasm32")]
        mod canic_greeting {
            use endpoint::runtime;
            use $crate::{Greeting, greeting as endpoint};

            #[runtime::query(
                runtime = runtime,
                guard = endpoint::bounded,
                decode = endpoint::decode,
                max_bytes = 4096,
                on_error = endpoint::fail,
                name = "greet",
                encode = endpoint::encode,
            )]
            #[allow(clippy::needless_pass_by_value)]
            fn greet(name: String) -> Greeting {
                $handler(&name)
            }

            #[runtime::update(
                runtime = runtime,
                guard = endpoint::bounded,
                decode = endpoint::decode,
                max_bytes = 4096,
                on_error = endpoint::fail,
                name = "probe",
                encode = endpoint::encode,
            )]
            #[allow(clippy::needless_pass_by_value)]
            fn probe(name: String) -> Greeting {
                $handler(&name)
            }
        }
    };
}
