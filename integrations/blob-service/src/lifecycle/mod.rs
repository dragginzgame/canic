//! Module: blob_service::lifecycle
//!
//! Responsibility: bound the nested service configuration after Canic admits the envelope.
//! Does not own: framework restoration, memory bootstrap or deferred service work.
//! Boundary: synchronous lifecycle participants delegate installation and restoration.

use canic::endpoint::ArgumentLimits;
use ic_blob_storage::dto::configuration::ServiceInstallationInput;

pub(crate) const REQUEST_LIMITS: ArgumentLimits = ArgumentLimits {
    max_bytes: 4096,
    decoding_quota: 2_000_000,
    skipping_quota: 1024,
    max_type_len: 64,
    max_header_len: 4096,
};
pub(crate) const MANIFEST_LIMITS: ArgumentLimits = ArgumentLimits {
    max_bytes: 131_072,
    ..REQUEST_LIMITS
};
const CONFIGURATION_LIMITS: ArgumentLimits = ArgumentLimits {
    max_bytes: 16_384,
    ..REQUEST_LIMITS
};
pub(crate) const ENVELOPE_LIMITS: ArgumentLimits = ArgumentLimits {
    max_bytes: 65_536,
    max_type_len: 512,
    max_header_len: 16_384,
    ..REQUEST_LIMITS
};

pub(crate) fn install() {
    // Canic has already admitted this envelope before restoring its invariants.
    // The participant has no argument parameter: re-read only under the same
    // raw-byte/decoder bounds, then decode the nested service bytes once.
    let (_, application): (canic::dto::abi::v1::CanisterInitPayload, Option<Vec<u8>>) =
        ENVELOPE_LIMITS.read().expect("admitted lifecycle envelope");
    let (input,): (ServiceInstallationInput,) = CONFIGURATION_LIMITS
        .decode(
            application
                .as_deref()
                .expect("service installation arguments"),
        )
        .expect("bounded service configuration");
    crate::workflow::install(&input);
}
