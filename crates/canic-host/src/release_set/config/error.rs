//! Module: release_set::config::error
//!
//! Responsibility: classify fleet-configuration projection and mutation failures.
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
/// FleetConfigOperation
///
/// Bounded configuration operation attached to core parsing failures.
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FleetConfigOperation {
    AttachRole,
    DeclareRole,
    Project,
    RenameRole,
}

///
/// FleetConfigIoOperation
///
/// Filesystem operation retained with its path and original source.
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FleetConfigIoOperation {
    ReadConfig,
    ReadPackageManifest,
    RestoreConfig,
    WriteConfig,
    WritePackageManifest,
}

impl Display for FleetConfigIoOperation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::ReadConfig => "read fleet config",
            Self::ReadPackageManifest => "read package manifest",
            Self::RestoreConfig => "restore fleet config",
            Self::WriteConfig => "write fleet config",
            Self::WritePackageManifest => "write package manifest",
        })
    }
}

///
/// FleetConfigNameField
///
/// Input-name family used by typed fleet mutation validation.
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FleetConfigNameField {
    Package,
    Role,
    Subnet,
}

impl Display for FleetConfigNameField {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Package => "package",
            Self::Role => "role",
            Self::Subnet => "subnet",
        })
    }
}

///
/// FleetConfigNameIssue
///
/// Reason a bounded fleet mutation name is invalid.
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FleetConfigNameIssue {
    Empty,
    InvalidCharacters,
}

impl Display for FleetConfigNameIssue {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Empty => "must not be empty",
            Self::InvalidCharacters => "must contain only ASCII letters, numbers, '_' or '-'",
        })
    }
}

///
/// FleetConfigDeclaration
///
/// Required declaration absent from a fleet configuration operation.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FleetConfigDeclaration {
    FleetName,
    Role { fleet: String, role: String },
}

impl Display for FleetConfigDeclaration {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FleetName => formatter.write_str("missing required [fleet].name in canic.toml"),
            Self::Role { fleet, role } => write!(formatter, "role {fleet}.{role} is not declared"),
        }
    }
}

///
/// FleetConfigMutationConflict
///
/// Existing configuration state that blocks a requested role mutation.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FleetConfigMutationConflict {
    RoleAlreadyAttached { fleet: String, role: String },
    RoleAlreadyDeclared { fleet: String, role: String },
    RootRoleAttach,
    RootRoleDeclare,
    RootRoleRename,
    SameRoleRename,
}

impl Display for FleetConfigMutationConflict {
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
/// FleetConfigPackageIssue
///
/// Generated package metadata invariant violated by a role rename.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FleetConfigPackageIssue {
    MetadataMissing,
    MetadataMismatch {
        expected_fleet: String,
        expected_role: String,
    },
}

impl Display for FleetConfigPackageIssue {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MetadataMissing => {
                formatter.write_str("updated manifest would remove [package.metadata.canic]")
            }
            Self::MetadataMismatch {
                expected_fleet,
                expected_role,
            } => write!(
                formatter,
                "updated manifest would not contain expected Canic metadata fleet={expected_fleet:?} role={expected_role:?}"
            ),
        }
    }
}

///
/// FleetConfigTomlOperation
///
/// TOML document family whose parser returned the retained source error.
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FleetConfigTomlOperation {
    ParseFleetIdentity,
    ParsePackageManifest,
}

///
/// FleetConfigError
///
/// Typed failure boundary for fleet configuration projections and mutations.
///

#[derive(Debug, ThisError)]
pub enum FleetConfigError {
    #[error("invalid {}: {source}", path.display())]
    ConfigInvalid {
        path: PathBuf,
        #[source]
        source: Box<Self>,
    },

    #[error("{source}")]
    CoreConfig {
        operation: FleetConfigOperation,
        #[source]
        source: ConfigError,
    },

    #[error("{declaration}")]
    DeclarationMissing { declaration: FleetConfigDeclaration },

    #[error("selected config declares fleet {actual:?}, not {expected:?}")]
    FleetMismatch { actual: String, expected: String },

    #[error("kind must be one of: service, singleton, shard, replica, instance")]
    InvalidKind { kind: String },

    #[error("{field} {issue}")]
    InvalidName {
        field: FleetConfigNameField,
        issue: FleetConfigNameIssue,
        value: String,
    },

    #[error("{detail}")]
    InvalidTableHeader { detail: &'static str },

    #[error("failed to {operation} {}: {source}", path.display())]
    Io {
        operation: FleetConfigIoOperation,
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("{conflict}")]
    MutationConflict {
        conflict: FleetConfigMutationConflict,
    },

    #[error("updated {}: {issue}", path.display())]
    PackageMetadataInvalid {
        path: PathBuf,
        issue: FleetConfigPackageIssue,
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
        operation: FleetConfigTomlOperation,
        #[source]
        source: toml::de::Error,
    },
}

impl FleetConfigError {
    pub(super) fn at_config_path(self, path: &Path) -> Self {
        match self {
            Self::ConfigInvalid { .. } | Self::Io { .. } => self,
            source => Self::ConfigInvalid {
                path: path.to_path_buf(),
                source: Box::new(source),
            },
        }
    }

    pub(super) fn io(operation: FleetConfigIoOperation, path: &Path, source: io::Error) -> Self {
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
