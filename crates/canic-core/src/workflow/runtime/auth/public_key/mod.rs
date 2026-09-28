//! Route authenticated Root public-key discovery through auth operations.
//!
//! The endpoint owns authorization; this workflow grants no signing authority.

use crate::{
    InternalError, dto::auth::RootChainKeyPublicKeyRequest, ops::auth::AuthOps,
    workflow::runtime::auth::RuntimeAuthWorkflow,
};

impl RuntimeAuthWorkflow {
    /// Derive public verification material bound to this Root.
    pub async fn root_chain_key_public_key(
        request: RootChainKeyPublicKeyRequest,
    ) -> Result<Vec<u8>, InternalError> {
        AuthOps::root_chain_key_public_key(request).await
    }
}
