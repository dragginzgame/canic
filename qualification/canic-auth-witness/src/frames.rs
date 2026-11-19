//! Qualification reply fault and observation codecs.
use crate::{LIMITS, OBSERVE};
use ic_platform_runtime::system as api;
use serde::{Serialize, Serializer};

// Invoke the production encoder with a deliberately failing serializer after
// real engine/application mutation. No production fault switch or alternate mount.
pub(crate) struct Unencodable;
impl Serialize for Unencodable {
    fn serialize<S: Serializer>(&self, _: S) -> Result<S::Ok, S::Error> {
        Err(serde::ser::Error::custom(
            "qualification reply encoding fault",
        ))
    }
}

pub(crate) fn reply_fault<const ID: u64>(enabled: bool) {
    if enabled {
        let _ = canic::auth::endpoint::encode::<_, ID>(Unencodable).unwrap_or_else(|error| {
            api::trap(&format!("qualification reply encoding fault: {error:?}"))
        });
    }
}

pub(crate) fn decode_observe(bytes: &[u8]) -> Result<(), std::convert::Infallible> {
    assert!(bytes.is_empty());
    Ok(())
}

pub(crate) fn encode_observe<T: Serialize>(value: T) -> Result<Vec<u8>, ic_wire::Error> {
    ic_wire::encode(
        &value,
        ic_wire::Encoding::Frame {
            method: OBSERVE,
            direction: ic_wire::Direction::Response,
        },
        LIMITS,
    )
}
