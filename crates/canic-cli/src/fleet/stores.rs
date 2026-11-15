//! Shared wording for terminal and localhost Store-maintenance diagnostics.
use canic_host::FleetStoreMaintenance;
use canic_management_protocol::store_maintenance::{MAX_UPGRADES, Phase};

pub(super) fn summary(report: &FleetStoreMaintenance) -> String {
    if report.observation_error.is_some() {
        return "UNKNOWN".into();
    }
    let Some(status) = &report.reported else {
        return "no retained maintenance attempt".into();
    };
    let phase = match status.phase {
        Phase::Reserved => "reserved; closure pending",
        Phase::Closing => "closure issued; outcome pending",
        Phase::Closed => "closed; capturing catalog",
        Phase::StopIssued => "stop issued; outcome pending",
        Phase::Stopped => "stopped; replacement not installed",
        Phase::UploadIssued => "upload issued; outcome pending",
        Phase::Uploaded => "replacement uploaded; installation pending",
        Phase::InstallIssued => "installation issued; outcome pending",
        Phase::Installed => "replacement installed; start pending",
        Phase::StartIssued => "start issued; outcome pending",
        Phase::Verifying => "verifying preserved catalog",
        Phase::Verified => "verified; awaiting explicit activation",
        Phase::ActivateIssued => "activation issued; outcome pending",
        Phase::Ready => "maintenance completed (historical)",
    };
    format!("attempt {} — {phase}", status.approval.attempt.sequence)
}

pub(super) fn details(report: &FleetStoreMaintenance) -> Vec<(&'static str, String)> {
    let mut rows = vec![("Root", report.root.clone())];
    if let Some(status) = &report.reported {
        rows.push((
            "Qualified history limit",
            format!("{MAX_UPGRADES} retained attempts"),
        ));
        rows.push((
            "Dispatch deadline (Unix seconds)",
            (status.approval.expires_at_ns / 1_000_000_000).to_string(),
        ));
        for (label, scan) in [
            ("Before replacement", status.preservation.as_ref()),
            (
                "After replacement",
                status
                    .installation
                    .as_ref()
                    .and_then(|i| i.verification.as_ref()),
            ),
        ] {
            rows.push((
                label,
                scan.map_or_else(
                    || "not retained".into(),
                    |scan| {
                        format!(
                            "{}/{} releases; {}",
                            scan.observed,
                            scan.catalog.releases,
                            if scan.complete {
                                "complete"
                            } else {
                                "in progress"
                            }
                        )
                    },
                ),
            ));
        }
    }
    rows
}
