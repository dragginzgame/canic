//! Explain retained Root evidence without granting recovery or replay authority.
use canic_host::{Error, Result};
use canic_management_protocol::{Failure, Phase, Status, upgrade};
use ic_host_text::{
    escape::terminal_text,
    table::{self, Alignment, Options},
};
use std::fmt::Write as _;

pub(super) fn render(value: &serde_json::Value, is_upgrade: bool, receipt: bool) -> Result<String> {
    if value.is_null() {
        return Ok("Root evidence: none retained; admission and progress are unknown.\nNext: inspect the host attempt below. Keep its original operation and signed attempt; absence of Root evidence does not prove no effect.\n".into());
    }
    let mut output = String::new();
    if is_upgrade {
        let status: upgrade::Status = serde_json::from_value(value.clone())
            .map_err(|error| Error::Supplier(Box::new(error)))?;
        writeln!(output, "Retained Root phase: {:?}", status.phase).expect("String write");
        output.push_str(&dispatches(
            &["upload", "upgrade"],
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
        let guidance = match status.phase {
            upgrade::Phase::Expired => EXPIRED,
            upgrade::Phase::ReserveBlocked => RESERVE,
            upgrade::Phase::UploadIssued | upgrade::Phase::InstallIssued => UNCERTAIN,
            _ if status.failure.is_some() => FAILED,
            upgrade::Phase::Ready if receipt => COMPLETE,
            upgrade::Phase::Ready => READY,
            upgrade::Phase::Empty => UNKNOWN,
            _ => PENDING,
        };
        writeln!(output, "Next: {guidance}").expect("String write");
    } else {
        let status: Status = serde_json::from_value(value.clone())
            .map_err(|error| Error::Supplier(Box::new(error)))?;
        writeln!(output, "Retained Root phase: {:?}", status.phase).expect("String write");
        output.push_str(&dispatches(
            &["create", "upload", "install"],
            &status.attempts,
            &status.requests,
        ));
        writeln!(
            output,
            "Retained child: {}\nCreation refund: {}",
            terminal_text(status.worker.as_deref().unwrap_or("unknown")),
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
    Ok(output)
}

const PENDING: &str = "observe Root with fleet show for current progress. Apply this original operation when Root reports Ready to retain completion.";
const READY: &str = "apply this original operation with its operator identity to verify current code/controllers and retain completion. Root readiness alone is not a receipt.";
const COMPLETE: &str =
    "use fleet show for current observations; the retained receipt is historical evidence.";
const UNKNOWN: &str = "review the original host attempt; missing admission evidence does not authorize a replacement operation.";
const UNCERTAIN: &str = "observe Root with fleet show, then use the original apply to refresh retained evidence. Apply does not reconcile Root or reissue its effects; persistent uncertainty requires owner recovery review.";
const FAILED: &str = "review the retained failure and observe Root with fleet show. Owner maintenance/reconciliation may be required; no CLI recovery command is qualified yet. Preserve this operation.";
const EXPIRED: &str = "retain this operation for owner recovery review. Root stopped before another effect; expiry does not undo issued effects or renew its consumed slot.";
const RESERVE: &str = "retain this operation for owner recovery review. Root stopped at its cycle reserve; adding cycles does not reset this consumed slot.";

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
        table::render(
            &["ROOT EFFECT", "DISPATCHES", "REQUEST HASH"],
            &rows,
            &[Alignment::Left, Alignment::Right, Alignment::Left],
            Options {
                column_gap: 2,
                header_separator: true
            }
        )
    )
}
