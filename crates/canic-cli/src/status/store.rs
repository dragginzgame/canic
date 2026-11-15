//! Presentation of retained Store evidence; recommendations grant no effect authority.
use canic_management_protocol::store_maintenance::{Phase, Preservation, Status};
use std::fmt::Write as _;

pub(crate) fn continuation(status: &Status, now: Option<u128>) -> (&'static str, &'static str) {
    let expired = now.is_some_and(|now| now >= u128::from(status.approval.expires_at_ns));
    let note = match status.phase {
        Phase::Closing
        | Phase::StopIssued
        | Phase::UploadIssued
        | Phase::InstallIssued
        | Phase::StartIssued
        | Phase::ActivateIssued => {
            return (
                "reconcile",
                if expired {
                    "Dispatch window expired; observe the issued effect only."
                } else {
                    "Observe the issued effect; do not issue a replacement."
                },
            );
        }
        Phase::Verifying => {
            return (
                "reconcile",
                if expired {
                    "Dispatch window expired; finish catalog verification with admission closed."
                } else {
                    "Finish catalog verification; admission stays closed."
                },
            );
        }
        Phase::Ready => {
            return (
                "reconcile",
                "Retain completion evidence for this original operation.",
            );
        }
        Phase::Reserved => "Continue the approved Store closure.",
        Phase::Closed => "Finish the catalog scan and stop Store under this approval.",
        Phase::Stopped => "Explicitly install the approved replacement.",
        Phase::Uploaded => "Continue the approved installation.",
        Phase::Installed => "Start the replacement with admission closed.",
        Phase::Verified => "Explicitly activate the verified replacement.",
    };
    if status.failure.is_some() {
        return (
            "status",
            "Review the retained failure; preserve this original operation.",
        );
    }
    match now {
        Some(now) if now < u128::from(status.approval.expires_at_ns) => ("apply", note),
        Some(_) => (
            "status",
            "Dispatch window expired; new maintenance effects remain blocked.",
        ),
        None => (
            "status",
            "Host clock unavailable; review the original dispatch deadline.",
        ),
    }
}

pub(crate) fn render(status: &Status, now: Option<u128>, receipt: bool) -> String {
    let mut output = format!("Store maintenance: {:?}\n", status.phase);
    if let Some(failure) = &status.failure {
        writeln!(output, "Retained failure: {failure:?}").expect("String write");
    }
    let before = status.preservation.as_ref();
    let after = status
        .installation
        .as_ref()
        .and_then(|i| i.verification.as_ref());
    if before.is_some() || after.is_some() {
        writeln!(
            output,
            "Catalog scan: before {}; after {}",
            scan(before),
            scan(after)
        )
        .expect("String write");
    }
    if !receipt {
        writeln!(output, "Continuation: {}", continuation(status, now).1).expect("String write");
    }
    output
}

fn scan(value: Option<&Preservation>) -> String {
    value.map_or_else(
        || "not retained".into(),
        |value| {
            format!(
                "{}/{} releases ({})",
                value.observed,
                value.catalog.releases,
                if value.complete {
                    "complete"
                } else {
                    "incomplete"
                },
            )
        },
    )
}
