//! Workspace and ICP CLI root discovery helpers for downstream install tooling.

use std::{
    fs, io,
    path::{Path, PathBuf},
};
use thiserror::Error as ThisError;

const WORKSPACE_MANIFEST_RELATIVE: &str = "Cargo.toml";
const ICP_CONFIG_FILE: &str = "icp.yaml";

/// Typed failure while locating a Cargo workspace or ICP project root.
#[derive(Debug, ThisError)]
pub enum WorkspaceDiscoveryError {
    #[error("failed to resolve current directory: {0}")]
    CurrentDirectory(#[source] io::Error),

    #[error("failed to inspect discovery path {}: {source}", path.display())]
    Inspect {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("discovery path must be a regular file or directory: {}", path.display())]
    UnsupportedPath { path: PathBuf },

    #[error("expected a regular project file at {}", path.display())]
    ExpectedFile { path: PathBuf },

    #[error("failed to read Cargo manifest {}: {source}", path.display())]
    ReadManifest {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("failed to parse Cargo manifest {}: {source}", path.display())]
    ParseManifest {
        path: PathBuf,
        #[source]
        source: Box<toml::de::Error>,
    },

    #[error("failed to canonicalize project root {}: {source}", path.display())]
    Canonicalize {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
}

// Resolve the nearest Cargo workspace root from a starting file or directory path.
pub fn discover_workspace_root_from(
    path: &Path,
) -> Result<Option<PathBuf>, WorkspaceDiscoveryError> {
    let start = discovery_start(path)?;

    for candidate in start.ancestors() {
        let manifest_path = candidate.join(WORKSPACE_MANIFEST_RELATIVE);
        if !project_file_exists(&manifest_path)? {
            continue;
        }

        let manifest = fs::read_to_string(&manifest_path).map_err(|source| {
            WorkspaceDiscoveryError::ReadManifest {
                path: manifest_path.clone(),
                source,
            }
        })?;
        if manifest_declares_workspace(&manifest).map_err(|source| {
            WorkspaceDiscoveryError::ParseManifest {
                path: manifest_path,
                source: Box::new(source),
            }
        })? {
            return Ok(Some(candidate.to_path_buf()));
        }
    }

    Ok(None)
}

fn manifest_declares_workspace(source: &str) -> Result<bool, toml::de::Error> {
    Ok(toml::from_str::<toml::Value>(source)?
        .get("workspace")
        .is_some())
}

// Resolve the nearest ICP CLI root from a starting file or directory path.
pub fn discover_icp_root_from(path: &Path) -> Result<Option<PathBuf>, WorkspaceDiscoveryError> {
    let start = discovery_start(path)?;

    for candidate in start.ancestors() {
        let icp_config = candidate.join(ICP_CONFIG_FILE);
        if project_file_exists(&icp_config)? {
            return Ok(Some(candidate.to_path_buf()));
        }
    }

    Ok(None)
}

fn discovery_start(path: &Path) -> Result<PathBuf, WorkspaceDiscoveryError> {
    let metadata = fs::metadata(path).map_err(|source| WorkspaceDiscoveryError::Inspect {
        path: path.to_path_buf(),
        source,
    })?;
    let canonical =
        path.canonicalize()
            .map_err(|source| WorkspaceDiscoveryError::Canonicalize {
                path: path.to_path_buf(),
                source,
            })?;
    if metadata.is_dir() {
        return Ok(canonical);
    }
    if metadata.is_file() {
        return canonical.parent().map(Path::to_path_buf).ok_or_else(|| {
            WorkspaceDiscoveryError::UnsupportedPath {
                path: path.to_path_buf(),
            }
        });
    }
    Err(WorkspaceDiscoveryError::UnsupportedPath {
        path: path.to_path_buf(),
    })
}

fn project_file_exists(path: &Path) -> Result<bool, WorkspaceDiscoveryError> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(source) if source.kind() == io::ErrorKind::NotFound => return Ok(false),
        Err(source) => {
            return Err(WorkspaceDiscoveryError::Inspect {
                path: path.to_path_buf(),
                source,
            });
        }
    };
    if metadata.file_type().is_symlink() {
        return match fs::metadata(path) {
            Ok(metadata) if metadata.is_file() => Ok(true),
            Ok(_) => Err(WorkspaceDiscoveryError::ExpectedFile {
                path: path.to_path_buf(),
            }),
            Err(source) => Err(WorkspaceDiscoveryError::Inspect {
                path: path.to_path_buf(),
                source,
            }),
        };
    }
    if metadata.is_file() {
        Ok(true)
    } else {
        Err(WorkspaceDiscoveryError::ExpectedFile {
            path: path.to_path_buf(),
        })
    }
}

// Normalize a workspace-relative path against the chosen workspace root.
pub fn normalize_workspace_path(workspace_root: &Path, path: PathBuf) -> PathBuf {
    if path.is_absolute() {
        path
    } else {
        workspace_root.join(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::temp_dir;

    struct TempProject {
        path: PathBuf,
    }

    impl TempProject {
        fn new(name: &str) -> Self {
            let path = temp_dir(name);
            fs::create_dir_all(&path).expect("create temp project");
            Self { path }
        }
    }

    impl Drop for TempProject {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    #[test]
    fn workspace_detection_parses_toml_structure() {
        assert!(manifest_declares_workspace("[workspace]\nmembers = []\n").expect("manifest"));
        assert!(manifest_declares_workspace("[ workspace ]\nmembers = []\n").expect("manifest"));
        assert!(
            !manifest_declares_workspace("description = \"example containing [workspace] text\"\n")
                .expect("manifest")
        );
        assert!(manifest_declares_workspace("[workspace\n").is_err());
    }

    #[test]
    fn root_discovery_accepts_a_file_start_and_returns_canonical_roots() {
        let project = TempProject::new("canic-workspace-root-discovery");
        let nested = project.path.join("backend/src/lib.rs");
        fs::create_dir_all(nested.parent().expect("nested parent")).expect("create nested dir");
        fs::write(&nested, "").expect("write nested file");
        fs::write(
            project.path.join(WORKSPACE_MANIFEST_RELATIVE),
            "[workspace]\nmembers = []\n",
        )
        .expect("write workspace manifest");
        fs::write(project.path.join(ICP_CONFIG_FILE), "").expect("write ICP config");
        let expected = project.path.canonicalize().expect("canonical project");

        assert_eq!(
            discover_workspace_root_from(&nested).expect("discover workspace"),
            Some(expected.clone())
        );
        assert_eq!(
            discover_icp_root_from(&nested).expect("discover ICP root"),
            Some(expected)
        );
    }

    #[test]
    fn workspace_discovery_preserves_manifest_parse_cause() {
        let project = TempProject::new("canic-workspace-invalid-manifest");
        fs::write(
            project.path.join(WORKSPACE_MANIFEST_RELATIVE),
            "[workspace\n",
        )
        .expect("write invalid manifest");

        let error =
            discover_workspace_root_from(&project.path).expect_err("invalid manifest must fail");
        let WorkspaceDiscoveryError::ParseManifest { path, .. } = error else {
            panic!("expected typed manifest parse error");
        };

        assert_eq!(path, project.path.join(WORKSPACE_MANIFEST_RELATIVE));
    }

    #[test]
    fn icp_discovery_rejects_non_file_project_config() {
        let project = TempProject::new("canic-workspace-invalid-icp-config");
        let config = project.path.join(ICP_CONFIG_FILE);
        fs::create_dir_all(&config).expect("create invalid ICP config directory");

        let error = discover_icp_root_from(&project.path)
            .expect_err("non-file ICP config must fail discovery");

        std::assert_matches!(
            error,
            WorkspaceDiscoveryError::ExpectedFile { path } if path == config
        );
    }

    #[cfg(unix)]
    #[test]
    fn icp_discovery_accepts_file_symlinks_and_rejects_dangling_markers() {
        use std::os::unix::fs::symlink;

        let project = TempProject::new("canic-workspace-symlinked-icp-config");
        let target = project.path.join("project-icp.yaml");
        let config = project.path.join(ICP_CONFIG_FILE);
        fs::write(&target, "").expect("write ICP config target");
        symlink(&target, &config).expect("link ICP config");

        assert_eq!(
            discover_icp_root_from(&project.path).expect("discover symlinked ICP config"),
            Some(project.path.canonicalize().expect("canonical project"))
        );

        fs::remove_file(&target).expect("remove ICP config target");
        let error = discover_icp_root_from(&project.path)
            .expect_err("dangling ICP config marker must fail discovery");
        std::assert_matches!(
            error,
            WorkspaceDiscoveryError::Inspect { path, source }
                if path == config && source.kind() == io::ErrorKind::NotFound
        );
    }
}
