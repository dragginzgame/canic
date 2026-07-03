//! Module: fleets::options
//! Responsibility: parse typed `canic fleet` command options from Clap matches.
//! Does not own: command dispatch, filesystem mutation, report rendering, or host operations.
//! Boundary: typed CLI request extraction for the fleet command family.

use crate::cli::{
    clap::{parse_matches, path_option, required_string, required_typed, string_option_or_else},
    defaults::local_network,
};
use canic_host::adoption::AdoptionProfileV1;
use clap::ValueEnum;
use std::{ffi::OsString, path::PathBuf};

use super::{
    FleetCommandError,
    command::{
        adoption_report_usage, check_usage, delete_usage, fleet_adoption_report_command,
        fleet_check_command, fleet_delete_command, fleet_list_command, fleet_role_attach_command,
        fleet_role_declare_command, fleet_role_inspect_command, fleet_role_list_command,
        fleet_role_rename_command, list_usage, role_attach_usage, role_declare_usage,
        role_inspect_usage, role_list_usage, role_rename_usage,
    },
};

///
/// FleetOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct FleetOptions {
    pub(super) network: String,
}

///
/// DeleteFleetOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct DeleteFleetOptions {
    pub(super) fleet: String,
    pub(super) dry_run: bool,
}

///
/// FleetCheckOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct FleetCheckOptions {
    pub(super) fleet: String,
}

///
/// RoleListOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct RoleListOptions {
    pub(super) fleet: String,
}

///
/// RoleInspectOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct RoleInspectOptions {
    pub(super) fleet: String,
    pub(super) role: String,
}

///
/// RoleDeclareOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct RoleDeclareOptions {
    pub(super) fleet: String,
    pub(super) role: String,
    pub(super) package: String,
    pub(super) dry_run: bool,
}

///
/// RoleAttachOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct RoleAttachOptions {
    pub(super) fleet: String,
    pub(super) role: String,
    pub(super) subnet: String,
    pub(super) kind: String,
    pub(super) dry_run: bool,
}

///
/// RoleRenameOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct RoleRenameOptions {
    pub(super) fleet: String,
    pub(super) old_role: String,
    pub(super) new_role: String,
    pub(super) dry_run: bool,
}

///
/// AdoptionReportOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct AdoptionReportOptions {
    pub(super) fleet: String,
    pub(super) profile: AdoptionProfileV1,
    pub(super) format: AdoptionReportFormat,
    pub(super) deployment_check: Option<PathBuf>,
    pub(super) inventory: Option<PathBuf>,
    pub(super) artifact_manifest: Option<PathBuf>,
    pub(super) cargo_metadata: Option<PathBuf>,
    pub(super) package_metadata: Option<PathBuf>,
    pub(super) build_provenance: Option<PathBuf>,
    pub(super) output: Option<PathBuf>,
}

///
/// AdoptionReportFormat
///

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub(super) enum AdoptionReportFormat {
    Text,
    Json,
    EnvelopeJson,
}

impl FleetOptions {
    pub(super) fn parse<I>(args: I) -> Result<Self, FleetCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_matches(fleet_list_command(), args)
            .map_err(|_| FleetCommandError::Usage(list_usage()))?;

        Ok(Self {
            network: string_option_or_else(&matches, "network", local_network),
        })
    }
}

impl DeleteFleetOptions {
    pub(super) fn parse<I>(args: I) -> Result<Self, FleetCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_matches(fleet_delete_command(), args)
            .map_err(|_| FleetCommandError::Usage(delete_usage()))?;

        Ok(Self {
            fleet: required_string(&matches, "fleet"),
            dry_run: matches.get_flag("dry-run"),
        })
    }
}

impl FleetCheckOptions {
    #[cfg(test)]
    pub(super) fn parse_test<I>(args: I) -> Result<Self, FleetCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        Self::parse(args)
    }

    pub(super) fn parse<I>(args: I) -> Result<Self, FleetCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_matches(fleet_check_command(), args)
            .map_err(|_| FleetCommandError::Usage(check_usage()))?;

        Ok(Self {
            fleet: required_string(&matches, "fleet"),
        })
    }
}

impl RoleListOptions {
    #[cfg(test)]
    pub(super) fn parse_test<I>(args: I) -> Result<Self, FleetCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        Self::parse(args)
    }

    pub(super) fn parse<I>(args: I) -> Result<Self, FleetCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_matches(fleet_role_list_command(), args)
            .map_err(|_| FleetCommandError::Usage(role_list_usage()))?;

        Ok(Self {
            fleet: required_string(&matches, "fleet"),
        })
    }
}

impl RoleInspectOptions {
    #[cfg(test)]
    pub(super) fn parse_test<I>(args: I) -> Result<Self, FleetCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        Self::parse(args)
    }

    pub(super) fn parse<I>(args: I) -> Result<Self, FleetCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_matches(fleet_role_inspect_command(), args)
            .map_err(|_| FleetCommandError::Usage(role_inspect_usage()))?;

        Ok(Self {
            fleet: required_string(&matches, "fleet"),
            role: required_string(&matches, "role"),
        })
    }
}

impl RoleDeclareOptions {
    #[cfg(test)]
    pub(super) fn parse_test<I>(args: I) -> Result<Self, FleetCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        Self::parse(args)
    }

    pub(super) fn parse<I>(args: I) -> Result<Self, FleetCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_matches(fleet_role_declare_command(), args)
            .map_err(|_| FleetCommandError::Usage(role_declare_usage()))?;

        Ok(Self {
            fleet: required_string(&matches, "fleet"),
            role: required_string(&matches, "role"),
            package: required_string(&matches, "package"),
            dry_run: matches.get_flag("dry-run"),
        })
    }
}

impl RoleAttachOptions {
    #[cfg(test)]
    pub(super) fn parse_test<I>(args: I) -> Result<Self, FleetCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        Self::parse(args)
    }

    pub(super) fn parse<I>(args: I) -> Result<Self, FleetCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_matches(fleet_role_attach_command(), args)
            .map_err(|_| FleetCommandError::Usage(role_attach_usage()))?;

        Ok(Self {
            fleet: required_string(&matches, "fleet"),
            role: required_string(&matches, "role"),
            subnet: required_string(&matches, "subnet"),
            kind: required_string(&matches, "kind"),
            dry_run: matches.get_flag("dry-run"),
        })
    }
}

impl RoleRenameOptions {
    #[cfg(test)]
    pub(super) fn parse_test<I>(args: I) -> Result<Self, FleetCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        Self::parse(args)
    }

    pub(super) fn parse<I>(args: I) -> Result<Self, FleetCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_matches(fleet_role_rename_command(), args)
            .map_err(|_| FleetCommandError::Usage(role_rename_usage()))?;

        Ok(Self {
            fleet: required_string(&matches, "fleet"),
            old_role: required_string(&matches, "old-role"),
            new_role: required_string(&matches, "new-role"),
            dry_run: matches.get_flag("dry-run"),
        })
    }
}

impl AdoptionReportOptions {
    #[cfg(test)]
    pub(super) fn parse_test<I>(args: I) -> Result<Self, FleetCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        Self::parse(args)
    }

    pub(super) fn parse<I>(args: I) -> Result<Self, FleetCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_matches(fleet_adoption_report_command(), args)
            .map_err(|_| FleetCommandError::Usage(adoption_report_usage()))?;

        let format = required_typed(&matches, "format");
        let build_provenance = path_option(&matches, "build-provenance");
        if build_provenance.is_some() && format != AdoptionReportFormat::EnvelopeJson {
            return Err(FleetCommandError::Usage(format!(
                "--build-provenance requires --format envelope-json\n\n{}",
                adoption_report_usage()
            )));
        }

        Ok(Self {
            fleet: required_string(&matches, "fleet"),
            profile: required_typed(&matches, "profile"),
            format,
            deployment_check: path_option(&matches, "deployment-check"),
            inventory: path_option(&matches, "inventory"),
            artifact_manifest: path_option(&matches, "artifact-manifest"),
            cargo_metadata: path_option(&matches, "cargo-metadata"),
            package_metadata: path_option(&matches, "package-metadata"),
            build_provenance,
            output: path_option(&matches, "output"),
        })
    }
}
