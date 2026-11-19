//! Original-request signing and the optional Root exchange for Backup.
mod recovery;
mod root;

use ic_backup::store::{BackupSigner, OperationSigningContent, OperationSigningRequest};
use ic_host_sign::{
    Error, Signer,
    protocol::{Operation, Request},
};
pub use recovery::{CounterRecoveryGuard, recovery_request};
pub use root::SnapshotCodec;

/// Adapts the selected external provider to Backup's original, bounded signing requests.
/// Backup compares this provider commitment with its retained plan before each invocation.
///
/// Select the provider by its trusted executable hash and public identity profile.
/// Pass the adapter to Backup with the application's guard and transport:
///
/// ```
/// # use canic_host::backup::DirectSigner;
/// # use ic_backup::store::{BackupApplicationGuard, BackupConnection, BackupHttpTransport};
/// # use ic_host_sign::{Error, Signer, protocol::Configuration};
/// # use std::path::Path;
/// # fn connect<G: BackupApplicationGuard>(
/// #     executable: &Path, approved_hash: [u8; 32], identity: Configuration,
/// #     transport: BackupHttpTransport, guard: G,
/// # ) -> Result<BackupConnection<DirectSigner, G>, Error> {
/// let signer = DirectSigner(Signer::new(executable, approved_hash, identity)?);
/// let connection = BackupConnection::new(transport, signer, guard);
/// # Ok(connection)
/// # }
/// ```
pub struct DirectSigner(pub Signer);

impl BackupSigner for DirectSigner {
    type Error = Error;

    fn protocol(&self) -> [u8; 32] {
        use sha2::{Digest, Sha256};
        Sha256::new()
            .chain_update(b"canic/backup-direct-signer/v1\0")
            .chain_update(self.0.commitment())
            .finalize()
            .into()
    }

    async fn sign(&mut self, request: OperationSigningRequest<'_>) -> Result<Vec<u8>, Error> {
        let operation = match request.signing {
            OperationSigningContent::Call { method, arguments } => {
                if arguments.len() > request.envelope_bytes as usize {
                    return Err(Error::Request);
                }
                Operation::Update {
                    recipient: request.plan.ingress_recipient().to_text(),
                    method: method.to_owned(),
                    arg: arguments.to_vec(),
                    nonce: Some(request.plan.digest().as_bytes().to_vec()),
                }
            }
            OperationSigningContent::Observation { request_id } => Operation::Status { request_id },
        };
        let exact = Request {
            sender: request.plan.sender().to_text(),
            effective: request
                .plan
                .root_manager()
                .map_or(request.plan.target().canister, |manager| manager.root)
                .to_text(),
            expires_ns: request.expires_ns,
            deadline_ns: request.deadline_ns,
            envelope_bytes: request.envelope_bytes as usize,
            network_requests: u64::from(request.network_requests),
            operation,
        };
        let signed = self.0.sign(exact, self.0.commitment()).await?;
        Ok(signed.envelope_bytes().to_vec())
    }
}
