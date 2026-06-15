use super::*;
use crate::deployment_truth::observe::{
    apply_canister_control_to_observed_pool, apply_live_status_to_registry_observation,
    observed_root_from_status, registry_entries_to_observed_canisters,
    registry_entries_to_observed_pool,
};
use crate::deployment_truth::report::{RootSubnetEvidence, RootSubnetEvidenceSource};
use crate::icp::{IcpCanisterStatusReport, IcpCanisterStatusSettings};
use crate::install_root::{InstallState, RootVerificationStatus};
use crate::registry::RegistryEntry;
use crate::release_set::{ConfiguredPoolExpectation, ROOT_RELEASE_SET_MANIFEST_FILE};
use crate::test_support::temp_dir;
use serde::Serialize;
use std::fs;

mod support;
use support::*;

mod authority;
mod comparison;
mod core;
mod diff;
mod execution_receipts;
mod lifecycle;
mod local_observation_plan;
mod promotion;
mod root_verification;
