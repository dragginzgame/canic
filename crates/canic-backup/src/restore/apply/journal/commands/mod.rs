use super::{RestoreApplyJournal, RestoreApplyJournalOperation, RestoreApplyOperationKind};
use crate::persistence::resolve_backup_artifact_path;
use serde::{Deserialize, Serialize};
use std::path::Path;

///
/// RestoreApplyCommandPreview
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "runner preview exposes machine-readable availability and safety flags"
)]
pub struct RestoreApplyCommandPreview {
    pub response_version: u16,
    pub backup_id: String,
    pub ready: bool,
    pub complete: bool,
    pub operation_available: bool,
    pub command_available: bool,
    pub blocked_reasons: Vec<String>,
    pub operation: Option<RestoreApplyJournalOperation>,
    pub command: Option<RestoreApplyRunnerCommand>,
}

impl RestoreApplyCommandPreview {
    /// Build a no-execute runner command preview from a restore apply journal.
    #[must_use]
    pub fn from_journal(journal: &RestoreApplyJournal) -> Self {
        Self::from_journal_with_config(journal, &RestoreApplyCommandConfig::default())
    }

    /// Build a configured no-execute runner command preview from a journal.
    #[must_use]
    pub fn from_journal_with_config(
        journal: &RestoreApplyJournal,
        config: &RestoreApplyCommandConfig,
    ) -> Self {
        let operation = journal.next_transition_operation().cloned();
        let command = operation.as_ref().and_then(|operation| {
            RestoreApplyRunnerCommand::from_operation(operation, journal, config)
        });

        Self {
            response_version: 1,
            backup_id: journal.backup_id.clone(),
            ready: journal.ready,
            complete: journal.is_complete(),
            operation_available: operation.is_some(),
            command_available: command.is_some(),
            blocked_reasons: journal.blocked_reasons.clone(),
            operation,
            command,
        }
    }
}

///
/// RestoreApplyCommandConfig
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RestoreApplyCommandConfig {
    pub program: String,
    pub network: Option<String>,
}

impl Default for RestoreApplyCommandConfig {
    /// Build the default restore apply command preview configuration.
    fn default() -> Self {
        Self {
            program: "icp".to_string(),
            network: None,
        }
    }
}

///
/// RestoreApplyRunnerCommand
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RestoreApplyRunnerCommand {
    pub program: String,
    pub args: Vec<String>,
    pub mutates: bool,
    pub requires_stopped_canister: bool,
    pub note: String,
}

impl RestoreApplyRunnerCommand {
    // Build a no-execute ICP CLI command preview for one ready operation.
    fn from_operation(
        operation: &RestoreApplyJournalOperation,
        journal: &RestoreApplyJournal,
        config: &RestoreApplyCommandConfig,
    ) -> Option<Self> {
        match operation.operation {
            RestoreApplyOperationKind::StopCanister => Some(Self {
                program: config.program.clone(),
                args: icp_canister_args(
                    config,
                    vec!["stop".to_string(), operation.target_canister.clone()],
                ),
                mutates: true,
                requires_stopped_canister: false,
                note: "stops the target canister before snapshot restore".to_string(),
            }),
            RestoreApplyOperationKind::StartCanister => Some(Self {
                program: config.program.clone(),
                args: icp_canister_args(
                    config,
                    vec!["start".to_string(), operation.target_canister.clone()],
                ),
                mutates: true,
                requires_stopped_canister: false,
                note: "starts the target canister after snapshot restore".to_string(),
            }),
            RestoreApplyOperationKind::UploadSnapshot => {
                let artifact_path = upload_artifact_command_path(operation, journal)?;
                Some(Self {
                    program: config.program.clone(),
                    args: icp_canister_args(
                        config,
                        vec![
                            "snapshot".to_string(),
                            "upload".to_string(),
                            operation.target_canister.clone(),
                            "--input".to_string(),
                            artifact_path,
                            "--json".to_string(),
                        ],
                    ),
                    mutates: true,
                    requires_stopped_canister: false,
                    note: "uploads the downloaded snapshot artifact to the target canister"
                        .to_string(),
                })
            }
            RestoreApplyOperationKind::LoadSnapshot => {
                let snapshot_id = journal.uploaded_snapshot_id_for_load(operation)?;
                Some(Self {
                    program: config.program.clone(),
                    args: icp_canister_args(
                        config,
                        vec![
                            "snapshot".to_string(),
                            "restore".to_string(),
                            operation.target_canister.clone(),
                            snapshot_id.to_string(),
                        ],
                    ),
                    mutates: true,
                    requires_stopped_canister: true,
                    note: "loads the uploaded snapshot into the target canister".to_string(),
                })
            }
            RestoreApplyOperationKind::VerifyMember | RestoreApplyOperationKind::VerifyFleet => {
                match operation.verification_kind.as_deref() {
                    Some("status") => Some(Self {
                        program: config.program.clone(),
                        args: icp_canister_args(
                            config,
                            vec![
                                "status".to_string(),
                                operation.target_canister.clone(),
                                "--json".to_string(),
                            ],
                        ),
                        mutates: false,
                        requires_stopped_canister: false,
                        note: verification_command_note(
                            &operation.operation,
                            "checks target canister status",
                            "checks target fleet root canister status",
                        )
                        .to_string(),
                    }),
                    Some(_) | None => None,
                }
            }
        }
    }
}

// Return an operator note for member-level or fleet-level verification commands.
const fn verification_command_note(
    operation: &RestoreApplyOperationKind,
    member_note: &'static str,
    fleet_note: &'static str,
) -> &'static str {
    match operation {
        RestoreApplyOperationKind::VerifyFleet => fleet_note,
        RestoreApplyOperationKind::StopCanister
        | RestoreApplyOperationKind::StartCanister
        | RestoreApplyOperationKind::UploadSnapshot
        | RestoreApplyOperationKind::LoadSnapshot
        | RestoreApplyOperationKind::VerifyMember => member_note,
    }
}

// Build `icp canister` arguments with the optional network selector.
fn icp_canister_args(config: &RestoreApplyCommandConfig, mut tail: Vec<String>) -> Vec<String> {
    let mut args = vec!["canister".to_string()];
    args.append(&mut tail);
    if let Some(network) = &config.network {
        args.push("-n".to_string());
        args.push(network.clone());
    }
    args
}

// Resolve upload artifact paths the same way validation resolved them.
fn upload_artifact_command_path(
    operation: &RestoreApplyJournalOperation,
    journal: &RestoreApplyJournal,
) -> Option<String> {
    let artifact_path = operation.artifact_path.as_ref()?;
    let backup_root = journal.backup_root.as_ref()?;
    resolve_backup_artifact_path(Path::new(backup_root), artifact_path)
        .map(|path| path.to_string_lossy().to_string())
}
