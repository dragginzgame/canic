//! Module: release_set::config::error
//!
//! Responsibility: classify App-configuration projection and mutation failures.
//! Does not own: configuration policy, TOML mutation, rollback execution, or CLI exits.
//! Boundary: retains typed core, TOML, I/O, mutation, and rollback causes for callers.

use std::{
    fmt::{self, Display},
    io,
    path::{Path, PathBuf},
};

use canic_core::{bootstrap::ConfigError, role_contract::RoleContractFinding};
use thiserror::Error as ThisError;

///
/// AppConfigOperation
///
/// Bounded configuration operation attached to core parsing failures.
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AppConfigOperation {
    AttachRole,
    DeclareRole,
    Project,
    RenameRole,
}

///
/// AppConfigIoOperation
///
/// Filesystem operation retained with its path and original source.
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AppConfigIoOperation {
    ReadConfig,
    ReadPackageManifest,
    RestoreConfig,
    WriteConfig,
    WritePackageManifest,
}

impl Display for AppConfigIoOperation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::ReadConfig => "read App config",
            Self::ReadPackageManifest => "read package manifest",
            Self::RestoreConfig => "restore App config",
            Self::WriteConfig => "write App config",
            Self::WritePackageManifest => "write package manifest",
        })
    }
}

///
/// AppConfigNameField
///
/// Input-name family used by typed fleet mutation validation.
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AppConfigNameField {
    Package,
    Role,
    Subnet,
}

impl Display for AppConfigNameField {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Package => "package",
            Self::Role => "role",
            Self::Subnet => "subnet",
        })
    }
}

///
/// AppConfigNameIssue
///
/// Reason a bounded fleet mutation name is invalid.
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AppConfigNameIssue {
    Empty,
    InvalidCharacters,
    InvalidSnakeCase,
    TooLong { max_bytes: usize },
}

impl Display for AppConfigNameIssue {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => formatter.write_str("must not be empty"),
            Self::InvalidCharacters => formatter.write_str(
                "must begin with an ASCII letter, number, or '_' and contain only ASCII letters, numbers, '_' or '-'",
            ),
            Self::InvalidSnakeCase => formatter.write_str(
                "must use lowercase snake_case beginning with an ASCII letter, with nonempty lowercase alphanumeric words separated by single '_' characters",
            ),
            Self::TooLong { max_bytes } => {
                write!(formatter, "must not exceed {max_bytes} bytes")
            }
        }
    }
}

///
/// AppConfigDeclaration
///
/// Required declaration absent from an App configuration operation.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AppConfigDeclaration {
    AppName,
    Role { fleet: String, role: String },
}

impl Display for AppConfigDeclaration {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AppName => formatter.write_str("missing required [app].name in canic.toml"),
            Self::Role { fleet, role } => write!(formatter, "role {fleet}.{role} is not declared"),
        }
    }
}

///
/// AppConfigMutationConflict
///
/// Existing configuration state that blocks a requested role mutation.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AppConfigMutationConflict {
    RoleAlreadyAttached { fleet: String, role: String },
    RoleAlreadyDeclared { fleet: String, role: String },
    RootRoleAttach,
    RootRoleDeclare,
    RootRoleRename,
    SameRoleRename,
}

impl Display for AppConfigMutationConflict {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RoleAlreadyAttached { fleet, role } => {
                write!(formatter, "role {fleet}.{role} is already attached")
            }
            Self::RoleAlreadyDeclared { fleet, role } => {
                write!(formatter, "role {fleet}.{role} is already declared")
            }
            Self::RootRoleAttach => {
                formatter.write_str("root role must already be attached through root topology")
            }
            Self::RootRoleDeclare => formatter
                .write_str("root role must be attached to topology; declare ordinary roles only"),
            Self::RootRoleRename => {
                formatter.write_str("root role cannot be renamed through fleet role rename")
            }
            Self::SameRoleRename => formatter.write_str("old role and new role must differ"),
        }
    }
}

///
/// AppConfigPackageIssue
///
/// Generated package metadata invariant violated by a role rename.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AppConfigPackageIssue {
    MetadataMissing,
    MetadataMismatch {
        expected_app: String,
        expected_role: String,
    },
}

impl Display for AppConfigPackageIssue {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MetadataMissing => {
                formatter.write_str("updated manifest would remove [package.metadata.canic]")
            }
            Self::MetadataMismatch {
                expected_app,
                expected_role,
            } => write!(
                formatter,
                "updated manifest would not contain expected Canic App identity {expected_app:?} and role {expected_role:?}"
            ),
        }
    }
}

///
/// AppConfigTomlOperation
///
/// TOML document family whose parser returned the retained source error.
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AppConfigTomlOperation {
    ParseAppIdentity,
    ParsePackageManifest,
}

///
/// AppConfigError
///
/// Typed failure boundary for App configuration projections and mutations.
///

#[derive(Debug, ThisError)]
pub enum AppConfigError {
    #[error("invalid {}: {source}", path.display())]
    ConfigInvalid {
        path: PathBuf,
        #[source]
        source: Box<Self>,
    },

    #[error("{source}")]
    CoreConfig {
        operation: AppConfigOperation,
        #[source]
        source: ConfigError,
    },

    #[error("{declaration}")]
    DeclarationMissing { declaration: AppConfigDeclaration },

    #[error("selected config declares App {actual:?}, not {expected:?}")]
    AppMismatch { actual: String, expected: String },

    #[error("kind must be one of: service, singleton, shard, replica, instance")]
    InvalidKind { kind: String },

    #[error("{field} {issue}")]
    InvalidName {
        field: AppConfigNameField,
        issue: AppConfigNameIssue,
        value: String,
    },

    #[error("{detail}")]
    InvalidTableHeader { detail: &'static str },

    #[error("failed to {operation} {}: {source}", path.display())]
    Io {
        operation: AppConfigIoOperation,
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("{conflict}")]
    MutationConflict { conflict: AppConfigMutationConflict },

    #[error("updated {}: {issue}", path.display())]
    PackageMetadataInvalid {
        path: PathBuf,
        issue: AppConfigPackageIssue,
    },

    #[error("{}", format_role_contract_findings(.errors))]
    RoleContractRejected { errors: Vec<RoleContractFinding> },

    #[error("{mutation}; {rollback}")]
    RollbackFailed {
        mutation: Box<Self>,
        rollback: Box<Self>,
    },

    #[error("{source}")]
    Toml {
        operation: AppConfigTomlOperation,
        #[source]
        source: toml::de::Error,
    },
}

impl AppConfigError {
    pub(super) fn at_config_path(self, path: &Path) -> Self {
        match self {
            Self::ConfigInvalid { .. } | Self::Io { .. } => self,
            source => Self::ConfigInvalid {
                path: path.to_path_buf(),
                source: Box::new(source),
            },
        }
    }

    pub(super) fn io(operation: AppConfigIoOperation, path: &Path, source: io::Error) -> Self {
        Self::Io {
            operation,
            path: path.to_path_buf(),
            source,
        }
    }
}

fn format_role_contract_findings(errors: &[RoleContractFinding]) -> String {
    errors
        .iter()
        .map(|finding| {
            format!(
                "{}: {}",
                finding.code(),
                crate::role_contract::finding_detail(finding)
            )
        })
        .collect::<Vec<_>>()
        .join("; ")
}
