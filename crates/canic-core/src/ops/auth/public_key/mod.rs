//! Bind Root public-key requests to the local canister before management execution.
//!
//! This operation derives public material; it never requests an ECDSA signature.

use crate::{
    InternalError,
    dto::auth::RootChainKeyPublicKeyRequest,
    ops::{
        auth::AuthOps,
        ic::{
            IcOps,
            mgmt::{EcdsaKeyId, EcdsaPublicKeyArgs, MgmtOps},
        },
    },
};

impl AuthOps {
    /// Convert the public request and derive only this canister's key.
    pub(crate) async fn root_chain_key_public_key(
        request: RootChainKeyPublicKeyRequest,
    ) -> Result<Vec<u8>, InternalError> {
        let result = MgmtOps::ecdsa_public_key(&EcdsaPublicKeyArgs {
            canister_id: Some(IcOps::canister_self()),
            derivation_path: request.derivation_path,
            key_id: EcdsaKeyId {
                name: request.key_id,
            },
        })
        .await?;
        Ok(result.public_key)
    }
}
