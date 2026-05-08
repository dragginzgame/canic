use super::state::{read_install_state, read_selected_fleet_name};
use crate::release_set::{configured_fleet_name, configured_fleet_roles};
use crate::table::WhitespaceTable;
use crate::workspace_discovery::normalize_workspace_path;
use std::{
    env, fs,
    io::{self, IsTerminal, Write},
    path::{Path, PathBuf},
};

///
/// ConfigChoiceRow
///

struct ConfigChoiceRow {
    option: String,
    config: String,
    canisters: String,
}

const CONFIG_CHOICE_ROLE_PREVIEW_LIMIT: usize = 6;
const FLEETS_ROOT: &str = "fleets";
const ROOT_CONFIG_RELATIVE: &str = "canic.toml";

// Resolve install config selection without silently choosing among demo/test configs.
pub(super) fn resolve_install_config_path(
    workspace_root: &Path,
    dfx_root: &Path,
    network: &str,
    explicit_config_path: Option<&str>,
    interactive: bool,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    if let Some(path) = explicit_config_path {
        return Ok(normalize_workspace_path(
            workspace_root,
            PathBuf::from(path),
        ));
    }

    if let Some(path) = env::var_os("CANIC_CONFIG_PATH") {
        return Ok(normalize_workspace_path(
            workspace_root,
            PathBuf::from(path),
        ));
    }

    if let Some(path) = selected_install_config_path(workspace_root, dfx_root, network)? {
        return Ok(path);
    }

    let default = workspace_root.join(FLEETS_ROOT).join(ROOT_CONFIG_RELATIVE);
    if default.is_file() {
        return Ok(default);
    }

    let choices = discover_workspace_canic_config_choices(workspace_root)?;
    if interactive
        && let Some(path) = prompt_install_config_choice(workspace_root, &default, &choices)?
    {
        return Ok(path);
    }

    Err(config_selection_error(workspace_root, &default, &choices).into())
}

// Resolve the selected fleet's config path before falling back to project defaults.
fn selected_install_config_path(
    workspace_root: &Path,
    dfx_root: &Path,
    network: &str,
) -> Result<Option<PathBuf>, Box<dyn std::error::Error>> {
    if let Some(state) = read_install_state(dfx_root, network)? {
        let path = normalize_workspace_path(workspace_root, PathBuf::from(state.config_path));
        if path.is_file() {
            return Ok(Some(path));
        }
    }

    let Some(fleet) = read_selected_fleet_name(dfx_root, network)? else {
        return Ok(None);
    };
    let mut matches = Vec::new();
    for path in discover_workspace_canic_config_choices(workspace_root)? {
        if let Some(path) = selected_config_match(path, &fleet) {
            matches.push(path);
        }
    }

    match matches.as_slice() {
        [] => Err(format!(
            "selected fleet {fleet} is not declared by any install config under fleets; run canic fleet list or canic fleet use <name>"
        )
        .into()),
        [path] => Ok(Some(path.clone())),
        _ => Err(format!(
            "multiple install configs declare selected fleet {fleet}; run canic install --config <path>"
        )
        .into()),
    }
}

// Return one config path when its declared fleet identity matches the selection.
fn selected_config_match(path: PathBuf, fleet: &str) -> Option<PathBuf> {
    match configured_fleet_name(&path) {
        Ok(name) if name == fleet => Some(path),
        Ok(_) | Err(_) => None,
    }
}

// Discover installable Canic config choices from the fleet root.
pub(super) fn discover_workspace_canic_config_choices(
    workspace_root: &Path,
) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
    discover_canic_config_choices(&workspace_root.join(FLEETS_ROOT))
}

// Discover candidate `canic.toml` files under one fleet config root.
pub fn discover_canic_config_choices(
    root: &Path,
) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
    let mut choices = Vec::new();
    collect_canic_config_choices(root, &mut choices)?;
    choices.sort();
    Ok(choices)
}

// Recursively collect candidate config paths.
fn collect_canic_config_choices(
    root: &Path,
    choices: &mut Vec<PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    if !root.is_dir() {
        return Ok(());
    }

    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let path = entry.path();
        if file_type.is_symlink() {
            continue;
        }
        if file_type.is_dir() {
            collect_canic_config_choices(&path, choices)?;
        } else if file_type.is_file()
            && path.file_name().and_then(|name| name.to_str()) == Some("canic.toml")
            && is_install_project_config(&path)
        {
            choices.push(path);
        }
    }

    Ok(())
}

// Treat only configs next to a root canister directory as installable choices.
fn is_install_project_config(path: &Path) -> bool {
    path.parent()
        .is_some_and(|parent| parent.join("root/Cargo.toml").is_file())
}

// Format an actionable config-selection error with whitespace-aligned choices.
pub(super) fn config_selection_error(
    workspace_root: &Path,
    default: &Path,
    choices: &[PathBuf],
) -> String {
    let mut lines = vec![format!(
        "missing default Canic config at {}",
        display_workspace_path(workspace_root, default)
    )];

    if choices.is_empty() {
        lines.push("create fleets/canic.toml or run canic install --config <path>".to_string());
        return lines.join("\n");
    }

    if choices.len() == 1 {
        let choice = display_workspace_path(workspace_root, &choices[0]);
        lines.push(String::new());
        lines.extend(config_choice_table(workspace_root, choices));
        lines.push(String::new());
        lines.push(format!("run: canic install --config {choice}"));
        return lines.join("\n");
    }

    lines.push("choose a config path explicitly:".to_string());
    lines.push(String::new());
    lines.extend(config_choice_table(workspace_root, choices));
    lines.push(String::new());
    lines.push("run: canic install --config <path>".to_string());
    lines.join("\n")
}

// Prompt interactively for one discovered config when running in a terminal.
fn prompt_install_config_choice(
    workspace_root: &Path,
    default: &Path,
    choices: &[PathBuf],
) -> Result<Option<PathBuf>, Box<dyn std::error::Error>> {
    if choices.is_empty() || !io::stdin().is_terminal() {
        return Ok(None);
    }

    eprintln!(
        "missing default Canic config at {}",
        display_workspace_path(workspace_root, default)
    );
    eprintln!();
    for line in config_choice_table(workspace_root, choices) {
        eprintln!("{line}");
    }
    eprintln!();

    loop {
        eprint!("enter config number (ctrl-c to quit): ");
        io::stderr().flush()?;

        let mut answer = String::new();
        if io::stdin().read_line(&mut answer)? == 0 {
            return Ok(None);
        }

        let trimmed = answer.trim();
        let Ok(index) = trimmed.parse::<usize>() else {
            eprintln!("invalid selection: {trimmed}");
            continue;
        };
        let Some(path) = choices.get(index.saturating_sub(1)) else {
            eprintln!("selection out of range: {index}");
            continue;
        };

        return Ok(Some(path.clone()));
    }
}

// Render config choices with enough metadata to choose the intended topology.
fn config_choice_table(workspace_root: &Path, choices: &[PathBuf]) -> Vec<String> {
    let rows = choices
        .iter()
        .enumerate()
        .map(|(index, path)| config_choice_row(workspace_root, index + 1, path))
        .collect::<Vec<_>>();
    let mut table = WhitespaceTable::new(["#", "CONFIG", "CANISTERS"]);
    for row in rows {
        table.push_row([row.option, row.config, row.canisters]);
    }
    table.render().lines().map(str::to_string).collect()
}

// Summarize the root-subnet fleet roles for one install config choice.
fn config_choice_row(workspace_root: &Path, option: usize, path: &Path) -> ConfigChoiceRow {
    let config = display_workspace_path(workspace_root, path);
    match configured_fleet_roles(path) {
        Ok(roles) => ConfigChoiceRow {
            option: option.to_string(),
            config,
            canisters: format_canister_summary(&roles),
        },
        Err(_) => ConfigChoiceRow {
            option: option.to_string(),
            config,
            canisters: "invalid config".to_string(),
        },
    }
}

// Format the root-subnet canister count with a bounded role preview.
fn format_canister_summary(roles: &[String]) -> String {
    if roles.is_empty() {
        return "0".to_string();
    }

    let preview = roles
        .iter()
        .take(CONFIG_CHOICE_ROLE_PREVIEW_LIMIT)
        .map(String::as_str)
        .collect::<Vec<_>>()
        .join(", ");
    let suffix = if roles.len() > CONFIG_CHOICE_ROLE_PREVIEW_LIMIT {
        ", ..."
    } else {
        ""
    };

    format!("{} ({preview}{suffix})", roles.len())
}

// Render a workspace-relative path where possible for concise diagnostics.
fn display_workspace_path(workspace_root: &Path, path: &Path) -> String {
    path.strip_prefix(workspace_root)
        .unwrap_or(path)
        .display()
        .to_string()
}
