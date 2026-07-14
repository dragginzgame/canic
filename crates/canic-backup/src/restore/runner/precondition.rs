//! Module: restore::runner::precondition
//!
//! Responsibility: enforce stopped-canister preconditions before snapshot load.
//! Does not own: apply journal transitions, command execution policy, or receipts.
//! Boundary: probes target status through the injected runner executor.

use super::{
    RestoreApplyCommandOutputPair, RestoreApplyJournalOperation, RestoreApplyRunnerCommand,
    constants::{RESTORE_RUN_OUTPUT_RECEIPT_LIMIT, RESTORE_RUN_STOPPED_PRECONDITION_FAILED},
    types::{
        RestoreRunnerCommandExecutor, RestoreRunnerConfig, RestoreRunnerError,
        RestoreStoppedPreconditionFailure,
    },
};
use crate::timestamp::state_updated_at;

pub(super) fn enforce_stopped_canister_precondition(
    config: &RestoreRunnerConfig,
    executor: &mut impl RestoreRunnerCommandExecutor,
    operation: &RestoreApplyJournalOperation,
    attempt: usize,
    updated_at: Option<&String>,
) -> Result<Option<RestoreStoppedPreconditionFailure>, RestoreRunnerError> {
    let command = stopped_canister_status_command(config, operation);
    let output = executor.execute(&command)?;
    let status_label = output.status;
    let output_pair = RestoreApplyCommandOutputPair::from_bytes(
        &output.stdout,
        &output.stderr,
        RESTORE_RUN_OUTPUT_RECEIPT_LIMIT,
    );
    if output.success && status_output_reports_stopped(&output_pair) {
        return Ok(None);
    }

    Ok(Some(RestoreStoppedPreconditionFailure {
        command,
        status_label,
        output: output_pair,
        failure_reason: format!(
            "{RESTORE_RUN_STOPPED_PRECONDITION_FAILED}-attempt-{attempt}-{}",
            state_updated_at(updated_at)
        ),
    }))
}

fn stopped_canister_status_command(
    config: &RestoreRunnerConfig,
    operation: &RestoreApplyJournalOperation,
) -> RestoreApplyRunnerCommand {
    let mut args = vec!["canister".to_string()];
    args.push("status".to_string());
    args.push(operation.target_canister.clone());
    args.push("--json".to_string());
    if let Some(network) = &config.command.network {
        args.push("-n".to_string());
        args.push(network.clone());
    }

    RestoreApplyRunnerCommand {
        program: config.command.program.clone(),
        args,
        mutates: false,
        requires_stopped_canister: false,
        note: "proves the target canister is stopped before snapshot load".to_string(),
    }
}

fn status_output_reports_stopped(output: &RestoreApplyCommandOutputPair) -> bool {
    status_json_reports_stopped(&output.stdout.text)
        || status_json_reports_stopped(&output.stderr.text)
}

fn status_json_reports_stopped(output: &str) -> bool {
    serde_json::from_str::<serde_json::Value>(output)
        .ok()
        .and_then(|value| {
            find_json_field(&value, "status")
                .and_then(serde_json::Value::as_str)
                .map(str::to_string)
        })
        .is_some_and(|status| status.eq_ignore_ascii_case("stopped"))
}

fn find_json_field<'a>(value: &'a serde_json::Value, field: &str) -> Option<&'a serde_json::Value> {
    match value {
        serde_json::Value::Object(map) => map
            .get(field)
            .or_else(|| map.values().find_map(|value| find_json_field(value, field))),
        serde_json::Value::Array(values) => values
            .iter()
            .find_map(|value| find_json_field(value, field)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_output_reports_stopped_status() {
        let output =
            RestoreApplyCommandOutputPair::from_bytes(br#"{"status":"Stopped"}"#, b"", 1024);

        assert!(status_output_reports_stopped(&output));

        let output = RestoreApplyCommandOutputPair::from_bytes(
            br#"{"canister":{"status":"Stopped"}}"#,
            b"",
            1024,
        );

        assert!(status_output_reports_stopped(&output));
    }

    #[test]
    fn status_output_rejects_running_status() {
        let output =
            RestoreApplyCommandOutputPair::from_bytes(br#"{"status":"Running"}"#, b"", 1024);

        assert!(!status_output_reports_stopped(&output));
    }

    #[test]
    fn status_output_rejects_non_json_status() {
        let output = RestoreApplyCommandOutputPair::from_bytes(b"canister is stopped\n", b"", 1024);

        assert!(!status_output_reports_stopped(&output));
    }
}
