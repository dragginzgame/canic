//! One bounded cache repair. Stable bytes and pins survive callback loss; cache writes are idempotent.
use super::{STATE, cdk, protocol};
use ic_platform::{
    candid_codec::{self, decoder},
    management_canister_types::{CanisterIdRecord, ChunkHash, UploadChunkArgs},
};
use ic_platform_runtime::{
    Principal,
    call::{Call, CallFailed, WaitPolicy},
};
use protocol::{CacheError, CacheReady, PrepareCache, Read};

const REPLY_LIMITS: decoder::Limits = decoder::Limits {
    bytes: 65_536,
    header: 4096,
    types: 256,
    work: 131_072,
    allocation: 262_144,
};
fn admit() -> Result<(), String> {
    if !cdk::api::is_controller(&cdk::api::msg_caller()) {
        return Err("Store controller required".into());
    }
    if ic_platform_runtime::ic0::msg_arg_data_size() > protocol::LIMITS.bytes {
        return Err("Store frame too large".into());
    }
    Ok(())
}
#[expect(
    clippy::needless_pass_by_value,
    reason = "Platform owns the argument buffer"
)]
fn decode(bytes: Vec<u8>) -> PrepareCache {
    ic_wire::decode(
        &bytes,
        ic_wire::Encoding::Frame {
            method: protocol::PREPARE_CACHE,
            direction: ic_wire::Direction::Request,
        },
        protocol::LIMITS,
    )
    .expect("cache request")
}
#[expect(clippy::needless_pass_by_value, reason = "Platform owns the reply")]
fn encode(reply: Result<CacheReady, CacheError>) -> Vec<u8> {
    ic_wire::encode(
        &reply,
        ic_wire::Encoding::Frame {
            method: protocol::PREPARE_CACHE,
            direction: ic_wire::Direction::Response,
        },
        protocol::LIMITS,
    )
    .expect("cache reply")
}
fn authority(caller: &Principal, expires: u64, epoch: u64) -> Result<(), CacheError> {
    if !cdk::api::is_controller(caller) {
        return Err(CacheError::Authority);
    }
    if cdk::api::time() > expires {
        return Err(CacheError::Expired);
    }
    STATE
        .with_borrow(|state| state.as_ref().expect("Store state").cache_fence(epoch))
        .map_err(CacheError::Pin)
}
async fn management(method: &str, bytes: Vec<u8>) -> Result<Vec<u8>, CacheError> {
    let reply = Call::new(
        Principal::management_canister(),
        method,
        WaitPolicy::Unbounded,
    )
    .take_raw_args(bytes)
    .execute()
    .await
    .map_err(|error| match error {
        CallFailed::CallRejected(rejection) => CacheError::Rejected(rejection.raw_reject_code()),
        _ => CacheError::Uncertain,
    })?;
    Ok(reply.to_vec())
}

#[cdk::update(guard = "admit", decode_with = "decode", encode_with = "encode")]
async fn prepare_cache(input: PrepareCache) -> Result<CacheReady, CacheError> {
    let caller = cdk::api::msg_caller();
    let epoch = STATE
        .with_borrow(|state| state.as_ref().expect("Store state").admission_epoch())
        .map_err(CacheError::Pin)?;
    authority(&caller, input.expires_at_ns, epoch)?;
    if input.expires_at_ns.saturating_sub(cdk::api::time()) > 300_000_000_000 {
        return Err(CacheError::Invalid);
    }
    STATE
        .with_borrow(|state| {
            state
                .as_ref()
                .expect("Store state")
                .observe_pin(caller.as_slice(), &input.pin)
        })
        .map_err(CacheError::Pin)?;
    let canister_id = cdk::api::canister_self();
    let args = candid_codec::encode_one(&CanisterIdRecord { canister_id }, 1024)
        .expect("chunk observation args");
    let reply = management("stored_chunks", args).await?;
    let mut chunks: Vec<ChunkHash> =
        decoder::decode_one(&reply, REPLY_LIMITS).map_err(|_| CacheError::Uncertain)?;
    let mut uploaded = 0;
    for (index, hash) in input.pin.manifest.chunks.iter().enumerate() {
        authority(&caller, input.expires_at_ns, epoch)?;
        if chunks.iter().any(|chunk| chunk.hash == *hash) {
            continue;
        }
        let bytes = STATE
            .with_borrow(|state| {
                let state = state.as_ref().expect("Store state");
                state.observe_pin(caller.as_slice(), &input.pin)?;
                state.read(&Read {
                    release: input.pin.manifest.release.clone(),
                    index: index as u32,
                })
            })
            .map_err(CacheError::Pin)?
            .0;
        let args = candid_codec::encode_one(
            &UploadChunkArgs {
                canister_id,
                chunk: bytes,
            },
            protocol::CHUNK_LIMITS.bytes,
        )
        .expect("chunk upload args");
        let reply = management("upload_chunk", args).await?;
        let result: ChunkHash =
            decoder::decode_one(&reply, REPLY_LIMITS).map_err(|_| CacheError::Uncertain)?;
        if result.hash != *hash {
            return Err(CacheError::Uncertain);
        }
        chunks.push(result);
        uploaded += 1;
    }
    // A close/reopen cycle fences even the final callback; it cannot report fresh readiness.
    STATE
        .with_borrow(|state| state.as_ref().expect("Store state").cache_fence(epoch))
        .map_err(CacheError::Pin)?;
    // Late completion remains evidence, never permission to install past the approval deadline.
    Ok(CacheReady {
        pin: input.pin,
        uploaded,
    })
}
