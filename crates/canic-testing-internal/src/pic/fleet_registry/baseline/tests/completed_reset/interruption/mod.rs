//! End a real adapter process at a durable Applied boundary, leaving partial infrastructure or activation.

use super::*;
use canic_host::fleet_ensure::{
    dto::{FleetEnsureActionKind, FleetEnsureProgressState},
    model::{
        CurrentFleetProtocolAction, EffectState, EnsureAction, FleetEnsureJournalRecord,
        clean_reinstall::CleanReinstallRecord,
    },
    ops::action_sha256,
    workflow::clean_reinstall,
};
use serde::{Deserialize, Serialize};

const PAUSE_ARGUMENTS: &str = "CANIC_RESET_PAUSE_ARGUMENTS";
const PAUSED: i32 = 73;

#[derive(Deserialize, Serialize)]
struct PausedApply {
    workspace: PathBuf,
    executable: PathBuf,
    fleet: String,
    plan_sha256: String,
    boundary: Boundary,
}

#[derive(Clone, Copy, Deserialize, Serialize)]
enum Boundary {
    AppliedEffects(usize),
    BeforeRegistryMirror,
}

pub(super) fn run_child() -> bool {
    let Ok(arguments) = std::env::var(PAUSE_ARGUMENTS) else {
        return false;
    };
    let request: PausedApply = serde_json::from_str(&arguments).unwrap();
    let paths = EnsurePaths::under(&request.workspace, "local", &request.fleet);
    let selection: CleanReinstallRecord = serde_json::from_slice(
        &std::fs::read(paths.plan.with_file_name("clean-reinstall.json")).unwrap(),
    )
    .unwrap();
    let executable = request.executable.to_str().unwrap();
    let mut platform = IcpEnsurePlatform::new(
        selection.desired.desired().clone(),
        executable,
        &request.workspace,
    )
    .with_progress_handler(move |progress| {
        let reached = match request.boundary {
            Boundary::AppliedEffects(count) => {
                usize::try_from(progress.applied_effects).unwrap() >= count
            }
            Boundary::BeforeRegistryMirror => progress
                .next_action
                .is_some_and(|next| next.kind == FleetEnsureActionKind::ActivateRegistryMirror),
        };
        if reached && matches!(progress.state, FleetEnsureProgressState::Advancing) {
            // Kernel lock release models actual process loss; production has already fsynced Applied.
            std::process::exit(PAUSED);
        }
    });
    let icp =
        canic_host::icp::IcpCli::new(executable, Some("local".into())).with_cwd(&request.workspace);
    clean_reinstall::apply(
        &request.workspace,
        "local",
        &request.fleet,
        &request.plan_sha256,
        &mut platform,
        &icp,
    )
    .unwrap();
    panic!("selected interruption boundary was not reached");
}

pub(super) fn pause_after(
    root: &Path,
    executable: &Path,
    fleet: &str,
    digest: &str,
    applied_effects: usize,
) {
    let journal = pause(
        root,
        executable,
        fleet,
        digest,
        Boundary::AppliedEffects(applied_effects),
    );
    assert_eq!(journal.effects.len(), applied_effects);
}

fn pause(
    root: &Path,
    executable: &Path,
    fleet: &str,
    digest: &str,
    boundary: Boundary,
) -> FleetEnsureJournalRecord {
    let request = PausedApply {
        workspace: root.into(),
        executable: executable.into(),
        fleet: fleet.into(),
        plan_sha256: digest.into(),
        boundary,
    };
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            CLI_TEST,
            "--exact",
            "--include-ignored",
            "--nocapture",
            "--test-threads=1",
        ])
        .current_dir(root)
        .env(PAUSE_ARGUMENTS, serde_json::to_string(&request).unwrap())
        .output()
        .unwrap();
    assert_eq!(
        output.status.code(),
        Some(PAUSED),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let paths = EnsurePaths::under(root, "local", fleet);
    let journal = canic_host::fleet_ensure::ops::read_journal(&paths)
        .unwrap()
        .unwrap();
    assert_eq!(
        journal.completion,
        canic_host::fleet_ensure::model::FleetEnsureCompletion::InProgress
    );
    assert!(
        journal
            .effects
            .iter()
            .all(|effect| effect.state == canic_host::fleet_ensure::model::EffectState::Applied)
    );
    journal
}

/// Stop after Coordinator activation but before Root mirror activation and workload provisioning.
pub(super) fn pause_activation(
    root: &Path,
    executable: &Path,
    fleet: &str,
    plan: &FleetEnsurePlan,
) {
    let journal = pause(
        root,
        executable,
        fleet,
        &plan.plan_sha256,
        Boundary::BeforeRegistryMirror,
    );
    let actions = plan.protocol_actions.iter().chain(
        journal
            .successor_phases
            .iter()
            .flat_map(|phase| phase.plan.as_ref().unwrap().protocol_actions.iter()),
    );
    let mut activation_applied = false;
    let mut mirror_unissued = false;
    for action in actions {
        let EnsureAction::FleetProtocol {
            action: protocol, ..
        } = action
        else {
            continue;
        };
        let effect = journal
            .effects
            .iter()
            .find(|effect| effect.action_sha256 == action_sha256(action));
        match protocol.as_ref() {
            CurrentFleetProtocolAction::ActivateRegistry { .. } => {
                activation_applied |=
                    effect.is_some_and(|effect| effect.state == EffectState::Applied);
            }
            CurrentFleetProtocolAction::ActivateRegistryMirror { .. } => {
                mirror_unissued |= effect.is_none();
            }
            _ => {}
        }
    }
    assert!(
        activation_applied && mirror_unissued,
        "Coordinator activation is durable while Root mirror activation remains unissued"
    );
}
