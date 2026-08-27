//! Module: apps::options
//! Responsibility: parse typed `canic app` command options from Clap matches.
//! Does not own: command dispatch, filesystem mutation, report rendering, or host operations.
//! Boundary: typed CLI request extraction for the app command family.

use crate::cli::{
    clap::{parse_matches, required_string, string_option_or_else},
    defaults::local_environment,
};
use std::ffi::OsString;

use super::{
    AppCommandError,
    command::{
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
    pub(super) component_spec: String,
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
    pub(super) fn parse<I>(args: I) -> Result<Self, AppCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_matches(app_role_attach_command(), args)
            .map_err(|_| AppCommandError::Usage(role_attach_usage()))?;

        Ok(Self {
            app: required_string(&matches, "app"),
            role: required_string(&matches, "role"),
            component_spec: required_string(&matches, "component-spec"),
            kind: required_string(&matches, "kind"),
            dry_run: matches.get_flag("dry-run"),
        })
    }
}

impl RoleRenameOptions {
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
