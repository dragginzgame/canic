use super::{RestoreApplyJournal, RestoreApplyJournalOperation, RestoreApplyOperationKind};
use serde::{Deserialize, Serialize};
use std::path::{Component, Path};

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
            program: "dfx".to_string(),
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
    // Build a no-execute dfx command preview for one ready operation.
    fn from_operation(
        operation: &RestoreApplyJournalOperation,
        journal: &RestoreApplyJournal,
        config: &RestoreApplyCommandConfig,
    ) -> Option<Self> {
        match operation.operation {
            RestoreApplyOperationKind::UploadSnapshot => {
                let artifact_path = upload_artifact_command_path(operation, journal)?;
                Some(Self {
                    program: config.program.clone(),
                    args: dfx_canister_args(
                        config,
                        vec![
                            "snapshot".to_string(),
                            "upload".to_string(),
                            "--dir".to_string(),
                            artifact_path,
                            operation.target_canister.clone(),
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
                    args: dfx_canister_args(
                        config,
                        vec![
                            "snapshot".to_string(),
                            "load".to_string(),
                            operation.target_canister.clone(),
                            snapshot_id.to_string(),
                        ],
                    ),
                    mutates: true,
                    requires_stopped_canister: true,
                    note: "loads the uploaded snapshot into the target canister".to_string(),
                })
            }
            RestoreApplyOperationKind::ReinstallCode => Some(Self {
                program: config.program.clone(),
                args: dfx_canister_args(
                    config,
                    vec![
                        "install".to_string(),
                        "--mode".to_string(),
                        "reinstall".to_string(),
                        "--yes".to_string(),
                        operation.target_canister.clone(),
                    ],
                ),
                mutates: true,
                requires_stopped_canister: false,
                note: "reinstalls target canister code using the local dfx project configuration"
                    .to_string(),
            }),
            RestoreApplyOperationKind::VerifyMember | RestoreApplyOperationKind::VerifyFleet => {
                match operation.verification_kind.as_deref() {
                    Some("status") => Some(Self {
                        program: config.program.clone(),
                        args: dfx_canister_args(
                            config,
                            vec!["status".to_string(), operation.target_canister.clone()],
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
                    Some(_) => {
                        let method = operation.verification_method.as_ref()?;
                        Some(Self {
                            program: config.program.clone(),
                            args: dfx_canister_args(
                                config,
                                vec![
                                    "call".to_string(),
                                    "--query".to_string(),
                                    operation.target_canister.clone(),
                                    method.clone(),
                                ],
                            ),
                            mutates: false,
                            requires_stopped_canister: false,
                            note: verification_command_note(
                                &operation.operation,
                                "runs the declared verification method as a query call",
                                "runs the declared fleet verification method as a query call",
                            )
                            .to_string(),
                        })
                    }
                    None => None,
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
        RestoreApplyOperationKind::UploadSnapshot
        | RestoreApplyOperationKind::LoadSnapshot
        | RestoreApplyOperationKind::ReinstallCode
        | RestoreApplyOperationKind::VerifyMember => member_note,
    }
}

// Build `dfx canister` arguments with the optional network selector.
fn dfx_canister_args(config: &RestoreApplyCommandConfig, mut tail: Vec<String>) -> Vec<String> {
    let mut args = vec!["canister".to_string()];
    if let Some(network) = &config.network {
        args.push("--network".to_string());
        args.push(network.clone());
    }
    args.append(&mut tail);
    args
}

// Resolve upload artifact paths the same way validation resolved them.
fn upload_artifact_command_path(
    operation: &RestoreApplyJournalOperation,
    journal: &RestoreApplyJournal,
) -> Option<String> {
    let artifact_path = operation.artifact_path.as_ref()?;
    let path = Path::new(artifact_path);
    if path.is_absolute() {
        return Some(artifact_path.clone());
    }

    let backup_root = journal.backup_root.as_ref()?;
    let is_safe = path
        .components()
        .all(|component| matches!(component, Component::Normal(_) | Component::CurDir));
    if !is_safe {
        return None;
    }

    Some(
        Path::new(backup_root)
            .join(path)
            .to_string_lossy()
            .to_string(),
    )
}
