//! Explain retained Root evidence without granting recovery or replay authority.
use canic_host::{ManagementStatus, StatusDetail};
use canic_management_protocol::{Failure, Phase, stored::PinState, upgrade};
use ic_host_text::{Alignment, escape, plain_table};
use std::fmt::Write as _;

pub(super) fn render(
    value: Option<&ManagementStatus>,
    detail: &StatusDetail,
    receipt: bool,
) -> String {
    let mut output = String::new();
    let upload = if let StatusDetail::StoredChild { delivery, .. } = detail {
        match delivery {
            None => output.push_str(
                "Store delivery: no retained observation; pin and cache preparation are unknown.\n",
            ),
            Some(delivery) => {
                writeln!(
                    output,
                    "Store pin: {}",
                    match delivery.state {
                        PinState::Pending => "not dispatched in this observation",
                        PinState::Issued => "issued; retention outcome unknown",
                        PinState::Retained => "retained for this exact operation and release",
                    }
                )
                .expect("String write");
                match delivery.cache_deadline {
                    Some(deadline) => writeln!(
                        output,
                        "Cache preparation deadline: {} (original)",
                        super::utc(deadline)
                    )
                    .expect("String write"),
                    None => output
                        .push_str("Cache preparation: no dispatch retained in this observation.\n"),
                }
            }
        }
        output.push_str("Cache contents are not observed here. Missing chunks do not authorize re-preparation or installation replay.\n");
        "prepare cache"
    } else {
        "upload"
    };
    let Some(value) = value else {
        output.push_str("Root evidence: none retained; admission and progress are unknown.\nNext: inspect the host attempt below. Keep its original operation and signed attempt; absence of Root evidence does not prove no effect.\n");
        return output;
    };
    if let ManagementStatus::Upgrade(status) = value {
        writeln!(output, "Retained Root phase: {:?}", status.phase).expect("String write");
        output.push_str(&dispatches(
            &[upload, "upgrade"],
            &status.attempts,
            &status.requests,
        ));
        writeln!(
            output,
            "Child version: {} → {}",
            version(status.previous_version),
            version(status.observed_version)
        )
        .expect("String write");
        failure(&mut output, status.failure.as_ref());
        let guidance = if let StatusDetail::StoredChild {
            maintenance: Some(maintenance),
            ..
        } = detail
        {
            writeln!(
                output,
                "Blob attempt {}: {:?}",
                maintenance.attempt, maintenance.phase
            )
            .expect("String write");
            output.push_str(&dispatches(
                &["drain", "stop", "start", "activate"],
                &maintenance.requests.map(|r| u8::from(r.is_some())),
                &maintenance.requests,
            ));
            if maintenance.phase
                == canic_management_protocol::application::MaintenancePhase::Inspected
                && !receipt
            {
                "review the reopened state, then explicitly authorize fleet activate for this original operation. Admission remains closed."
            } else {
                upgrade_continuation(status, receipt).1
            }
        } else {
            upgrade_continuation(status, receipt).1
        };
        writeln!(output, "Next: {guidance}").expect("String write");
    } else if let ManagementStatus::Creation(status) = value {
        writeln!(output, "Retained Root phase: {:?}", status.phase).expect("String write");
        output.push_str(&dispatches(
            &["create", upload, "install"],
            &status.attempts,
            &status.requests,
        ));
        writeln!(
            output,
            "Retained child: {}\nCreation refund: {}",
            escape(status.worker.as_deref().unwrap_or("unknown")),
            status
                .creation_refund
                .map_or_else(|| "unknown".into(), |cycles| format!("{cycles} cycles"))
        )
        .expect("String write");
        failure(&mut output, status.failure.as_ref());
        let guidance = match status.phase {
            Phase::Expired => EXPIRED,
            Phase::ReserveBlocked => RESERVE,
            Phase::CreateIssued if status.worker.is_none() => {
                "observe Root with fleet show. The creation outcome and child principal remain unresolved; no second creation is authorized. If still unresolved, retain this journal for owner recovery review."
            }
            Phase::CreateIssued | Phase::UploadIssued | Phase::InstallIssued => UNCERTAIN,
            _ if status.failure.is_some() => FAILED,
            Phase::Ready if receipt => COMPLETE,
            Phase::Ready => READY,
            Phase::Empty => UNKNOWN,
            _ => PENDING,
        };
        writeln!(output, "Next: {guidance}").expect("String write");
    }
    output
}

const PENDING: &str = "observe Root with fleet show for current progress. Apply this original operation when Root reports Ready to retain completion.";
const READY: &str = "apply this original operation with its operator identity to verify current code/controllers and retain completion. Root readiness alone is not a receipt.";
const COMPLETE: &str =
    "use fleet show for current observations; the retained receipt is historical evidence.";
const UNKNOWN: &str = "review the original host attempt; missing admission evidence does not authorize a replacement operation.";
const UNCERTAIN: &str = "apply this original operation to refresh retained evidence. Use fleet reconcile with this operation and its operator identity to request Root recovery; original effect limits still apply.";
const FAILED: &str = "review the retained failure. Use fleet reconcile with this operation and its operator identity for supported recovery. A trapped owner may still need maintenance; preserve this operation.";
const EXPIRED: &str = "retain this operation for owner recovery review. Root stopped before another effect; expiry does not undo issued effects or renew its consumed slot.";
const RESERVE: &str = "retain this operation for owner recovery review. Root stopped at its cycle reserve; adding cycles does not reset this consumed slot.";

/// Project a retained upgrade's next command without granting new effect authority.
pub(crate) fn upgrade_continuation(
    status: &upgrade::Status,
    receipt: bool,
) -> (&'static str, &'static str) {
    match status.phase {
        upgrade::Phase::Expired => ("status", EXPIRED),
        upgrade::Phase::ReserveBlocked => ("status", RESERVE),
        upgrade::Phase::UploadIssued
        | upgrade::Phase::InstallIssued
        | upgrade::Phase::StopIssued
        | upgrade::Phase::StartIssued => (
            "reconcile",
            "request Root recovery for this original operation; original effect limits still apply. Reconciliation does not authorize another installation.",
        ),
        _ if status.failure.is_some() => ("reconcile", FAILED),
        upgrade::Phase::Ready if receipt => ("status", COMPLETE),
        upgrade::Phase::Ready => ("apply", READY),
        upgrade::Phase::Empty => ("status", UNKNOWN),
        _ => ("apply", PENDING),
    }
}

fn version(value: Option<u64>) -> String {
    value.map_or_else(|| "unknown".into(), |value| value.to_string())
}

fn failure(output: &mut String, failure: Option<&Failure>) {
    let Some(failure) = failure else {
        return;
    };
    let explanation = match failure {
        Failure::Rejected(code) => format!("provider rejected the call (code {code})"),
        Failure::Uncertain => "call outcome is uncertain".into(),
        Failure::InvalidReply => "reply could not be decoded or admitted".into(),
        Failure::Mismatch => "observed code, controllers or readiness did not match".into(),
    };
    writeln!(
        output,
        "Retained failure: {explanation}; no retry authority granted."
    )
    .expect("String write");
}

fn dispatches<const N: usize>(
    labels: &[&str; N],
    attempts: &[u8; N],
    requests: &[Option<[u8; 32]>; N],
) -> String {
    let rows: Vec<_> = labels
        .iter()
        .zip(attempts)
        .zip(requests)
        .map(|((label, attempts), request)| {
            [
                (*label).to_owned(),
                attempts.to_string(),
                if request.is_some() {
                    "retained"
                } else {
                    "not retained"
                }
                .into(),
            ]
        })
        .collect();
    format!(
        "\n{}\n",
        plain_table(
            &["ROOT EFFECT", "DISPATCHES", "REQUEST HASH"],
            &rows,
            &[Alignment::Left, Alignment::Right, Alignment::Left]
        )
    )
}
