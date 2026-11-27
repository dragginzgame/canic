//! Presentation of retained Store evidence; recommendations grant no effect authority.
use canic_management_protocol::store_maintenance::{Preservation, Status};
use std::fmt::Write as _;

pub(crate) fn continuation(status: &Status, now: Option<u128>) -> (&'static str, &'static str) {
    use canic_host::StoreContinuation as Next;
    match canic_host::store_continuation(status, now) {
        Next::RecoverIssued { expired } => (
            "reconcile",
            if expired {
                "Dispatch window expired; observe the issued effect only."
            } else {
                "Observe the issued effect; do not issue a replacement."
            },
        ),
        Next::VerifyCatalog { expired } => (
            "reconcile",
            if expired {
                "Dispatch window expired; finish catalog verification with admission closed."
            } else {
                "Finish catalog verification; admission stays closed."
            },
        ),
        Next::RetainCompletion => (
            "reconcile",
            "Retain completion evidence for this original operation.",
        ),
        Next::Close => ("apply", "Continue the approved Store closure."),
        Next::ScanAndStop => (
            "apply",
            "Finish the catalog scan and stop Store under this approval.",
        ),
        Next::Install => ("apply", "Explicitly install the approved replacement."),
        Next::ContinueInstallation => ("apply", "Continue the approved installation."),
        Next::Start => ("apply", "Start the replacement with admission closed."),
        Next::Activate => ("apply", "Explicitly activate the verified replacement."),
        Next::ReviewFailure => (
            "status",
            "Review the retained failure; preserve this original operation.",
        ),
        Next::Expired => (
            "status",
            "Dispatch window expired; new maintenance effects remain blocked.",
        ),
        Next::ClockUnavailable => (
            "status",
            "Host clock unavailable; review the original dispatch deadline.",
        ),
    }
}

pub(crate) fn render(status: &Status, now: Option<u128>, receipt: bool) -> String {
    let mut output = format!(
        "Store maintenance: {:?} (attempt {})\n",
        status.phase, status.approval.attempt.sequence
    );
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
