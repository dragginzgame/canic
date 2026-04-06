use crate::cdk::types::Principal;
use crate::dto::{
    auth::{
        DelegationProof, DelegationProvisionResponse, DelegationProvisionStatus,
        DelegationProvisionTargetKind, DelegationProvisionTargetResponse,
    },
    rpc::{CyclesResponse, Response},
};
use candid::{decode_one, encode_one};

use self::{guard::ReplayPending, slot as replay_slot};

pub mod guard;
pub mod key;
pub mod slot;
pub mod ttl;

const ROOT_REPLAY_COMPACT_TAG: &[u8] = b"RR2";
const ROOT_REPLAY_COMPACT_CYCLES_V1: u8 = 0;
const ROOT_REPLAY_COMPACT_DELEGATION_ISSUED_V1: u8 = 1;

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
    if let Response::Cycles(CyclesResponse { cycles_transferred }) = response {
        let payload = cycles_transferred.to_be_bytes();
        let mut bytes = Vec::with_capacity(ROOT_REPLAY_COMPACT_TAG.len() + 1 + payload.len());
        bytes.extend_from_slice(ROOT_REPLAY_COMPACT_TAG);
        bytes.push(ROOT_REPLAY_COMPACT_CYCLES_V1);
        bytes.extend_from_slice(&payload);
        return Some(bytes);
    }

    let Response::DelegationIssued(DelegationProvisionResponse { proof, results }) = response
    else {
        return None;
    };

    let mut verifier_targets = Vec::with_capacity(results.len());
    for result in results {
        if !matches!(
            result,
            DelegationProvisionTargetResponse {
                kind: DelegationProvisionTargetKind::Verifier,
                status: DelegationProvisionStatus::Ok,
                error: None,
                ..
            }
        ) {
            return None;
        }
        verifier_targets.push(result.target);
    }

    let compact = ReplayDelegationIssuedCompactV1 {
        proof,
        verifier_targets,
    };
    let payload = encode_compact_delegation_issued(&compact);

    let mut bytes = Vec::with_capacity(ROOT_REPLAY_COMPACT_TAG.len() + 1 + payload.len());
    bytes.extend_from_slice(ROOT_REPLAY_COMPACT_TAG);
    bytes.push(ROOT_REPLAY_COMPACT_DELEGATION_ISSUED_V1);
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
        ROOT_REPLAY_COMPACT_DELEGATION_ISSUED_V1 => {
            let compact = decode_compact_delegation_issued(payload)?;
            let results = compact
                .verifier_targets
                .into_iter()
                .map(|target| DelegationProvisionTargetResponse {
                    target,
                    kind: DelegationProvisionTargetKind::Verifier,
                    status: DelegationProvisionStatus::Ok,
                    error: None,
                })
                .collect();
            Ok(Some(Response::DelegationIssued(
                DelegationProvisionResponse {
                    proof: compact.proof,
                    results,
                },
            )))
        }
        other => Err(ReplayDecodeError::DecodeFailed(format!(
            "unknown root replay compact variant tag: {other}"
        ))),
    }
}

struct ReplayDelegationIssuedCompactV1<'a> {
    proof: &'a DelegationProof,
    verifier_targets: Vec<Principal>,
}

struct ReplayDelegationIssuedCompactOwned {
    proof: DelegationProof,
    verifier_targets: Vec<Principal>,
}

fn encode_compact_delegation_issued(compact: &ReplayDelegationIssuedCompactV1<'_>) -> Vec<u8> {
    let mut bytes = Vec::new();
    encode_principal(&mut bytes, compact.proof.cert.root_pid);
    encode_principal(&mut bytes, compact.proof.cert.shard_pid);
    bytes.extend_from_slice(&compact.proof.cert.issued_at.to_be_bytes());
    bytes.extend_from_slice(&compact.proof.cert.expires_at.to_be_bytes());
    encode_string_vec(&mut bytes, &compact.proof.cert.scopes);
    encode_principal_vec(&mut bytes, &compact.proof.cert.aud);
    encode_bytes(&mut bytes, &compact.proof.cert_sig);
    encode_principal_vec(&mut bytes, &compact.verifier_targets);
    bytes
}

fn decode_compact_delegation_issued(
    mut payload: &[u8],
) -> Result<ReplayDelegationIssuedCompactOwned, ReplayDecodeError> {
    let root_pid = decode_principal(&mut payload)?;
    let shard_pid = decode_principal(&mut payload)?;
    let issued_at = decode_u64(&mut payload)?;
    let expires_at = decode_u64(&mut payload)?;
    let scopes = decode_string_vec(&mut payload)?;
    let aud = decode_principal_vec(&mut payload)?;
    let cert_sig = decode_bytes(&mut payload)?;
    let verifier_targets = decode_principal_vec(&mut payload)?;
    if !payload.is_empty() {
        return Err(ReplayDecodeError::DecodeFailed(
            "root replay compact payload had trailing bytes".to_string(),
        ));
    }

    Ok(ReplayDelegationIssuedCompactOwned {
        proof: DelegationProof {
            cert: crate::dto::auth::DelegationCert {
                root_pid,
                shard_pid,
                issued_at,
                expires_at,
                scopes,
                aud,
            },
            cert_sig,
        },
        verifier_targets,
    })
}

fn encode_principal(bytes: &mut Vec<u8>, principal: Principal) {
    encode_bytes(bytes, principal.as_slice());
}

fn encode_principal_vec(bytes: &mut Vec<u8>, principals: &[Principal]) {
    encode_len(bytes, principals.len());
    for principal in principals {
        encode_principal(bytes, *principal);
    }
}

fn encode_string_vec(bytes: &mut Vec<u8>, values: &[String]) {
    encode_len(bytes, values.len());
    for value in values {
        encode_bytes(bytes, value.as_bytes());
    }
}

fn encode_bytes(bytes: &mut Vec<u8>, value: &[u8]) {
    encode_len(bytes, value.len());
    bytes.extend_from_slice(value);
}

fn encode_len(bytes: &mut Vec<u8>, len: usize) {
    let len = u32::try_from(len).expect("root replay compact field length overflow");
    bytes.extend_from_slice(&len.to_be_bytes());
}

fn decode_u64(payload: &mut &[u8]) -> Result<u64, ReplayDecodeError> {
    let raw = take_exact(payload, 8, "u64 field")?;
    let mut bytes = [0u8; 8];
    bytes.copy_from_slice(raw);
    Ok(u64::from_be_bytes(bytes))
}

fn decode_u128(payload: &mut &[u8]) -> Result<u128, ReplayDecodeError> {
    let raw = take_exact(payload, 16, "u128 field")?;
    let mut bytes = [0u8; 16];
    bytes.copy_from_slice(raw);
    Ok(u128::from_be_bytes(bytes))
}

fn decode_principal(payload: &mut &[u8]) -> Result<Principal, ReplayDecodeError> {
    let raw = decode_bytes(payload)?;
    Ok(Principal::from_slice(&raw))
}

fn decode_principal_vec(payload: &mut &[u8]) -> Result<Vec<Principal>, ReplayDecodeError> {
    let count = decode_len(payload, "principal vector length")?;
    let mut principals = Vec::with_capacity(count);
    for _ in 0..count {
        principals.push(decode_principal(payload)?);
    }
    Ok(principals)
}

fn decode_string_vec(payload: &mut &[u8]) -> Result<Vec<String>, ReplayDecodeError> {
    let count = decode_len(payload, "string vector length")?;
    let mut values = Vec::with_capacity(count);
    for _ in 0..count {
        let raw = decode_bytes(payload)?;
        let value = String::from_utf8(raw)
            .map_err(|err| ReplayDecodeError::DecodeFailed(format!("invalid utf-8: {err}")))?;
        values.push(value);
    }
    Ok(values)
}

fn decode_bytes(payload: &mut &[u8]) -> Result<Vec<u8>, ReplayDecodeError> {
    let len = decode_len(payload, "byte field length")?;
    Ok(take_exact(payload, len, "byte field")?.to_vec())
}

fn decode_len(payload: &mut &[u8], context: &'static str) -> Result<usize, ReplayDecodeError> {
    let raw = take_exact(payload, 4, context)?;
    let mut bytes = [0u8; 4];
    bytes.copy_from_slice(raw);
    Ok(u32::from_be_bytes(bytes) as usize)
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
    use crate::dto::{
        auth::DelegationCert,
        error::{Error, ErrorCode},
    };

    fn p(id: u8) -> Principal {
        Principal::from_slice(&[id; 29])
    }

    fn delegation_response_ok() -> Response {
        Response::DelegationIssued(DelegationProvisionResponse {
            proof: DelegationProof {
                cert: DelegationCert {
                    root_pid: p(1),
                    shard_pid: p(2),
                    issued_at: 11,
                    expires_at: 22,
                    scopes: vec!["verify".to_string()],
                    aud: vec![p(3)],
                },
                cert_sig: vec![7, 8, 9],
            },
            results: vec![DelegationProvisionTargetResponse {
                target: p(3),
                kind: DelegationProvisionTargetKind::Verifier,
                status: DelegationProvisionStatus::Ok,
                error: None,
            }],
        })
    }

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

    #[test]
    fn compact_root_replay_round_trips_successful_delegation_response() {
        let response = delegation_response_ok();
        let encoded = encode_root_replay_response(&response).expect("encode");

        assert!(
            encoded.starts_with(ROOT_REPLAY_COMPACT_TAG),
            "successful delegation replay should use compact encoding"
        );

        let decoded = decode_root_replay_response(&encoded).expect("decode");
        match (decoded, response) {
            (Response::DelegationIssued(decoded), Response::DelegationIssued(expected)) => {
                assert_eq!(decoded.proof, expected.proof);
                assert_eq!(decoded.results.len(), expected.results.len());
                for (decoded, expected) in decoded.results.iter().zip(expected.results.iter()) {
                    assert_eq!(decoded.target, expected.target);
                    assert_eq!(decoded.kind, expected.kind);
                    assert_eq!(decoded.status, expected.status);
                    assert_eq!(
                        decoded.error.as_ref().map(|err| err.code),
                        expected.error.as_ref().map(|err| err.code)
                    );
                    assert_eq!(
                        decoded.error.as_ref().map(|err| err.message.as_str()),
                        expected.error.as_ref().map(|err| err.message.as_str())
                    );
                }
            }
            _ => panic!("expected delegation-issued replay response"),
        }
    }

    #[test]
    fn root_replay_falls_back_to_generic_encoding_for_failed_delegation_results() {
        let response = Response::DelegationIssued(DelegationProvisionResponse {
            proof: match delegation_response_ok() {
                Response::DelegationIssued(response) => response.proof,
                _ => unreachable!(),
            },
            results: vec![DelegationProvisionTargetResponse {
                target: p(3),
                kind: DelegationProvisionTargetKind::Verifier,
                status: DelegationProvisionStatus::Failed,
                error: Some(Error::new(ErrorCode::Internal, "push failed".to_string())),
            }],
        });

        let encoded = encode_root_replay_response(&response).expect("encode");
        assert!(
            !encoded.starts_with(ROOT_REPLAY_COMPACT_TAG),
            "failed delegation replay must keep generic encoding"
        );

        let decoded = decode_root_replay_response(&encoded).expect("decode");
        match (decoded, response) {
            (Response::DelegationIssued(decoded), Response::DelegationIssued(expected)) => {
                assert_eq!(decoded.proof, expected.proof);
                assert_eq!(decoded.results.len(), expected.results.len());
                for (decoded, expected) in decoded.results.iter().zip(expected.results.iter()) {
                    assert_eq!(decoded.target, expected.target);
                    assert_eq!(decoded.kind, expected.kind);
                    assert_eq!(decoded.status, expected.status);
                    assert_eq!(
                        decoded.error.as_ref().map(|err| err.code),
                        expected.error.as_ref().map(|err| err.code)
                    );
                    assert_eq!(
                        decoded.error.as_ref().map(|err| err.message.as_str()),
                        expected.error.as_ref().map(|err| err.message.as_str())
                    );
                }
            }
            _ => panic!("expected delegation-issued replay response"),
        }
    }
}
