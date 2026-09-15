//! Readback orchestration for the verified frontend handoff.

use crate::frontend::{
    FrontendError,
    model::FrontendUploadedInput,
    ops::{FrontendAssetReader, prepare_uploaded},
    view::FrontendUploadView,
};
use canic_core::cdk::utils::hash::{decode_hex, sha256_hex};

/// Verify every declared handoff file using queries only; immediate repeats are effect-free.
pub fn verify_uploaded(
    input: &FrontendUploadedInput,
    reader: &mut impl FrontendAssetReader,
) -> Result<FrontendUploadView, FrontendError> {
    let expected = prepare_uploaded(input)?;
    for file in &expected.files {
        let digest = decode_hex(&file.sha256).map_err(|_| FrontendError::Integrity)?;
        let first = reader.first(&file.key)?;
        if first.total_length != file.bytes
            || first.encoding != "identity"
            || first.sha256.as_deref() != Some(digest.as_slice())
            || first.content.len() as u64 > file.bytes
        {
            return Err(FrontendError::AssetMismatch {
                key: file.key.clone(),
            });
        }
        let mut bytes = first.content;
        let chunk_size = bytes.len();
        let mut index = 1;
        while (bytes.len() as u64) < file.bytes {
            if chunk_size == 0 || index >= 64 {
                return Err(FrontendError::Bound("asset chunks"));
            }
            let chunk = reader.next(&file.key, &digest, index)?;
            let remaining = usize::try_from(file.bytes)
                .map_err(|_| FrontendError::Bound("asset bytes"))?
                .saturating_sub(bytes.len());
            if chunk.len() != remaining.min(chunk_size) {
                return Err(FrontendError::AssetMismatch {
                    key: file.key.clone(),
                });
            }
            bytes.extend_from_slice(&chunk);
            index += 1;
        }
        if sha256_hex(&bytes) != file.sha256 {
            return Err(FrontendError::AssetMismatch {
                key: file.key.clone(),
            });
        }
    }
    Ok(expected)
}
