//! Certified physical canister identity observed at the current IC boundary.

use candid::Principal;
use canic_contracts::ids::SubnetId;

///
/// CertifiedCanisterCustodyView
///
/// One verified certificate's controller, code and subnet observations. This is
/// a time-local custody sample; it contains no cycle balance or wipe permission.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CertifiedCanisterCustodyView {
    pub(in crate::fleet_ensure) principal: Principal,
    pub(in crate::fleet_ensure) subnet: SubnetId,
    pub(in crate::fleet_ensure) controllers: Vec<Principal>,
    pub(in crate::fleet_ensure) module_sha256: Option<String>,
    pub(in crate::fleet_ensure) certificate_tree_sha256: [u8; 32],
}

impl CertifiedCanisterCustodyView {
    /// Physical identity authenticated by the certificate delegation.
    #[must_use]
    pub const fn principal(&self) -> Principal {
        self.principal
    }
    /// Observed subnet, independently checked against source placement.
    #[must_use]
    pub const fn subnet(&self) -> SubnetId {
        self.subnet
    }
    /// Exact observed controllers, including every additional controller.
    #[must_use]
    pub fn controllers(&self) -> &[Principal] {
        &self.controllers
    }
    /// Installed module, or certified absence for an empty canister.
    #[must_use]
    pub fn module_sha256(&self) -> Option<&str> {
        self.module_sha256.as_deref()
    }
    /// Certified tree root binds the observed fields and IC certificate time.
    #[must_use]
    pub const fn certificate_tree_sha256(&self) -> [u8; 32] {
        self.certificate_tree_sha256
    }
}
