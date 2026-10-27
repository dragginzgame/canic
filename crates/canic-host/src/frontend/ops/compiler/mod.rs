//! Compile admitted Wire sources with prepared, pinned tools and shared artifact custody.
//! This owner never downloads tools or selects Fleet/release authority.

use crate::{artifact_io::WireBindings, frontend::ops::MAX_FRONTEND_BUNDLE_BYTES};
use ic_build::artifacts::{
    ArtifactCacheOutcome, ArtifactCachePreparation, ArtifactCacheSpec, prepare_artifact_cache,
};
use ic_host_fs::read::read_file_no_follow;
use ic_host_process::tool::{OutputLimit, OutputLimits, ToolError, capture_command};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::Duration,
};

#[cfg(test)]
mod tests;

const PINS: &str = include_str!("../../../../examples/frontend-consumer/package.json");
const CONFIG: &str = r#"{"compilerOptions":{"strict":true,"target":"ES2022","module":"ES2022","moduleResolution":"Bundler","lib":["ES2022","DOM"],"types":[],"typeRoots":[],"declaration":true,"noEmitOnError":true,"outDir":"compiled"},"files":["bindings.ts","canic-adapter.ts","identities.ts","wire-adapter.ts","wire-runtime.ts"]}"#;
const MODULES: [&str; 5] = [
    "bindings",
    "canic-adapter",
    "identities",
    "wire-adapter",
    "wire-runtime",
];

///
/// BrowserCompiler
///
/// Explicit operator-selected tools; package preparation remains separate from export.
///

pub struct BrowserCompiler {
    pub node: PathBuf,
    /// Prepared TypeScript package directory containing package.json, bin/tsc and lib/.
    pub typescript: PathBuf,
}

///
/// CompiledBrowserBindings
///
/// Complete immutable compiler output and its retained input/tool identity.
///

pub struct CompiledBrowserBindings {
    pub acquisition: ArtifactCacheOutcome,
    pub files: BTreeMap<String, String>,
}

///
/// BrowserCompilerError
///
/// Typed compiler admission, execution and artifact failures.
///

#[derive(Debug, thiserror::Error)]
pub enum BrowserCompilerError {
    #[error(transparent)]
    Cache(#[from] ic_build::artifacts::ArtifactCacheError),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error("browser compiler output inventory is incomplete or exceeds its bound")]
    Output,

    #[error("browser compiler does not match the selected Node/TypeScript pins")]
    Selection,

    #[error(transparent)]
    Tool(#[from] ToolError),
}

/// Compile exact admitted sources; reuse binds source bytes, compiler tree, executable and options.
/// The destination is a materialized JSON file map, not the final frontend manifest.
pub fn compile(
    cache: &Path,
    destination: &Path,
    compiler: &BrowserCompiler,
    bindings: &WireBindings,
) -> Result<CompiledBrowserBindings, BrowserCompilerError> {
    let node = compiler.node.canonicalize()?;
    let typescript = compiler.typescript.canonicalize()?;
    admit(&node, &typescript)?;
    let mut sources = bindings.browser_modules.clone();
    if sources
        .insert("bindings.ts".into(), bindings.typescript.clone())
        .is_some()
        || sources.keys().map(String::as_str).ne(MODULES
            .map(|name| format!("{name}.ts"))
            .iter()
            .map(String::as_str))
    {
        return Err(BrowserCompilerError::Output);
    }
    let source_bytes = serde_json::to_vec(&sources)?;
    if source_bytes.len() > MAX_FRONTEND_BUNDLE_BYTES {
        return Err(BrowserCompilerError::Output);
    }
    let spec = ArtifactCacheSpec::new(cache, "canic-browser", "canic.browser.v1")
        .with_tool("node", &node)
        .with_input("typescript", &typescript)
        .with_identity_bytes("sources", &source_bytes)
        .with_identity_bytes("schema", &bindings.schema_sha256)
        .with_identity_bytes("options", CONFIG.as_bytes())
        .with_identity_bytes("pins", PINS.as_bytes())
        .with_identity_bytes("recipe", include_bytes!("mod.rs"))
        .with_environment(
            crate::build_environment::inputs()
                .into_iter()
                .filter(|(name, _)| name != "NODE_OPTIONS" && name != "NODE_PATH"),
        )
        .with_unset_environment(["NODE_OPTIONS", "NODE_PATH"])
        .with_output("bundle", destination);
    let acquisition = match prepare_artifact_cache(&spec)? {
        ArtifactCachePreparation::Reused(record) => ArtifactCacheOutcome::Reused(record),
        ArtifactCachePreparation::Build(transaction) => {
            let work = transaction.staging_directory().join("compiler");
            fs::create_dir(&work)?;
            for (name, source) in &sources {
                fs::write(work.join(name), source)?;
            }
            fs::write(work.join("tsconfig.json"), CONFIG)?;
            let mut command = command(&node);
            command
                .arg(typescript.join("bin/tsc"))
                .args(["--project", "tsconfig.json", "--pretty", "false"])
                .current_dir(&work);
            capture_command(&mut command, limits())?
                .require_success()?
                .require_complete()?;
            let mut files = sources;
            for module in MODULES {
                for suffix in ["js", "d.ts"] {
                    let name = format!("{module}.{suffix}");
                    let bytes = read_file_no_follow(
                        &work.join("compiled").join(&name),
                        crate::frontend::ops::MAX_FRONTEND_FILE_BYTES,
                    )
                    .map_err(std::io::Error::from)?;
                    files.insert(
                        name,
                        String::from_utf8(bytes).map_err(|_| BrowserCompilerError::Output)?,
                    );
                }
            }
            files.insert("package.json".into(), "{\"type\":\"module\"}\n".into());
            let bytes = serde_json::to_vec(&files)?;
            if bytes.len() > MAX_FRONTEND_BUNDLE_BYTES {
                return Err(BrowserCompilerError::Output);
            }
            fs::write(transaction.output_path("bundle")?, bytes)?;
            fs::remove_dir_all(&work)?;
            transaction.commit()?
        }
    };
    let artifact = acquisition
        .record()
        .artifacts()
        .iter()
        .find(|artifact| artifact.name() == "bundle")
        .ok_or(BrowserCompilerError::Output)?;
    let bytes = read_file_no_follow(artifact.path(), MAX_FRONTEND_BUNDLE_BYTES)
        .map_err(std::io::Error::from)?;
    let files: BTreeMap<String, String> = serde_json::from_slice(&bytes)?;
    Ok(CompiledBrowserBindings { acquisition, files })
}

fn admit(node: &Path, typescript: &Path) -> Result<(), BrowserCompilerError> {
    let pins: serde_json::Value = serde_json::from_str(PINS)?;
    let version = pins["devDependencies"]["typescript"]
        .as_str()
        .ok_or(BrowserCompilerError::Selection)?;
    let expected_node = pins["engines"]["node"]
        .as_str()
        .ok_or(BrowserCompilerError::Selection)?;
    let package: serde_json::Value = serde_json::from_slice(
        &read_file_no_follow(&typescript.join("package.json"), 64 * 1024)
            .map_err(std::io::Error::from)?,
    )?;
    if package["name"] != "typescript" || package["version"] != version {
        return Err(BrowserCompilerError::Selection);
    }
    for (mut invocation, expected) in [
        (
            {
                let mut invocation = command(node);
                invocation.arg("--version");
                invocation
            },
            format!("v{expected_node}\n"),
        ),
        (
            {
                let mut invocation = command(node);
                invocation.arg(typescript.join("bin/tsc")).arg("--version");
                invocation
            },
            format!("Version {version}\n"),
        ),
    ] {
        let output = capture_command(&mut invocation, limits())?
            .require_success()?
            .require_complete()?;
        if output.stdout != expected.as_bytes() {
            return Err(BrowserCompilerError::Selection);
        }
    }
    Ok(())
}

fn command(node: &Path) -> Command {
    let mut command = crate::build_environment::command(node);
    command.env_remove("NODE_OPTIONS").env_remove("NODE_PATH");
    command
}

const fn limits() -> OutputLimits {
    OutputLimits {
        stdout: OutputLimit::Terminate(64 * 1024),
        stderr: OutputLimit::Terminate(64 * 1024),
        timeout: Some(Duration::from_secs(120)),
    }
}
