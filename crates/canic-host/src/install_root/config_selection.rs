use crate::release_set::{configured_fleet_name, configured_fleet_roles};
use crate::table::{ColumnAlign, render_table};
use crate::workspace_discovery::normalize_workspace_path;
use std::{
    collections::BTreeMap,
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
    reject_duplicate_fleet_names(&choices)?;
    Ok(choices)
}

fn reject_duplicate_fleet_names(choices: &[PathBuf]) -> Result<(), Box<dyn std::error::Error>> {
    let mut by_fleet = BTreeMap::<String, Vec<&PathBuf>>::new();
    for path in choices {
        if let Ok(fleet) = configured_fleet_name(path) {
            by_fleet.entry(fleet).or_default().push(path);
        }
    }

    for (fleet, paths) in by_fleet {
        if paths.len() > 1 {
            let configs = paths
                .into_iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>()
                .join(", ");
            return Err(format!("multiple configs declare fleet {fleet}: {configs}").into());
        }
    }

    Ok(())
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

// Treat checked-in fleet configs under the searched root as installable choices.
// The canister crates may live elsewhere in split-source downstream repos and
// are resolved separately.
const fn is_install_project_config(_path: &Path) -> bool {
    true
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
        lines.push("create fleets/<fleet>/canic.toml and run canic install <fleet>".to_string());
        return lines.join("\n");
    }

    if choices.len() == 1 {
        let fleet = fleet_name_from_config_path(&choices[0]).unwrap_or("<fleet>");
        lines.push(String::new());
        lines.extend(config_choice_table(workspace_root, choices));
        lines.push(String::new());
        lines.push(format!("run: canic install {fleet}"));
        return lines.join("\n");
    }

    lines.push("choose a fleet explicitly:".to_string());
    lines.push(String::new());
    lines.extend(config_choice_table(workspace_root, choices));
    lines.push(String::new());
    lines.push("run: canic install <fleet>".to_string());
    lines.join("\n")
}

fn fleet_name_from_config_path(path: &Path) -> Option<&str> {
    path.parent()?.file_name()?.to_str()
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
        .map(|row| [row.option, row.config, row.canisters])
        .collect::<Vec<_>>();
    render_table(
        &["#", "CONFIG", "CANISTERS"],
        &rows,
        &[ColumnAlign::Right, ColumnAlign::Left, ColumnAlign::Left],
    )
    .lines()
    .map(str::to_string)
    .collect()
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
