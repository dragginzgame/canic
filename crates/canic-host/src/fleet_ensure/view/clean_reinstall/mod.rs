//! Current clean-reset phase reports, without mutation authority.

use crate::fleet_ensure::model::{FleetEnsureReport, capacity_import::CapacityImportJournalRecord};

/// Current phase review or completion; infrastructure and import never claim workload readiness.
#[derive(Debug)]
pub enum CleanReinstallReport {
    Infrastructure(Box<FleetEnsureReport>),
    Import(Box<CapacityImportJournalRecord>),
    Fleet(Box<FleetEnsureReport>),
}
