use super::*;
use crate::deployment_truth::report::{RootSubnetEvidence, RootSubnetEvidenceSource};
use crate::fleet_catalog::{FleetCatalogEntryV1, FleetCatalogError};
use crate::release_set::ROOT_RELEASE_SET_MANIFEST_FILE;
use crate::test_support::temp_dir;
use canic_core::ids::{AppId, CanonicalNetworkId, FleetId};
use serde::Serialize;
use std::fs;

mod support;
use support::*;

mod comparison;
mod core;
mod diff;
mod execution_receipts;
mod local_observation_plan;
