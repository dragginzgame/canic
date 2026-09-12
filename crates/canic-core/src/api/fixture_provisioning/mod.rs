//! Module: api::fixture_provisioning
//!
//! Responsibility: expose the registered application importer and bounded consumer step.
//! Does not own: endpoint authentication, database writes or autonomous retry scheduling.
//! Boundary: delegates delivery and receipt observation to the provisioning workflow.

use crate::dto::fixture_provisioning::{FixtureImportError, FixtureProvisioningStatus};
pub use crate::model::fixture_importer::FixtureImporter;

/// Application-facing fixture consumer; call registration from the synchronous lifecycle participant.
pub struct FixtureProvisioningApi;

impl FixtureProvisioningApi {
    /// Observe the existing fetch lease in an internal qualification canister.
    #[cfg(feature = "internal-test-fixtures")]
    #[must_use]
    pub fn fetch_in_flight() -> bool {
        crate::workflow::fixture_provisioning::fetch_in_flight()
    }

    /// Register one application participant after database restoration on each fresh heap.
    pub fn register(importer: &'static dyn FixtureImporter) -> Result<(), FixtureImportError> {
        crate::workflow::fixture_provisioning::register(importer)
    }

    /// Observe the selected data prerequisite and exact application-owned completion receipt.
    pub fn status() -> Result<FixtureProvisioningStatus, FixtureImportError> {
        crate::workflow::fixture_provisioning::status()
    }
}
