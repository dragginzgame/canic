//! Exact frontend upload readback through the asset canister query interface.
//!
//! Prepares bounded expected file identities; never uploads or changes asset settings.

use crate::{
    durable_io::read_regular_bytes,
    frontend::{
        FrontendError,
        model::{FrontendManifestRecord, FrontendUploadedInput},
        ops::{MAX_FRONTEND_FILE_BYTES, verify_bundle},
        view::{FrontendAssetChunkView, FrontendUploadView, FrontendUploadedFileView},
    },
    icp::IcpCli,
};
use candid::{CandidType, Deserialize, Nat, Principal};
use canic_core::cdk::utils::hash::sha256_hex;
use std::time::{Duration, Instant};

///
/// FrontendAssetReader
///
/// Query-only boundary for reading exact identity-encoded assets.
///

pub trait FrontendAssetReader {
    fn first(&mut self, key: &str) -> Result<FrontendAssetChunkView, FrontendError>;
    fn next(&mut self, key: &str, digest: &[u8], index: u64) -> Result<Vec<u8>, FrontendError>;
}

/// Build expected remote keys only from a verified local bundle and explicit target.
pub fn prepare_uploaded(
    input: &FrontendUploadedInput,
) -> Result<FrontendUploadView, FrontendError> {
    let manifest = verify_bundle(&input.directory, &input.manifest_sha256)?;
    if manifest.environment != input.environment {
        return Err(FrontendError::Environment);
    }
    if manifest.canonical_network_id != input.network {
        return Err(FrontendError::Environment);
    }
    if manifest
        .asset
        .as_ref()
        .is_none_or(|asset| asset.canister_id != input.canister_id)
    {
        return Err(FrontendError::Principal);
    }
    validate_prefix(&input.prefix)?;
    let prefix = input.prefix.trim_end_matches('/');
    let mut files = super::bundle::manifest_files(&manifest)
        .into_iter()
        .map(|file| FrontendUploadedFileView {
            key: if file.path == manifest.alternative_origins.path {
                format!("/{}", file.path)
            } else {
                format!("{prefix}/{}", file.path)
            },
            bytes: file.bytes,
            sha256: file.sha256.clone(),
        })
        .collect::<Vec<_>>();
    let bytes = read_regular_bytes(
        &input.directory.join("canic-frontend.json"),
        MAX_FRONTEND_FILE_BYTES,
    )?;
    if serde_json::from_slice::<FrontendManifestRecord>(&bytes)? != manifest {
        return Err(FrontendError::Integrity);
    }
    files.push(FrontendUploadedFileView {
        key: format!("{prefix}/canic-frontend.json"),
        bytes: bytes.len() as u64,
        sha256: sha256_hex(&bytes),
    });
    Ok(FrontendUploadView {
        environment: input.environment.clone(),
        canister_id: input.canister_id,
        manifest_sha256: input.manifest_sha256.clone(),
        files,
    })
}

fn validate_prefix(prefix: &str) -> Result<(), FrontendError> {
    if prefix == "/" {
        return Ok(());
    }
    let relative = prefix.strip_prefix('/').ok_or(FrontendError::AssetPrefix)?;
    for part in relative.split('/') {
        if matches!(part, "" | "." | "..") {
            return Err(FrontendError::AssetPrefix);
        }
        if !part
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
        {
            return Err(FrontendError::AssetPrefix);
        }
    }
    Ok(())
}

///
/// IcpFrontendAssetReader
///
/// Live query adapter with a finite readback deadline and bounded Candid decoding.
///

pub struct IcpFrontendAssetReader {
    agent: ic_agent::Agent,
    canister: Principal,
    runtime: tokio::runtime::Runtime,
    deadline: Instant,
}

impl IcpFrontendAssetReader {
    pub fn new(icp: &IcpCli, input: &FrontendUploadedInput) -> Result<Self, FrontendError> {
        if icp.environment() != Some(input.environment.as_str()) {
            return Err(FrontendError::Environment);
        }
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        let agent = icp
            .authenticated_agent_with_response_limit(8 * 1024 * 1024)
            .map_err(Box::new)?;
        let network =
            canic_core::ids::CanonicalNetworkId::from_der_root_trust_anchor(&agent.read_root_key())
                .map_err(|_| FrontendError::Environment)?;
        if network != input.network {
            return Err(FrontendError::Environment);
        }
        Ok(Self {
            agent,
            canister: input.canister_id,
            runtime,
            deadline: Instant::now() + Duration::from_secs(120),
        })
    }

    fn query<I: CandidType, O: CandidType + serde::de::DeserializeOwned>(
        &self,
        method: &str,
        input: &I,
    ) -> Result<O, FrontendError> {
        let argument = candid::encode_one(input).map_err(|_| FrontendError::AssetResponse)?;
        let remaining = self
            .deadline
            .checked_duration_since(Instant::now())
            .ok_or(FrontendError::AssetTimeout)?;
        let response = self
            .runtime
            .block_on(async {
                tokio::time::timeout(
                    remaining.min(Duration::from_secs(30)),
                    self.agent
                        .query(&self.canister, method)
                        .with_arg(argument)
                        .call(),
                )
                .await
            })
            .map_err(|_| FrontendError::AssetTimeout)?
            .map_err(|error| FrontendError::AssetQuery(Box::new(error)))?;
        if response.len() > MAX_FRONTEND_FILE_BYTES + 4096 {
            return Err(FrontendError::Bound("asset response bytes"));
        }
        let mut config = candid::de::DecoderConfig::new();
        config.set_decoding_quota((MAX_FRONTEND_FILE_BYTES + 4096) * 64);
        config.set_skipping_quota(MAX_FRONTEND_FILE_BYTES + 4096);
        candid::utils::decode_one_with_config(&response, &config)
            .map_err(|_| FrontendError::AssetResponse)
    }
}

#[derive(CandidType)]
struct GetAsset {
    key: String,
    accept_encodings: Vec<String>,
}
#[derive(CandidType, Deserialize)]
struct AssetReply {
    content: Vec<u8>,
    total_length: Nat,
    content_encoding: String,
    sha256: Option<Vec<u8>>,
}
#[derive(CandidType)]
struct GetChunk {
    key: String,
    content_encoding: String,
    index: Nat,
    sha256: Option<Vec<u8>>,
}
#[derive(CandidType, Deserialize)]
struct ChunkReply {
    content: Vec<u8>,
}

impl FrontendAssetReader for IcpFrontendAssetReader {
    fn first(&mut self, key: &str) -> Result<FrontendAssetChunkView, FrontendError> {
        let reply: AssetReply = self.query(
            "get",
            &GetAsset {
                key: key.into(),
                accept_encodings: vec!["identity".into()],
            },
        )?;
        Ok(FrontendAssetChunkView {
            content: reply.content,
            total_length: reply
                .total_length
                .0
                .try_into()
                .map_err(|_| FrontendError::AssetResponse)?,
            encoding: reply.content_encoding,
            sha256: reply.sha256,
        })
    }
    fn next(&mut self, key: &str, digest: &[u8], index: u64) -> Result<Vec<u8>, FrontendError> {
        let reply: ChunkReply = self.query(
            "get_chunk",
            &GetChunk {
                key: key.into(),
                content_encoding: "identity".into(),
                index: index.into(),
                sha256: Some(digest.to_vec()),
            },
        )?;
        Ok(reply.content)
    }
}
