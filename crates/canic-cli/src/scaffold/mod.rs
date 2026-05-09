use crate::{
    args::{flag_arg, parse_matches, path_option, print_help_or_version},
    version_text,
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

///
/// ScaffoldCommandError
///

#[derive(Debug, ThisError)]
pub enum ScaffoldCommandError {
    #[error("{0}")]
    Usage(String),

    #[error("fleet target already exists: {0}")]
    TargetExists(String),

    #[error("fleet create cancelled")]
    Cancelled,

    #[error(transparent)]
    Io(#[from] io::Error),
}

///
/// ScaffoldOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScaffoldOptions {
    pub name: String,
    pub fleets_dir: PathBuf,
    pub yes: bool,
}

impl ScaffoldOptions {
    #[cfg(test)]
    pub fn parse<I>(args: I) -> Result<Self, ScaffoldCommandError>
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
            fleets_dir: path_option(&matches, "dir").unwrap_or_else(|| PathBuf::from("fleets")),
            yes: matches.get_flag("yes"),
        })
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

fn run_scaffold(options: ScaffoldOptions) -> Result<(), ScaffoldCommandError> {
    if !options.yes {
        confirm_scaffold(&options, io::stdin().lock(), io::stdout())?;
    }

    let result = scaffold_project(&options)?;
    println!("Created Canic fleet:");
    println!("  {}", result.project_dir.display());
    println!("  {}", result.root_dir.display());
    println!("  {}", result.app_dir.display());
    println!();
    println!("Next:");
    println!("  canic install {}", options.name);
    Ok(())
}

///
/// ScaffoldResult
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScaffoldResult {
    pub project_dir: PathBuf,
    pub root_dir: PathBuf,
    pub app_dir: PathBuf,
    pub config_path: PathBuf,
}

/// Create a minimal root plus app canister fleet scaffold.
pub fn scaffold_project(options: &ScaffoldOptions) -> Result<ScaffoldResult, ScaffoldCommandError> {
    let project_dir = options.fleets_dir.join(&options.name);
    if project_dir.exists() {
        return Err(ScaffoldCommandError::TargetExists(
            project_dir.display().to_string(),
        ));
    }

    let root_dir = project_dir.join("root");
    let root_src_dir = root_dir.join("src");
    let app_dir = project_dir.join("app");
    let app_src_dir = app_dir.join("src");
    fs::create_dir_all(&root_src_dir)?;
    fs::create_dir_all(&app_src_dir)?;

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
            Arg::new("dir")
                .long("dir")
                .value_name("dir")
                .num_args(1)
                .help("Fleets directory to create under; defaults to fleets"),
        )
        .arg(
            flag_arg("yes")
                .long("yes")
                .short('y')
                .help("Create the fleet without prompting for confirmation"),
        )
        .after_help(FLEET_CREATE_HELP_AFTER)
}

pub fn fleet_create_usage() -> String {
    let mut command = fleet_create_command();
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
    let project_dir = options.fleets_dir.join(&options.name);
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

fn write_new_file(path: &Path, contents: &str) -> Result<(), ScaffoldCommandError> {
    if path.exists() {
        return Err(ScaffoldCommandError::TargetExists(
            path.display().to_string(),
        ));
    }
    fs::write(path, contents).map_err(ScaffoldCommandError::from)
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

[subnets.prime]
auto_create = ["app"]
subnet_index = ["app"]

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

[lib]
crate-type = ["cdylib"]

[dependencies]
candid = {{ version = "0.10", default-features = false }}
canic = {{ version = "{canic_version}", features = ["control-plane"] }}
ic-cdk = "0.20"
serde = {{ version = "1", default-features = false }}

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
    canic::build_root!("../canic.toml");
}
"#;

const APP_BUILD_RS: &str = r#"fn main() {
    canic::build!("../canic.toml");
}
"#;

const ROOT_LIB_RS: &str = r"#![expect(clippy::unused_async)]

//
// CANIC
//

canic::start_root!();

/// Run no-op setup for this scaffolded root.
pub async fn canic_setup() {}

/// Run no-op install handling for this scaffolded root.
pub async fn canic_install() {}

/// Run no-op upgrade handling for this scaffolded root.
pub async fn canic_upgrade() {}

canic::cdk::export_candid_debug!();
";

const APP_LIB_RS: &str = r#"#![expect(clippy::unused_async)]

use canic::ids::CanisterRole;

const APP: CanisterRole = CanisterRole::new("app");

/// Run no-op setup for this scaffolded app.
pub async fn canic_setup() {}

/// Accept no install payload for this scaffolded app.
pub async fn canic_install(_: Option<Vec<u8>>) {}

/// Run no-op upgrade handling for this scaffolded app.
pub async fn canic_upgrade() {}

canic::start!(APP);

canic::cdk::export_candid_debug!();
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::temp_dir;

    // Ensure scaffold options parse the project name and optional root directory.
    #[test]
    fn parses_scaffold_options() {
        let options = ScaffoldOptions::parse([
            OsString::from("my_app"),
            OsString::from("--dir"),
            OsString::from("tmp_fleets"),
        ])
        .expect("parse scaffold options");

        assert_eq!(options.name, "my_app");
        assert_eq!(options.fleets_dir, PathBuf::from("tmp_fleets"));
        assert!(!options.yes);
    }

    // Ensure scaffold accepts explicit noninteractive confirmation.
    #[test]
    fn parses_scaffold_yes_option() {
        let options = ScaffoldOptions::parse([OsString::from("my_app"), OsString::from("--yes")])
            .expect("parse scaffold yes option");

        assert!(options.yes);
    }

    // Ensure confirmation accepts an explicit yes response.
    #[test]
    fn confirm_scaffold_accepts_yes() {
        let root = temp_dir("canic-cli-scaffold-confirm-yes");
        let options = ScaffoldOptions {
            name: "my_app".to_string(),
            fleets_dir: root.join("fleets"),
            yes: false,
        };
        let mut output = Vec::new();

        confirm_scaffold(&options, io::Cursor::new(b"y\n"), &mut output).expect("confirm scaffold");

        let output = String::from_utf8(output).expect("utf8 prompt");
        assert!(output.contains("target:"));
        assert!(output.contains("fleets/my_app"));
        assert!(output.contains("install: canic install my_app"));
    }

    // Ensure confirmation defaults to no on empty input.
    #[test]
    fn confirm_scaffold_rejects_empty_response() {
        let root = temp_dir("canic-cli-scaffold-confirm-no");
        let options = ScaffoldOptions {
            name: "my_app".to_string(),
            fleets_dir: root.join("fleets"),
            yes: false,
        };
        let mut output = Vec::new();

        let err = confirm_scaffold(&options, io::Cursor::new(b"\n"), &mut output)
            .expect_err("empty response should cancel");

        assert!(matches!(err, ScaffoldCommandError::Cancelled));
    }

    // Ensure invalid scaffold names are rejected before filesystem writes.
    #[test]
    fn rejects_invalid_project_names() {
        for name in ["MyApp", "my-app", "_app", "app_", "app__one", "1app"] {
            assert!(matches!(
                ScaffoldOptions::parse([OsString::from(name)]),
                Err(ScaffoldCommandError::Usage(_))
            ));
        }
    }

    // Ensure scaffold writes the expected minimal root and app files.
    #[test]
    fn scaffold_project_writes_root_and_app_files() {
        let root = temp_dir("canic-cli-scaffold");
        let options = ScaffoldOptions {
            name: "my_app".to_string(),
            fleets_dir: root.join("fleets"),
            yes: true,
        };

        let result = scaffold_project(&options).expect("scaffold project");
        let config = fs::read_to_string(&result.config_path).expect("read config");
        let root_lib =
            fs::read_to_string(result.root_dir.join("src/lib.rs")).expect("read root lib");
        let root_manifest =
            fs::read_to_string(result.root_dir.join("Cargo.toml")).expect("read root manifest");
        let app_lib = fs::read_to_string(result.app_dir.join("src/lib.rs")).expect("read app lib");
        let app_manifest =
            fs::read_to_string(result.app_dir.join("Cargo.toml")).expect("read app manifest");

        fs::remove_dir_all(root).expect("remove scaffold temp root");
        assert!(config.contains("name = \"my_app\""));
        assert!(config.contains("[auth.delegated_tokens]"));
        assert!(config.contains("enabled = false"));
        assert!(config.contains("auto_create = [\"app\"]"));
        assert!(config.contains("subnet_index = [\"app\"]"));
        assert!(config.contains("[subnets.prime.canisters.root]"));
        assert!(config.contains("[subnets.prime.canisters.app]"));
        assert!(root_manifest.contains("version = \"0.1.0\""));
        assert!(root_manifest.contains("canic = { version = \""));
        assert!(!root_manifest.contains("workspace = true"));
        assert!(root_lib.contains("canic::start_root!();"));
        assert!(app_manifest.contains("name = \"canister_my_app_app\""));
        assert!(app_manifest.contains("canic = \""));
        assert!(!app_manifest.contains("workspace = true"));
        assert!(app_lib.contains("CanisterRole::new(\"app\")"));
        assert!(app_lib.contains("canic::start!(APP);"));
    }

    // Ensure scaffold refuses to overwrite an existing project directory.
    #[test]
    fn scaffold_project_rejects_existing_target() {
        let root = temp_dir("canic-cli-scaffold-existing");
        let options = ScaffoldOptions {
            name: "my_app".to_string(),
            fleets_dir: root.join("fleets"),
            yes: true,
        };
        fs::create_dir_all(options.fleets_dir.join("my_app")).expect("create existing target");

        let err = scaffold_project(&options).expect_err("existing scaffold should fail");

        fs::remove_dir_all(root).expect("remove scaffold temp root");
        assert!(matches!(err, ScaffoldCommandError::TargetExists(_)));
    }
}
