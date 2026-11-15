//! Explicit lifecycle and controller-only publication/read endpoints.
use crate::state::State;
use canic_wasm_store_protocol as protocol;
use ic_platform_runtime::cdk;
use ic_storage::DefaultMemoryImpl;
use std::cell::RefCell;
mod cache;
thread_local! { static STATE: RefCell<Option<State<DefaultMemoryImpl>>> = const { RefCell::new(None) }; }
#[cdk::init]
fn init() {
    STATE.with_borrow_mut(|state| *state = Some(State::open(DefaultMemoryImpl::default(), true)));
}
#[cdk::post_upgrade]
fn post_upgrade() {
    STATE.with_borrow_mut(|state| *state = Some(State::open(DefaultMemoryImpl::default(), false)));
}

// Each concrete declaration owns its access, frame budgets and protocol identity.
// Platform owns IC entrypoints; Wire owns codecs. No lifecycle is generated here.
macro_rules! endpoint {
    ($name:ident, $kind:ident, $id:ident, $request:ty, $reply:ty, $input_limits:ident, $output_limits:ident, $operation:expr) => {
        mod $name {
            use super::{DefaultMemoryImpl, STATE, State, cdk, protocol};
            fn admit() -> Result<(), String> {
                if !cdk::api::is_controller(&cdk::api::msg_caller()) {
                    return Err("Store controller required".into());
                }
                if ic_platform_runtime::ic0::msg_arg_data_size() > protocol::$input_limits.bytes {
                    return Err("Store frame too large".into());
                }
                Ok(())
            }

            fn decode(bytes: Vec<u8>) -> $request {
                ic_wire::decode(
                    &bytes,
                    ic_wire::Encoding::Frame {
                        method: protocol::$id,
                        direction: ic_wire::Direction::Request,
                    },
                    protocol::$input_limits,
                )
                .expect("Store request")
            }

            fn encode(reply: $reply) -> Vec<u8> {
                ic_wire::encode(
                    &reply,
                    ic_wire::Encoding::Frame {
                        method: protocol::$id,
                        direction: ic_wire::Direction::Response,
                    },
                    protocol::$output_limits,
                )
                .expect("Store reply")
            }
            #[cdk::$kind(
                crate = "cdk",
                guard = "admit",
                decode_with = "decode",
                encode_with = "encode"
            )]

            fn $name(input: $request) -> $reply {
                STATE.with_borrow_mut(|state| {
                    let operation: fn(&mut State<DefaultMemoryImpl>, &$request) -> $reply =
                        $operation;
                    operation(state.as_mut().expect("initialized Store"), &input)
                })
            }
        }
    };
}
endpoint!(prepare, update, PREPARE, protocol::Manifest, Result<protocol::Status, protocol::Error>, LIMITS, LIMITS, |state, input| state.prepare(input));
endpoint!(put, update, PUT, protocol::Put, Result<protocol::Status, protocol::Error>, CHUNK_LIMITS, LIMITS, |state, input| state.put(input));
endpoint!(publish, update, PUBLISH, protocol::Release, Result<protocol::Status, protocol::Error>, LIMITS, LIMITS, |state, input| state.publish(input));
endpoint!(status, query, STATUS, protocol::Release, Result<protocol::Status, protocol::Error>, LIMITS, LIMITS, |state, input| state.status(input));
endpoint!(observe, update, STATUS, protocol::Release, Result<protocol::Status, protocol::Error>, LIMITS, LIMITS, |state, input| state.status(input));
endpoint!(observe_custody, update, CUSTODY, protocol::Release, Result<protocol::Custody, protocol::Error>, LIMITS, LIMITS, |state, input| state.custody(input));
endpoint!(observe_preservation, update, PRESERVATION, protocol::preservation::Request, Result<protocol::preservation::Page, protocol::Error>, LIMITS, LIMITS, |state, input| state.preservation(cdk::api::msg_caller().as_slice(), input));
endpoint!(
    admission,
    update,
    ADMISSION,
    protocol::AdmissionCommand,
    Result<Option<protocol::Admission>, protocol::Error>,
    LIMITS,
    LIMITS,
    |state, input| state.admission(cdk::api::msg_caller().as_slice(), input)
);
endpoint!(read, query, READ, protocol::Read, Result<protocol::Chunk, protocol::Error>, LIMITS, CHUNK_LIMITS, |state, input| state.read(input));
endpoint!(read_chunk, update, READ, protocol::Read, Result<protocol::Chunk, protocol::Error>, LIMITS, CHUNK_LIMITS, |state, input| state.read(input));
endpoint!(
    list,
    query,
    LIST,
    Option<protocol::Release>,
    Result<Vec<protocol::Status>, protocol::Error>,
    LIMITS,
    LIMITS,
    |state, input| state.list(input.as_ref())
);
endpoint!(
    info,
    query,
    INFO,
    (),
    protocol::Info,
    LIMITS,
    LIMITS,
    |state, _| state.info()
);
endpoint!(
    observe_info,
    update,
    INFO,
    (),
    protocol::Info,
    LIMITS,
    LIMITS,
    |state, _| state.info()
);
endpoint!(configure, update, CONFIGURE, protocol::Quota, Result<protocol::Info, protocol::Error>, LIMITS, LIMITS, |state, input| state.configure(input));
endpoint!(pin, update, PIN, protocol::Pin, Result<protocol::PinReceipt, protocol::Error>, LIMITS, LIMITS, |state, input| state.pin(cdk::api::msg_caller().as_slice(), input));
endpoint!(observe_pin, update, OBSERVE_PIN, protocol::Pin, Result<protocol::PinReceipt, protocol::Error>, LIMITS, LIMITS, |state, input| state.observe_pin(cdk::api::msg_caller().as_slice(), input));
