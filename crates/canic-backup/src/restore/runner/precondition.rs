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
use crate::persistence::CommandLifetimeHandle;
use crate::timestamp::state_updated_at;

pub(super) fn enforce_stopped_canister_precondition(
    config: &RestoreRunnerConfig,
    executor: &mut impl RestoreRunnerCommandExecutor,
    operation: &RestoreApplyJournalOperation,
    attempt: usize,
    updated_at: Option<&String>,
    command_lifetime: Option<CommandLifetimeHandle>,
) -> Result<Option<RestoreStoppedPreconditionFailure>, RestoreRunnerError> {
    let observation = observe_canister_status(config, executor, operation, command_lifetime)?;
    if observation.success && observation.status == Some(RestoreObservedCanisterStatus::Stopped) {
        return Ok(None);
    }

    Ok(Some(RestoreStoppedPreconditionFailure {
        command: observation.command,
        status_label: observation.status_label,
        output: observation.output,
        failure_reason: format!(
            "{RESTORE_RUN_STOPPED_PRECONDITION_FAILED}-attempt-{attempt}-{}",
            state_updated_at(updated_at)
        ),
    }))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum RestoreObservedCanisterStatus {
    Running,
    Stopped,
    Stopping,
}

pub(super) struct RestoreCanisterStatusEvidence {
    pub(super) status: RestoreObservedCanisterStatus,
    pub(super) module_hash: Option<String>,
}

#[derive(serde::Deserialize)]
struct RestoreCanisterStatusResponse {
    status: String,
    module_hash: Option<String>,
}

pub(super) struct RestoreCanisterStatusObservation {
    pub(super) command: RestoreApplyRunnerCommand,
    pub(super) success: bool,
    pub(super) status_label: String,
    pub(super) output: RestoreApplyCommandOutputPair,
    pub(super) status: Option<RestoreObservedCanisterStatus>,
}

pub(super) fn observe_canister_status(
    config: &RestoreRunnerConfig,
    executor: &mut impl RestoreRunnerCommandExecutor,
    operation: &RestoreApplyJournalOperation,
    command_lifetime: Option<CommandLifetimeHandle>,
) -> Result<RestoreCanisterStatusObservation, RestoreRunnerError> {
    let command = stopped_canister_status_command(config, operation);
    let raw = executor.execute(&command, command_lifetime)?;
    let output = RestoreApplyCommandOutputPair::from_bytes(
        &raw.stdout,
        &raw.stderr,
        RESTORE_RUN_OUTPUT_RECEIPT_LIMIT,
    );
    let status = status_output(&output);
    Ok(RestoreCanisterStatusObservation {
        command,
        success: raw.success,
        status_label: raw.status,
        output,
        status,
    })
}

fn stopped_canister_status_command(
    config: &RestoreRunnerConfig,
    operation: &RestoreApplyJournalOperation,
) -> RestoreApplyRunnerCommand {
    let mut args = vec!["canister".to_string()];
    args.push("status".to_string());
    args.push(operation.target_canister.clone());
    args.push("--json".to_string());
    if let Some(environment) = &config.command.environment {
        args.push("-e".to_string());
        args.push(environment.clone());
    }

    RestoreApplyRunnerCommand {
        program: config.command.program.clone(),
        args,
        mutates: false,
        requires_stopped_canister: false,
        note: "proves the target canister is stopped before snapshot load".to_string(),
    }
}

#[cfg(test)]
fn status_output_reports_stopped(output: &RestoreApplyCommandOutputPair) -> bool {
    status_output(output) == Some(RestoreObservedCanisterStatus::Stopped)
}

fn status_output(output: &RestoreApplyCommandOutputPair) -> Option<RestoreObservedCanisterStatus> {
    status_json(&output.stdout.text).or_else(|| status_json(&output.stderr.text))
}

fn status_json(output: &str) -> Option<RestoreObservedCanisterStatus> {
    parse_canister_status_evidence(output).map(|evidence| evidence.status)
}

pub(super) fn parse_canister_status_evidence(
    output: &str,
) -> Option<RestoreCanisterStatusEvidence> {
    let response = serde_json::from_str::<RestoreCanisterStatusResponse>(output).ok()?;
    let status = if response.status.eq_ignore_ascii_case("running") {
        RestoreObservedCanisterStatus::Running
    } else if response.status.eq_ignore_ascii_case("stopped") {
        RestoreObservedCanisterStatus::Stopped
    } else if response.status.eq_ignore_ascii_case("stopping") {
        RestoreObservedCanisterStatus::Stopping
    } else {
        return None;
    };
    Some(RestoreCanisterStatusEvidence {
        status,
        module_hash: response.module_hash,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_output_reports_stopped_status() {
        let output =
            RestoreApplyCommandOutputPair::from_bytes(br#"{"status":"Stopped"}"#, b"", 1024);

        assert!(status_output_reports_stopped(&output));

        let evidence =
            parse_canister_status_evidence(r#"{"status":"Running","module_hash":"0x0123"}"#)
                .expect("status evidence");
        assert_eq!(evidence.status, RestoreObservedCanisterStatus::Running);
        assert_eq!(evidence.module_hash.as_deref(), Some("0x0123"));
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
