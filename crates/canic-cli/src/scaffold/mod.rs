use crate::{
    cli::clap::{flag_arg, parse_matches, parse_subcommand, passthrough_subcommand, string_option},
    cli::help::print_help_or_version,
    version_text,
};
use canic_host::{
    install_root::{current_canic_project_root, discover_project_canic_config_choices},
    release_set::{
        configured_role_lifecycle, declare_fleet_role, display_workspace_path,
        matching_fleet_config_paths,
    },
};
use clap::{Arg, Command as ClapCommand};
use std::{
    ffi::OsString,
    fs,
    io::{self, BufRead, Write},
    path::{Path, PathBuf},
};
use thiserror::Error as ThisError;

const FLEET_CREATE_HELP_AFTER: &str = "\
Examples:
  canic fleet create demo
  canic fleet create demo --yes";
const SCAFFOLD_HELP_AFTER: &str = "\
Examples:
  canic scaffold canister demo store";
const SCAFFOLD_CANISTER_HELP_AFTER: &str = "\
Examples:
  canic scaffold canister demo store";

///
/// ScaffoldCommandError
///

#[derive(Debug, ThisError)]
pub enum ScaffoldCommandError {
    #[error("{0}")]
    Usage(String),

    #[error("scaffold target already exists: {0}")]
    TargetExists(String),

    #[error("fleet create cancelled")]
    Cancelled,

    #[error("no Canic fleet configs found under fleets; run canic fleet create <name>")]
    NoConfigChoices,

    #[error("unknown fleet {0}; run canic fleet list to inspect config-defined fleets")]
    UnknownFleet(String),

    #[error(
        "multiple configs declare fleet {0}; use distinct [fleet].name values before selecting it"
    )]
    DuplicateFleet(String),

    #[error("fleet {0} config does not have a parent directory")]
    MissingFleetDirectory(String),

    #[error(transparent)]
    Host(#[from] Box<dyn std::error::Error>),

    #[error(transparent)]
    Io(#[from] io::Error),
}

///
/// ScaffoldOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
struct ScaffoldOptions {
    name: String,
    #[cfg(test)]
    project_root: Option<PathBuf>,
    yes: bool,
}

///
/// CanisterScaffoldOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
struct CanisterScaffoldOptions {
    fleet: String,
    role: String,
    #[cfg(test)]
    project_root: Option<PathBuf>,
}

impl ScaffoldOptions {
    #[cfg(test)]
    fn parse<I>(args: I) -> Result<Self, ScaffoldCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        Self::parse_with(args, fleet_create_command(), fleet_create_usage)
    }

    fn parse_with<I>(
        args: I,
        command: ClapCommand,
        usage: fn() -> String,
    ) -> Result<Self, ScaffoldCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches =
            parse_matches(command, args).map_err(|_| ScaffoldCommandError::Usage(usage()))?;
        Ok(Self {
            name: matches
                .get_one::<String>("name")
                .expect("clap requires name")
                .clone(),
            #[cfg(test)]
            project_root: None,
            yes: matches.get_flag("yes"),
        })
    }
}

impl CanisterScaffoldOptions {
    #[cfg(test)]
    fn parse<I>(args: I) -> Result<Self, ScaffoldCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        Self::parse_with(args, scaffold_canister_command(), scaffold_canister_usage)
    }

    fn parse_with<I>(
        args: I,
        command: ClapCommand,
        usage: fn() -> String,
    ) -> Result<Self, ScaffoldCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches =
            parse_matches(command, args).map_err(|_| ScaffoldCommandError::Usage(usage()))?;
        Ok(Self {
            fleet: string_option(&matches, "fleet").expect("clap requires fleet"),
            role: string_option(&matches, "role").expect("clap requires role"),
            #[cfg(test)]
            project_root: None,
        })
    }
}

/// Run the top-level scaffold command.
pub fn run<I>(args: I) -> Result<(), ScaffoldCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, usage, version_text()) {
        return Ok(());
    }

    match parse_subcommand(scaffold_command(), args)
        .map_err(|_| ScaffoldCommandError::Usage(usage()))?
    {
        None => {
            println!("{}", usage());
            Ok(())
        }
        Some((command, args)) => match command.as_str() {
            "canister" => run_canister(args),
            _ => unreachable!("scaffold dispatch command only defines known commands"),
        },
    }
}

/// Run the fleet create command.
pub fn run_fleet_create<I>(args: I) -> Result<(), ScaffoldCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, fleet_create_usage, version_text()) {
        return Ok(());
    }

    let options = ScaffoldOptions::parse_with(args, fleet_create_command(), fleet_create_usage)?;
    run_scaffold(options)
}

fn run_canister<I>(args: I) -> Result<(), ScaffoldCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, scaffold_canister_usage, version_text()) {
        return Ok(());
    }

    let options = CanisterScaffoldOptions::parse_with(
        args,
        scaffold_canister_command(),
        scaffold_canister_usage,
    )?;
    let result = scaffold_canister(&options)?;
    println!("Created Canic canister role:");
    println!("  role: {}.{}", result.fleet, result.role);
    println!("  package: {}", result.package);
    println!("  crate: {}", result.canister_dir.display());
    println!("  config: {}", result.config_path.display());
    println!("  state: declared");
    println!();
    println!("Next:");
    println!("  cargo check -p {}", result.package_name);
    println!(
        "  canic fleet role attach {} {} --subnet <subnet>",
        result.fleet, result.role
    );
    Ok(())
}

fn run_scaffold(options: ScaffoldOptions) -> Result<(), ScaffoldCommandError> {
    if !options.yes {
        confirm_scaffold(&options, io::stdin().lock(), io::stdout())?;
    }

    let result = scaffold_project(&options)?;
    println!("Created Canic fleet:");
    println!("  {}", result.project_dir.display());
    println!("  {}", result.root_dir.display());
    println!("  {}", result.app_dir.display());
    println!("  {}", result.config_path.display());
    println!();
    println!("Next:");
    println!("  edit icp.yaml");
    println!("  canic status");
    println!("  canic install {}", options.name);
    Ok(())
}

///
/// CanisterScaffoldResult
///

#[derive(Clone, Debug, Eq, PartialEq)]
struct CanisterScaffoldResult {
    fleet: String,
    role: String,
    package: String,
    package_name: String,
    canister_dir: PathBuf,
    config_path: PathBuf,
}

///
/// ScaffoldResult
///

#[derive(Clone, Debug, Eq, PartialEq)]
struct ScaffoldResult {
    project_dir: PathBuf,
    root_dir: PathBuf,
    app_dir: PathBuf,
    config_path: PathBuf,
}

/// Create a minimal root plus app canister fleet scaffold.
fn scaffold_project(options: &ScaffoldOptions) -> Result<ScaffoldResult, ScaffoldCommandError> {
    let project_dir = scaffold_project_root(options)?
        .join("fleets")
        .join(&options.name);
    if project_dir.exists() {
        return Err(ScaffoldCommandError::TargetExists(
            project_dir.display().to_string(),
        ));
    }

    let root_dir = project_dir.join("root");
    let root_src_dir = root_dir.join("src");
    let app_dir = project_dir.join("app");
    let app_src_dir = app_dir.join("src");

    let config_path = project_dir.join("canic.toml");
    write_new_file(&config_path, &canic_toml(&options.name))?;
    write_new_file(
        &root_dir.join("Cargo.toml"),
        &root_cargo_toml(&options.name),
    )?;
    write_new_file(&root_dir.join("build.rs"), ROOT_BUILD_RS)?;
    write_new_file(&root_src_dir.join("lib.rs"), ROOT_LIB_RS)?;
    write_new_file(&app_dir.join("Cargo.toml"), &app_cargo_toml(&options.name))?;
    write_new_file(&app_dir.join("build.rs"), APP_BUILD_RS)?;
    write_new_file(&app_src_dir.join("lib.rs"), APP_LIB_RS)?;

    Ok(ScaffoldResult {
        project_dir,
        root_dir,
        app_dir,
        config_path,
    })
}

/// Create a declared-only canister crate under an existing fleet config.
fn scaffold_canister(
    options: &CanisterScaffoldOptions,
) -> Result<CanisterScaffoldResult, ScaffoldCommandError> {
    let project_root = scaffold_canister_project_root(options)?;
    let config_path = selected_fleet_config_path(&project_root, &options.fleet)?;
    let fleet_dir = config_path
        .parent()
        .ok_or_else(|| ScaffoldCommandError::MissingFleetDirectory(options.fleet.clone()))?;
    ensure_role_not_declared(&config_path, &options.fleet, &options.role)?;
    let canister_dir = fleet_dir.join(&options.role);
    if canister_dir.exists() {
        return Err(ScaffoldCommandError::TargetExists(
            canister_dir.display().to_string(),
        ));
    }

    let src_dir = canister_dir.join("src");

    let package = options.role.clone();
    let package_name = canister_package_name(&options.fleet, &options.role);
    let workspace_member = display_workspace_path(&project_root, &canister_dir);
    write_new_file(
        &canister_dir.join("Cargo.toml"),
        &canister_cargo_toml(&options.fleet, &options.role),
    )?;
    write_new_file(&canister_dir.join("build.rs"), CANISTER_BUILD_RS)?;
    write_new_file(&src_dir.join("lib.rs"), CANISTER_LIB_RS)?;
    append_workspace_member(&project_root, &workspace_member)?;
    declare_fleet_role(&config_path, &options.fleet, &options.role, &package)?;

    Ok(CanisterScaffoldResult {
        fleet: options.fleet.clone(),
        role: options.role.clone(),
        package,
        package_name,
        canister_dir: display_path(&project_root, &canister_dir),
        config_path: display_path(&project_root, &config_path),
    })
}

fn append_workspace_member(
    workspace_root: &Path,
    member: &str,
) -> Result<(), ScaffoldCommandError> {
    let manifest_path = workspace_root.join("Cargo.toml");
    if !manifest_path.is_file() {
        return Ok(());
    }

    let source = fs::read_to_string(&manifest_path)?;
    let updated = append_workspace_member_source(&source, member)?;
    if updated != source {
        fs::write(manifest_path, updated)?;
    }
    Ok(())
}

fn append_workspace_member_source(
    source: &str,
    member: &str,
) -> Result<String, ScaffoldCommandError> {
    let member_literal = toml_string_literal(member);
    if source.contains(&member_literal) {
        return Ok(source.to_string());
    }

    let workspace_line = source
        .lines()
        .position(|line| line.trim() == "[workspace]")
        .ok_or_else(|| {
            ScaffoldCommandError::Usage("Cargo.toml is missing [workspace]".to_string())
        })?;
    let workspace_start = line_start_offset(source, workspace_line + 1);
    let workspace_end = section_end_offset(source, workspace_start);
    let section = &source[workspace_start..workspace_end];

    if let Some(members_offset) = find_members_array_offset(section) {
        return insert_workspace_member(source, workspace_start + members_offset, &member_literal);
    }

    let mut updated = source.to_string();
    updated.insert_str(
        workspace_start,
        &format!("members = [\n    {member_literal},\n]\n"),
    );
    Ok(updated)
}

fn insert_workspace_member(
    source: &str,
    members_offset: usize,
    member_literal: &str,
) -> Result<String, ScaffoldCommandError> {
    let array_start = source[members_offset..]
        .find('[')
        .map(|offset| members_offset + offset)
        .ok_or_else(|| {
            ScaffoldCommandError::Usage("workspace members must be an array".to_string())
        })?;
    let array_end = source[array_start..]
        .find(']')
        .map(|offset| array_start + offset)
        .ok_or_else(|| {
            ScaffoldCommandError::Usage("workspace members array is not closed".to_string())
        })?;
    if !source[array_start + 1..array_end].contains('\n') {
        return Ok(rewrite_single_line_members_array(
            source,
            array_start,
            array_end,
            member_literal,
        ));
    }

    let insert_at = source[..array_end]
        .rfind('\n')
        .map_or(array_end, |offset| offset + 1);
    let mut updated = source.to_string();
    updated.insert_str(insert_at, &format!("    {member_literal},\n"));
    Ok(updated)
}

fn rewrite_single_line_members_array(
    source: &str,
    array_start: usize,
    array_end: usize,
    member_literal: &str,
) -> String {
    let existing = source[array_start + 1..array_end]
        .trim()
        .trim_end_matches(',');
    let replacement = if existing.is_empty() {
        format!("[\n    {member_literal},\n]")
    } else {
        format!("[\n    {existing},\n    {member_literal},\n]")
    };
    let mut updated = source.to_string();
    updated.replace_range(array_start..=array_end, &replacement);
    updated
}

fn line_start_offset(source: &str, line_index: usize) -> usize {
    source
        .match_indices('\n')
        .nth(line_index.saturating_sub(1))
        .map_or(0, |(offset, _)| offset + 1)
}

fn section_end_offset(source: &str, section_start: usize) -> usize {
    source[section_start..]
        .match_indices('\n')
        .find_map(|(offset, _)| {
            let line_start = section_start + offset + 1;
            let line = source[line_start..].lines().next().unwrap_or_default();
            line.trim_start().starts_with('[').then_some(line_start)
        })
        .unwrap_or(source.len())
}

fn find_members_array_offset(section: &str) -> Option<usize> {
    section.match_indices("members").find_map(|(offset, _)| {
        let rest = &section[offset + "members".len()..];
        rest.trim_start().starts_with('=').then_some(offset)
    })
}

fn ensure_role_not_declared(
    config_path: &Path,
    fleet: &str,
    role: &str,
) -> Result<(), ScaffoldCommandError> {
    let roles = configured_role_lifecycle(config_path)?;
    if roles.iter().any(|row| row.role == role) {
        return Err(ScaffoldCommandError::Usage(format!(
            "role {fleet}.{role} is already declared"
        )));
    }
    Ok(())
}

fn scaffold_command() -> ClapCommand {
    ClapCommand::new("scaffold")
        .bin_name("canic scaffold")
        .about("Scaffold Canic source files")
        .disable_help_flag(true)
        .subcommand(passthrough_subcommand(
            ClapCommand::new("canister")
                .about("Create a declared-only canister role")
                .disable_help_flag(true),
        ))
        .after_help(SCAFFOLD_HELP_AFTER)
}

fn fleet_create_command() -> ClapCommand {
    ClapCommand::new("create")
        .bin_name("canic fleet create")
        .about("Create a minimal Canic fleet")
        .disable_help_flag(true)
        .arg(
            Arg::new("name")
                .value_name("name")
                .required(true)
                .value_parser(clap::builder::ValueParser::new(parse_project_name))
                .help("Snake-case fleet name to create"),
        )
        .arg(
            flag_arg("yes")
                .long("yes")
                .short('y')
                .help("Create the fleet without prompting for confirmation"),
        )
        .after_help(FLEET_CREATE_HELP_AFTER)
}

fn scaffold_canister_command() -> ClapCommand {
    ClapCommand::new("canister")
        .bin_name("canic scaffold canister")
        .about("Create a declared-only canister role")
        .disable_help_flag(true)
        .arg(
            Arg::new("fleet")
                .value_name("fleet")
                .required(true)
                .value_parser(clap::builder::ValueParser::new(parse_project_name))
                .help("Config-defined fleet name"),
        )
        .arg(
            Arg::new("role")
                .value_name("role")
                .required(true)
                .value_parser(clap::builder::ValueParser::new(parse_project_name))
                .help("Snake-case role name to scaffold"),
        )
        .after_help(SCAFFOLD_CANISTER_HELP_AFTER)
}

pub fn usage() -> String {
    let mut command = scaffold_command();
    command.render_help().to_string()
}

pub fn fleet_create_usage() -> String {
    let mut command = fleet_create_command();
    command.render_help().to_string()
}

pub fn scaffold_canister_usage() -> String {
    let mut command = scaffold_canister_command();
    command.render_help().to_string()
}

fn confirm_scaffold<R, W>(
    options: &ScaffoldOptions,
    mut reader: R,
    mut writer: W,
) -> Result<(), ScaffoldCommandError>
where
    R: BufRead,
    W: Write,
{
    let project_dir = scaffold_project_root(options)?
        .join("fleets")
        .join(&options.name);
    if project_dir.exists() {
        return Err(ScaffoldCommandError::TargetExists(
            project_dir.display().to_string(),
        ));
    }

    writeln!(writer, "Create Canic fleet?")?;
    writeln!(writer, "  project: {}", options.name)?;
    writeln!(writer, "  target:  {}", project_dir.display())?;
    writeln!(writer, "  install: canic install {}", options.name)?;
    write!(writer, "Continue? [y/N] ")?;
    writer.flush()?;

    let mut answer = String::new();
    reader.read_line(&mut answer)?;
    if matches!(answer.trim().to_ascii_lowercase().as_str(), "y" | "yes") {
        return Ok(());
    }

    Err(ScaffoldCommandError::Cancelled)
}

fn scaffold_project_root(options: &ScaffoldOptions) -> Result<PathBuf, ScaffoldCommandError> {
    #[cfg(not(test))]
    let _ = options;

    #[cfg(test)]
    if let Some(root) = &options.project_root {
        return Ok(root.clone());
    }

    current_canic_project_root().map_err(|err| ScaffoldCommandError::Usage(err.to_string()))
}

fn scaffold_canister_project_root(
    options: &CanisterScaffoldOptions,
) -> Result<PathBuf, ScaffoldCommandError> {
    #[cfg(not(test))]
    let _ = options;

    #[cfg(test)]
    if let Some(root) = &options.project_root {
        return Ok(root.clone());
    }

    current_canic_project_root().map_err(|err| ScaffoldCommandError::Usage(err.to_string()))
}

fn selected_fleet_config_path(
    project_root: &Path,
    fleet: &str,
) -> Result<PathBuf, ScaffoldCommandError> {
    let choices = discover_project_canic_config_choices(project_root)?;
    if choices.is_empty() {
        return Err(ScaffoldCommandError::NoConfigChoices);
    }

    let matches = matching_fleet_config_paths(&choices, fleet);
    match matches.as_slice() {
        [path] => Ok(path.clone()),
        [] => Err(ScaffoldCommandError::UnknownFleet(fleet.to_string())),
        _ => Err(ScaffoldCommandError::DuplicateFleet(fleet.to_string())),
    }
}

fn parse_project_name(name: &str) -> Result<String, String> {
    let mut previous_underscore = false;
    for (index, ch) in name.chars().enumerate() {
        let valid = ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_';
        if !valid {
            return Err(format!("project name must be snake_case: {name}"));
        }
        if index == 0 && !ch.is_ascii_lowercase() {
            return Err(format!("project name must be snake_case: {name}"));
        }
        if ch == '_' && previous_underscore {
            return Err(format!("project name must be snake_case: {name}"));
        }
        previous_underscore = ch == '_';
    }

    if name.is_empty() || name.ends_with('_') {
        return Err(format!("project name must be snake_case: {name}"));
    }

    Ok(name.to_string())
}

fn canister_package_name(fleet: &str, role: &str) -> String {
    format!("canister_{fleet}_{role}").replace('-', "_")
}

fn display_path(workspace_root: &Path, path: &Path) -> PathBuf {
    PathBuf::from(display_workspace_path(workspace_root, path))
}

fn toml_string_literal(value: &str) -> String {
    let mut escaped = String::from("\"");
    for ch in value.chars() {
        match ch {
            '\\' => escaped.push_str("\\\\"),
            '"' => escaped.push_str("\\\""),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            ch => escaped.push(ch),
        }
    }
    escaped.push('"');
    escaped
}

fn write_new_file(path: &Path, contents: &str) -> Result<(), ScaffoldCommandError> {
    if path.exists() {
        return Err(ScaffoldCommandError::TargetExists(
            path.display().to_string(),
        ));
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, contents).map_err(ScaffoldCommandError::from)
}

fn canister_cargo_toml(fleet: &str, role: &str) -> String {
    let canic_version = env!("CARGO_PKG_VERSION");
    let package_name = canister_package_name(fleet, role);
    format!(
        r#"[package]
name = "{package_name}"
edition = "2024"
rust-version = "1.91.0"
version = "0.1.0"
publish = false

[package.metadata.canic]
fleet = "{fleet}"
role = "{role}"

[lib]
crate-type = ["cdylib"]

[dependencies]
candid = {{ version = "0.10", default-features = false }}
canic = "{canic_version}"
ic-cdk = "0.20"

[build-dependencies]
canic = "{canic_version}"
"#
    )
}

fn canic_toml(name: &str) -> String {
    format!(
        r#"# Minimal Canic fleet config.

controllers = []
app_index = []

[fleet]
name = "{name}"

[auth.delegated_tokens]
enabled = false

[roles.root]
kind = "root"
package = "root"

[roles.app]
kind = "canister"
package = "app"

[subnets.prime.canisters.root]
kind = "root"

[subnets.prime.canisters.app]
kind = "singleton"
"#
    )
}

fn root_cargo_toml(name: &str) -> String {
    let canic_version = env!("CARGO_PKG_VERSION");
    format!(
        r#"[package]
name = "canister_{name}_root"
edition = "2024"
rust-version = "1.91.0"
version = "0.1.0"
publish = false

[package.metadata.canic]
fleet = "{name}"
role = "root"

[lib]
crate-type = ["cdylib"]

[dependencies]
candid = {{ version = "0.10", default-features = false }}
canic = {{ version = "{canic_version}", features = ["control-plane"] }}
ic-cdk = "0.20"

[build-dependencies]
canic = "{canic_version}"
"#
    )
}

fn app_cargo_toml(name: &str) -> String {
    let canic_version = env!("CARGO_PKG_VERSION");
    format!(
        r#"[package]
name = "canister_{name}_app"
edition = "2024"
rust-version = "1.91.0"
version = "0.1.0"
publish = false

[package.metadata.canic]
fleet = "{name}"
role = "app"

[lib]
crate-type = ["cdylib"]

[dependencies]
candid = {{ version = "0.10", default-features = false }}
canic = "{canic_version}"
ic-cdk = "0.20"

[build-dependencies]
canic = "{canic_version}"
"#
    )
}

const ROOT_BUILD_RS: &str = r#"fn main() {
    canic::build!("../canic.toml");
}
"#;

const APP_BUILD_RS: &str = r#"fn main() {
    canic::build!("../canic.toml");
}
"#;

const CANISTER_BUILD_RS: &str = r#"fn main() {
    canic::build!("../canic.toml");
}
"#;

const ROOT_LIB_RS: &str = r"#![expect(clippy::unused_async)]

//
// CANIC
//

canic::start!();

/// Run no-op setup for this scaffolded root.
async fn canic_setup() {}

/// Run no-op install handling for this scaffolded root.
async fn canic_install() {}

/// Run no-op upgrade handling for this scaffolded root.
async fn canic_upgrade() {}

canic::finish!();
";

const APP_LIB_RS: &str = r"#![expect(clippy::unused_async)]

/// Run no-op setup for this scaffolded app.
async fn canic_setup() {}

/// Accept no install payload for this scaffolded app.
async fn canic_install(_: Option<Vec<u8>>) {}

/// Run no-op upgrade handling for this scaffolded app.
async fn canic_upgrade() {}

canic::start!();

canic::finish!();
";

const CANISTER_LIB_RS: &str = r"#![expect(clippy::unused_async)]

canic::start!();

pub async fn canic_setup() {}

pub async fn canic_install(_: Option<Vec<u8>>) {}

pub async fn canic_upgrade() {}

canic::finish!();
";

#[cfg(test)]
mod tests;
