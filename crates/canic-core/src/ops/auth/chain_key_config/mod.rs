//! Shared chain-key configuration admission for signing and verification.

use crate::{InternalError, cdk::utils::hash::decode_hex, ops::auth::AuthValidationError};

/// Caller context for otherwise identical chain-key field admission.
pub(in crate::ops::auth) struct ChainKeyConfigFields {
    pub(in crate::ops::auth) purpose: &'static str,
}

impl ChainKeyConfigFields {
    pub(in crate::ops::auth) fn required_chain_key_field<'a>(
        &self,
        value: Option<&'a str>,
        field: &'static str,
    ) -> Result<&'a str, InternalError> {
        let Some(value) = value else {
            return Err(AuthValidationError::Auth(format!(
                "auth.delegated_tokens.chain_key_root_proof.{field} is required for {}",
                self.purpose
            ))
            .into());
        };
        let value = value.trim();
        if value.is_empty() {
            return Err(AuthValidationError::Auth(format!(
                "auth.delegated_tokens.chain_key_root_proof.{field} must not be empty"
            ))
            .into());
        }
        Ok(value)
    }
    pub(in crate::ops::auth) fn required_chain_key_u64(
        &self,
        value: Option<u64>,
        field: &'static str,
    ) -> Result<u64, InternalError> {
        value.ok_or_else(|| {
            AuthValidationError::Auth(format!(
                "auth.delegated_tokens.chain_key_root_proof.{field} is required for {}",
                self.purpose
            ))
            .into()
        })
    }
    pub(in crate::ops::auth) fn required_fixed_32_chain_key_hex(
        &self,
        value: Option<&str>,
        field: &'static str,
    ) -> Result<[u8; 32], InternalError> {
        let value = self.required_chain_key_field(value, field)?;
        let decoded = decode_hex(value).map_err(|err| {
            AuthValidationError::Auth(format!(
                "auth.delegated_tokens.chain_key_root_proof.{field} is not valid hex: {err}"
            ))
        })?;
        decoded.try_into().map_err(|decoded: Vec<u8>| {
            AuthValidationError::Auth(format!(
            "auth.delegated_tokens.chain_key_root_proof.{field} must decode to 32 bytes, got {}",
            decoded.len()
        ))
        .into()
        })
    }
}
