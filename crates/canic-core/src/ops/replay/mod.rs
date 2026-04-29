use crate::dto::rpc::{CyclesResponse, Response};
use candid::{decode_one, encode_one};

use self::{guard::ReplayPending, slot as replay_slot};

pub mod guard;
pub mod key;
pub mod slot;
pub mod ttl;

const ROOT_REPLAY_COMPACT_TAG: &[u8] = b"RR2";
const ROOT_REPLAY_COMPACT_CYCLES_V1: u8 = 0;

///
/// ReplayReserveError
/// Mechanical replay-reservation failures surfaced by ops replay reservation APIs.
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReplayReserveError {
    CapacityReached { max_entries: usize },
}

///
/// ReplayCommitError
/// Mechanical replay-commit failures surfaced by ops replay commit APIs.
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReplayCommitError {
    EncodeFailed(String),
}

///
/// ReplayDecodeError
/// Mechanical replay-decode failures surfaced by cached replay readers.
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReplayDecodeError {
    DecodeFailed(String),
}

/// reserve_root_replay
///
/// Persist a pending replay reservation marker before capability execution.
pub fn reserve_root_replay(
    pending: ReplayPending,
    max_entries: usize,
) -> Result<(), ReplayReserveError> {
    if !replay_slot::has_root_slot(pending.slot_key) && replay_slot::root_slot_len() >= max_entries
    {
        return Err(ReplayReserveError::CapacityReached { max_entries });
    }

    replay_slot::reserve_root_slot(pending);
    Ok(())
}

/// commit_root_replay
///
/// Persist canonical response bytes for an existing root replay reservation.
pub fn commit_root_replay(
    pending: ReplayPending,
    response: &Response,
) -> Result<(), ReplayCommitError> {
    let response_bytes = encode_root_replay_response(response)?;
    replay_slot::commit_root_slot(pending, response_bytes);
    Ok(())
}

/// commit_root_cycles_replay
///
/// Persist a cached cycles response without rebuilding the enum wrapper at the call site.
pub fn commit_root_cycles_replay(pending: ReplayPending, response: &CyclesResponse) {
    let response_bytes = encode_root_cycles_replay_response(response);
    replay_slot::commit_root_slot(pending, response_bytes);
}

/// decode_root_replay_response
///
/// Decode cached replay bytes back into the canonical root response payload.
pub fn decode_root_replay_response(bytes: &[u8]) -> Result<Response, ReplayDecodeError> {
    if let Some(response) = try_decode_compact_root_replay_response(bytes)? {
        return Ok(response);
    }

    decode_one(bytes).map_err(|err| ReplayDecodeError::DecodeFailed(err.to_string()))
}

/// decode_root_cycles_replay_response
///
/// Decode cached replay bytes directly into the cycles response shape.
pub fn decode_root_cycles_replay_response(
    bytes: &[u8],
) -> Result<CyclesResponse, ReplayDecodeError> {
    let response = decode_root_replay_response(bytes)?;
    match response {
        Response::Cycles(response) => Ok(response),
        _ => Err(ReplayDecodeError::DecodeFailed(
            "cached replay payload was not a cycles response".to_string(),
        )),
    }
}

/// abort_root_replay
///
/// Remove an in-flight replay reservation after failed capability execution.
pub fn abort_root_replay(pending: ReplayPending) {
    let _ = replay_slot::remove_root_slot(pending.slot_key);
}

fn encode_root_replay_response(response: &Response) -> Result<Vec<u8>, ReplayCommitError> {
    if let Some(bytes) = try_encode_compact_root_replay_response(response) {
        return Ok(bytes);
    }

    encode_one(response).map_err(|err| ReplayCommitError::EncodeFailed(err.to_string()))
}

fn encode_root_cycles_replay_response(response: &CyclesResponse) -> Vec<u8> {
    let payload = response.cycles_transferred.to_be_bytes();
    let mut bytes = Vec::with_capacity(ROOT_REPLAY_COMPACT_TAG.len() + 1 + payload.len());
    bytes.extend_from_slice(ROOT_REPLAY_COMPACT_TAG);
    bytes.push(ROOT_REPLAY_COMPACT_CYCLES_V1);
    bytes.extend_from_slice(&payload);
    bytes
}

fn try_encode_compact_root_replay_response(response: &Response) -> Option<Vec<u8>> {
    let Response::Cycles(CyclesResponse { cycles_transferred }) = response else {
        return None;
    };

    let payload = cycles_transferred.to_be_bytes();
    let mut bytes = Vec::with_capacity(ROOT_REPLAY_COMPACT_TAG.len() + 1 + payload.len());
    bytes.extend_from_slice(ROOT_REPLAY_COMPACT_TAG);
    bytes.push(ROOT_REPLAY_COMPACT_CYCLES_V1);
    bytes.extend_from_slice(&payload);
    Some(bytes)
}

fn try_decode_compact_root_replay_response(
    bytes: &[u8],
) -> Result<Option<Response>, ReplayDecodeError> {
    if !bytes.starts_with(ROOT_REPLAY_COMPACT_TAG) {
        return Ok(None);
    }

    let Some((&kind, mut payload)) = bytes[ROOT_REPLAY_COMPACT_TAG.len()..].split_first() else {
        return Err(ReplayDecodeError::DecodeFailed(
            "root replay compact payload missing variant tag".to_string(),
        ));
    };

    match kind {
        ROOT_REPLAY_COMPACT_CYCLES_V1 => {
            let cycles_transferred = decode_u128(&mut payload)?;
            if !payload.is_empty() {
                return Err(ReplayDecodeError::DecodeFailed(
                    "root replay compact cycles payload had trailing bytes".to_string(),
                ));
            }
            Ok(Some(Response::Cycles(CyclesResponse {
                cycles_transferred,
            })))
        }
        other => Err(ReplayDecodeError::DecodeFailed(format!(
            "unknown root replay compact variant tag: {other}"
        ))),
    }
}

fn decode_u128(payload: &mut &[u8]) -> Result<u128, ReplayDecodeError> {
    let raw = take_exact(payload, 16, "u128 field")?;
    let mut bytes = [0u8; 16];
    bytes.copy_from_slice(raw);
    Ok(u128::from_be_bytes(bytes))
}

fn take_exact<'a>(
    payload: &mut &'a [u8],
    len: usize,
    context: &'static str,
) -> Result<&'a [u8], ReplayDecodeError> {
    if payload.len() < len {
        return Err(ReplayDecodeError::DecodeFailed(format!(
            "root replay compact payload truncated while reading {context}"
        )));
    }
    let (value, rest) = payload.split_at(len);
    *payload = rest;
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compact_root_replay_round_trips_cycles_response() {
        let response = Response::Cycles(CyclesResponse {
            cycles_transferred: 123_456_789_012_345_678_901_234_567_890u128,
        });
        let encoded = encode_root_replay_response(&response).expect("encode");

        assert!(
            encoded.starts_with(ROOT_REPLAY_COMPACT_TAG),
            "cycles replay should use compact encoding"
        );

        let decoded = decode_root_replay_response(&encoded).expect("decode");
        match (decoded, response) {
            (Response::Cycles(decoded), Response::Cycles(expected)) => {
                assert_eq!(decoded.cycles_transferred, expected.cycles_transferred);
            }
            _ => panic!("expected cycles replay response"),
        }
    }
}
