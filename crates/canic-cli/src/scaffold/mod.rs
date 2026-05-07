use crate::{
    args::{first_arg_is_help, first_arg_is_version, flag_arg, parse_matches, path_option},
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

///
/// ScaffoldCommandError
///

#[derive(Debug, ThisError)]
pub enum ScaffoldCommandError {
    #[error("{0}")]
    Usage(String),

    #[error("project name must be snake_case: {0}")]
    InvalidProjectName(String),

    #[error("scaffold target already exists: {0}")]
    TargetExists(String),

    #[error("scaffold cancelled")]
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
    pub canisters_dir: PathBuf,
    pub yes: bool,
}

impl ScaffoldOptions {
    /// Parse scaffold options from CLI arguments.
    pub fn parse<I>(args: I) -> Result<Self, ScaffoldCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_matches(scaffold_command(), args)
            .map_err(|_| ScaffoldCommandError::Usage(usage()))?;
        let name = matches
            .get_one::<String>("name")
            .expect("clap requires name")
            .clone();
        validate_project_name(&name)?;

        Ok(Self {
            name,
            canisters_dir: path_option(&matches, "dir")
                .unwrap_or_else(|| PathBuf::from("canisters")),
            yes: matches.get_flag("yes"),
        })
    }
}

/// Run the scaffold command.
pub fn run<I>(args: I) -> Result<(), ScaffoldCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if first_arg_is_help(&args) {
        println!("{}", usage());
        return Ok(());
    }
    if first_arg_is_version(&args) {
        println!("{}", version_text());
        return Ok(());
    }

    let options = ScaffoldOptions::parse(args)?;
    if !options.yes {
        confirm_scaffold(&options, io::stdin().lock(), io::stdout())?;
    }

    let result = scaffold_project(&options)?;
    println!("Created Canic scaffold:");
    println!("  {}", result.project_dir.display());
    println!("  {}", result.root_dir.display());
    println!();
    println!("Next:");
    println!("  canic install --config {}", result.config_path.display());
    Ok(())
}

///
/// ScaffoldResult
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScaffoldResult {
    pub project_dir: PathBuf,
    pub root_dir: PathBuf,
    pub config_path: PathBuf,
}

/// Create a minimal root-canister project scaffold.
pub fn scaffold_project(options: &ScaffoldOptions) -> Result<ScaffoldResult, ScaffoldCommandError> {
    let project_dir = options.canisters_dir.join(&options.name);
    if project_dir.exists() {
        return Err(ScaffoldCommandError::TargetExists(
            project_dir.display().to_string(),
        ));
    }

    let root_dir = project_dir.join("root");
    let src_dir = root_dir.join("src");
    fs::create_dir_all(&src_dir)?;

    let config_path = project_dir.join("canic.toml");
    write_new_file(&config_path, &canic_toml(&options.name))?;
    write_new_file(
        &root_dir.join("Cargo.toml"),
        &root_cargo_toml(&options.name),
    )?;
    write_new_file(&root_dir.join("build.rs"), ROOT_BUILD_RS)?;
    write_new_file(&src_dir.join("lib.rs"), ROOT_LIB_RS)?;

    Ok(ScaffoldResult {
        project_dir,
        root_dir,
        config_path,
    })
}

// Build the scaffold parser.
fn scaffold_command() -> ClapCommand {
    ClapCommand::new("scaffold")
        .bin_name("canic scaffold")
        .about("Create a minimal Canic fleet scaffold")
        .disable_help_flag(true)
        .arg(
            Arg::new("name")
                .value_name("name")
                .required(true)
                .help("Snake-case project and fleet name to create"),
        )
        .arg(
            Arg::new("dir")
                .long("dir")
                .value_name("dir")
                .num_args(1)
                .help("Canisters directory to create under; defaults to canisters"),
        )
        .arg(
            flag_arg("yes")
                .long("yes")
                .short('y')
                .help("Create the scaffold without prompting for confirmation"),
        )
}

// Return scaffold command usage text.
fn usage() -> String {
    let mut command = scaffold_command();
    command.render_help().to_string()
}

// Ask the operator to confirm the target before creating multiple files.
fn confirm_scaffold<R, W>(
    options: &ScaffoldOptions,
    mut reader: R,
    mut writer: W,
) -> Result<(), ScaffoldCommandError>
where
    R: BufRead,
    W: Write,
{
    let project_dir = options.canisters_dir.join(&options.name);
    if project_dir.exists() {
        return Err(ScaffoldCommandError::TargetExists(
            project_dir.display().to_string(),
        ));
    }

    writeln!(writer, "Create Canic scaffold?")?;
    writeln!(writer, "  project: {}", options.name)?;
    writeln!(writer, "  target:  {}", project_dir.display())?;
    write!(writer, "Continue? [y/N] ")?;
    writer.flush()?;

    let mut answer = String::new();
    reader.read_line(&mut answer)?;
    if matches!(answer.trim().to_ascii_lowercase().as_str(), "y" | "yes") {
        return Ok(());
    }

    Err(ScaffoldCommandError::Cancelled)
}

// Validate project names before they become directory and package identifiers.
fn validate_project_name(name: &str) -> Result<(), ScaffoldCommandError> {
    let mut previous_underscore = false;
    for (index, ch) in name.chars().enumerate() {
        let valid = ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_';
        if !valid {
            return Err(ScaffoldCommandError::InvalidProjectName(name.to_string()));
        }
        if index == 0 && !ch.is_ascii_lowercase() {
            return Err(ScaffoldCommandError::InvalidProjectName(name.to_string()));
        }
        if ch == '_' && previous_underscore {
            return Err(ScaffoldCommandError::InvalidProjectName(name.to_string()));
        }
        previous_underscore = ch == '_';
    }

    if name.is_empty() || name.ends_with('_') {
        return Err(ScaffoldCommandError::InvalidProjectName(name.to_string()));
    }

    Ok(())
}

// Write one scaffold file without overwriting any user-owned file.
fn write_new_file(path: &Path, contents: &str) -> Result<(), ScaffoldCommandError> {
    if path.exists() {
        return Err(ScaffoldCommandError::TargetExists(
            path.display().to_string(),
        ));
    }
    fs::write(path, contents).map_err(ScaffoldCommandError::from)
}

// Render the minimal Canic config for one scaffolded fleet.
fn canic_toml(name: &str) -> String {
    format!(
        r#"# Minimal Canic fleet config.

controllers = []
app_index = []

[fleet]
name = "{name}"

[subnets.prime]
auto_create = []
subnet_index = []

[subnets.prime.canisters.root]
kind = "root"
"#
    )
}

// Render the scaffolded root canister package manifest.
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

const ROOT_BUILD_RS: &str = r#"fn main() {
    canic::build_root!("../canic.toml");
}
"#;

const ROOT_LIB_RS: &str = r"#![allow(clippy::unused_async)]

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
            OsString::from("tmp_canisters"),
        ])
        .expect("parse scaffold options");

        assert_eq!(options.name, "my_app");
        assert_eq!(options.canisters_dir, PathBuf::from("tmp_canisters"));
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
            canisters_dir: root.join("canisters"),
            yes: false,
        };
        let mut output = Vec::new();

        confirm_scaffold(&options, io::Cursor::new(b"y\n"), &mut output).expect("confirm scaffold");

        let output = String::from_utf8(output).expect("utf8 prompt");
        assert!(output.contains("target:"));
        assert!(output.contains("canisters/my_app"));
    }

    // Ensure confirmation defaults to no on empty input.
    #[test]
    fn confirm_scaffold_rejects_empty_response() {
        let root = temp_dir("canic-cli-scaffold-confirm-no");
        let options = ScaffoldOptions {
            name: "my_app".to_string(),
            canisters_dir: root.join("canisters"),
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
                Err(ScaffoldCommandError::InvalidProjectName(_))
            ));
        }
    }

    // Ensure scaffold writes the expected minimal root files.
    #[test]
    fn scaffold_project_writes_root_files() {
        let root = temp_dir("canic-cli-scaffold");
        let options = ScaffoldOptions {
            name: "my_app".to_string(),
            canisters_dir: root.join("canisters"),
            yes: true,
        };

        let result = scaffold_project(&options).expect("scaffold project");
        let config = fs::read_to_string(&result.config_path).expect("read config");
        let root_lib =
            fs::read_to_string(result.root_dir.join("src/lib.rs")).expect("read root lib");
        let root_manifest =
            fs::read_to_string(result.root_dir.join("Cargo.toml")).expect("read root manifest");

        fs::remove_dir_all(root).expect("remove scaffold temp root");
        assert!(config.contains("name = \"my_app\""));
        assert!(config.contains("[subnets.prime.canisters.root]"));
        assert!(root_manifest.contains("version = \"0.1.0\""));
        assert!(root_manifest.contains("canic = { version = \""));
        assert!(!root_manifest.contains("workspace = true"));
        assert!(root_lib.contains("canic::start_root!();"));
    }

    // Ensure scaffold refuses to overwrite an existing project directory.
    #[test]
    fn scaffold_project_rejects_existing_target() {
        let root = temp_dir("canic-cli-scaffold-existing");
        let options = ScaffoldOptions {
            name: "my_app".to_string(),
            canisters_dir: root.join("canisters"),
            yes: true,
        };
        fs::create_dir_all(options.canisters_dir.join("my_app")).expect("create existing target");

        let err = scaffold_project(&options).expect_err("existing scaffold should fail");

        fs::remove_dir_all(root).expect("remove scaffold temp root");
        assert!(matches!(err, ScaffoldCommandError::TargetExists(_)));
    }
}
