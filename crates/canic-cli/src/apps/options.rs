//! Module: apps::options
//! Responsibility: parse typed `canic app` command options from Clap matches.
//! Does not own: command dispatch, filesystem mutation, report rendering, or host operations.
//! Boundary: typed CLI request extraction for the app command family.

use crate::cli::{
    clap::{parse_matches, path_option, required_string, required_typed, string_option_or_else},
    defaults::local_environment,
};
use canic_host::adoption::AdoptionProfileV1;
use std::{ffi::OsString, path::PathBuf};

use super::{
    AppCommandError,
    command::{
        EVIDENCE_ENVELOPE_ARG, JSON_ARG, adoption_report_usage, app_adoption_report_command,
        app_check_command, app_delete_command, app_list_command, app_role_attach_command,
        app_role_declare_command, app_role_inspect_command, app_role_list_command,
        app_role_rename_command, check_usage, delete_usage, list_usage, role_attach_usage,
        role_declare_usage, role_inspect_usage, role_list_usage, role_rename_usage,
    },
};

///
/// AppOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct AppOptions {
    pub(super) environment: String,
}

///
/// DeleteAppOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct DeleteAppOptions {
    pub(super) app: String,
    pub(super) dry_run: bool,
}

///
/// AppCheckOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct AppCheckOptions {
    pub(super) app: String,
}

///
/// RoleListOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct RoleListOptions {
    pub(super) app: String,
}

///
/// RoleInspectOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct RoleInspectOptions {
    pub(super) app: String,
    pub(super) role: String,
}

///
/// RoleDeclareOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct RoleDeclareOptions {
    pub(super) app: String,
    pub(super) role: String,
    pub(super) package: String,
    pub(super) dry_run: bool,
}

///
/// RoleAttachOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct RoleAttachOptions {
    pub(super) app: String,
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
    pub(super) app: String,
    pub(super) old_role: String,
    pub(super) new_role: String,
    pub(super) dry_run: bool,
}

///
/// AdoptionReportOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct AdoptionReportOptions {
    pub(super) app: String,
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum AdoptionReportFormat {
    Text,
    Json,
    EnvelopeJson,
}

impl AppOptions {
    pub(super) fn parse<I>(args: I) -> Result<Self, AppCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_matches(app_list_command(), args)
            .map_err(|_| AppCommandError::Usage(list_usage()))?;

        Ok(Self {
            environment: string_option_or_else(&matches, "environment", local_environment),
        })
    }
}

impl DeleteAppOptions {
    pub(super) fn parse<I>(args: I) -> Result<Self, AppCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_matches(app_delete_command(), args)
            .map_err(|_| AppCommandError::Usage(delete_usage()))?;

        Ok(Self {
            app: required_string(&matches, "app"),
            dry_run: matches.get_flag("dry-run"),
        })
    }
}

impl AppCheckOptions {
    #[cfg(test)]
    pub(super) fn parse_test<I>(args: I) -> Result<Self, AppCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        Self::parse(args)
    }

    pub(super) fn parse<I>(args: I) -> Result<Self, AppCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_matches(app_check_command(), args)
            .map_err(|_| AppCommandError::Usage(check_usage()))?;

        Ok(Self {
            app: required_string(&matches, "app"),
        })
    }
}

impl RoleListOptions {
    #[cfg(test)]
    pub(super) fn parse_test<I>(args: I) -> Result<Self, AppCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        Self::parse(args)
    }

    pub(super) fn parse<I>(args: I) -> Result<Self, AppCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_matches(app_role_list_command(), args)
            .map_err(|_| AppCommandError::Usage(role_list_usage()))?;

        Ok(Self {
            app: required_string(&matches, "app"),
        })
    }
}

impl RoleInspectOptions {
    #[cfg(test)]
    pub(super) fn parse_test<I>(args: I) -> Result<Self, AppCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        Self::parse(args)
    }

    pub(super) fn parse<I>(args: I) -> Result<Self, AppCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_matches(app_role_inspect_command(), args)
            .map_err(|_| AppCommandError::Usage(role_inspect_usage()))?;

        Ok(Self {
            app: required_string(&matches, "app"),
            role: required_string(&matches, "role"),
        })
    }
}

impl RoleDeclareOptions {
    #[cfg(test)]
    pub(super) fn parse_test<I>(args: I) -> Result<Self, AppCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        Self::parse(args)
    }

    pub(super) fn parse<I>(args: I) -> Result<Self, AppCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_matches(app_role_declare_command(), args)
            .map_err(|_| AppCommandError::Usage(role_declare_usage()))?;

        Ok(Self {
            app: required_string(&matches, "app"),
            role: required_string(&matches, "role"),
            package: required_string(&matches, "package"),
            dry_run: matches.get_flag("dry-run"),
        })
    }
}

impl RoleAttachOptions {
    #[cfg(test)]
    pub(super) fn parse_test<I>(args: I) -> Result<Self, AppCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        Self::parse(args)
    }

    pub(super) fn parse<I>(args: I) -> Result<Self, AppCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_matches(app_role_attach_command(), args)
            .map_err(|_| AppCommandError::Usage(role_attach_usage()))?;

        Ok(Self {
            app: required_string(&matches, "app"),
            role: required_string(&matches, "role"),
            subnet: required_string(&matches, "subnet"),
            kind: required_string(&matches, "kind"),
            dry_run: matches.get_flag("dry-run"),
        })
    }
}

impl RoleRenameOptions {
    #[cfg(test)]
    pub(super) fn parse_test<I>(args: I) -> Result<Self, AppCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        Self::parse(args)
    }

    pub(super) fn parse<I>(args: I) -> Result<Self, AppCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_matches(app_role_rename_command(), args)
            .map_err(|_| AppCommandError::Usage(role_rename_usage()))?;

        Ok(Self {
            app: required_string(&matches, "app"),
            old_role: required_string(&matches, "old-role"),
            new_role: required_string(&matches, "new-role"),
            dry_run: matches.get_flag("dry-run"),
        })
    }
}

impl AdoptionReportOptions {
    #[cfg(test)]
    pub(super) fn parse_test<I>(args: I) -> Result<Self, AppCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        Self::parse(args)
    }

    pub(super) fn parse<I>(args: I) -> Result<Self, AppCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_matches(app_adoption_report_command(), args)
            .map_err(|_| AppCommandError::Usage(adoption_report_usage()))?;

        let format = adoption_report_format(
            matches.get_flag(JSON_ARG),
            matches.get_flag(EVIDENCE_ENVELOPE_ARG),
        );
        let build_provenance = path_option(&matches, "build-provenance");
        if build_provenance.is_some() && format != AdoptionReportFormat::EnvelopeJson {
            return Err(AppCommandError::Usage(format!(
                "--build-provenance requires --evidence-envelope\n\n{}",
                adoption_report_usage()
            )));
        }

        Ok(Self {
            app: required_string(&matches, "app"),
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

const fn adoption_report_format(json: bool, evidence_envelope: bool) -> AdoptionReportFormat {
    match (json, evidence_envelope) {
        (true, false) => AdoptionReportFormat::Json,
        (false, true) => AdoptionReportFormat::EnvelopeJson,
        _ => AdoptionReportFormat::Text,
    }
}
