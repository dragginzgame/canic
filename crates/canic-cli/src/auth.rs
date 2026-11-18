//! Auth uses ordinary fleet selection, immutable plans and the shared effect commands.
use crate::{Output, args::FleetAuthCommand};

pub(crate) fn run(command: &FleetAuthCommand, json: bool) -> canic_host::Result<Output> {
    let (selection, state_dir) = match command {
        FleetAuthCommand::Plan {
            selection,
            state_dir,
            ..
        }
        | FleetAuthCommand::PlanRevocation {
            selection,
            state_dir,
            ..
        } => (selection, state_dir),
    };
    let path = selection.config.path(selection.app.as_deref())?;
    let selected = canic_host::load_config(&path)?
        .select(selection.app.as_deref(), selection.fleet.as_deref())?;
    let journal = state_dir
        .clone()
        .unwrap_or_else(|| selected.journal_path(&path));
    let (operation, plan) = match command {
        FleetAuthCommand::Plan {
            controller,
            receiver,
            ..
        } => canic_host::create_receiver_auth_plan(
            &journal,
            &path,
            &selected,
            controller,
            receiver.as_deref(),
        )?,
        FleetAuthCommand::PlanRevocation { operation, .. } => {
            canic_host::create_auth_revocation(&journal, operation, &selected)?
        }
    };
    if json {
        return Ok(Output::Json(
            serde_json::json!({"operation": operation, "plan": plan}),
        ));
    }
    Ok(Output::Text(format!(
        "{}\nPlan retained; review before using fleet apply.\n",
        crate::status::render(&canic_host::status(&journal, &operation)?, &journal)?
    )))
}
