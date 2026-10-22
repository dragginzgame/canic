//! Module: apps::workspace
//!
//! Responsibility: plan and publish workspace membership for App deletion.
//! Does not own: discovery, confirmation or source scaffolding.
//! Boundary: publishes the reviewed membership before irreversible deletion.

use super::AppCommandError;
use ic_host_fs::{
    durable::{PublicationMode, WriteOptions, write_bytes},
    path::canonicalize_allow_missing,
};
use serde::{Deserialize, Serialize};
use std::{
    fs, io,
    path::{Path, PathBuf},
};

/// The exact reviewed manifest and membership removed with one App.
pub(super) struct WorkspaceRemoval {
    path: PathBuf,
    target: PathBuf,
    original: Option<String>,
    updated: Option<String>,
    /// Exact member entries shown in the deletion preview and confirmation.
    pub(super) removed: Vec<String>,
}

/// Deserialize only the owning Cargo workspace fields, preserving all other bytes.
#[derive(Deserialize)]
struct WorkspaceDocument {
    workspace: WorkspaceLists,
}

/// Cargo's two lists that must not retain a deleted package directory.
#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
struct WorkspaceLists {
    members: Option<toml::Spanned<Vec<String>>>,
    default_members: Option<toml::Spanned<Vec<String>>>,
}

impl WorkspaceRemoval {
    pub(super) fn plan(root: &Path, target: &Path) -> io::Result<Self> {
        let path = root.join("Cargo.toml");
        let original = match fs::read_to_string(&path) {
            Ok(source) => Some(source),
            Err(error) if error.kind() == io::ErrorKind::NotFound => None,
            Err(error) => return Err(error),
        };
        let mut removed = Vec::new();
        let updated = original
            .as_deref()
            .map(|source| remove_members(source, root, target, &mut removed))
            .transpose()?;
        removed.sort();
        removed.dedup();
        Ok(Self {
            path,
            target: target.to_path_buf(),
            original,
            updated,
            removed,
        })
    }

    pub(super) fn delete(&self) -> Result<(), AppCommandError> {
        let current = match fs::read_to_string(&self.path) {
            Ok(source) => Some(source),
            Err(error) if error.kind() == io::ErrorKind::NotFound => None,
            Err(error) => return Err(error.into()),
        };
        if current != self.original {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Cargo.toml changed after App deletion was reviewed; review again",
            )
            .into());
        }
        if self.updated != self.original {
            write_bytes(
                &self.path,
                WriteOptions {
                    mode: PublicationMode::Replace,
                    permissions: 0o666,
                },
                self.updated
                    .as_ref()
                    .expect("changed existing manifest")
                    .as_bytes(),
            )
            .map_err(canic_host::publication::ops::io_error)?;
        }
        fs::remove_dir_all(&self.target).map_err(|source| AppCommandError::DeleteIncomplete {
            path: self.target.clone(),
            source,
        })
    }
}

fn remove_members(
    source: &str,
    root: &Path,
    target: &Path,
    removed: &mut Vec<String>,
) -> io::Result<String> {
    let document: WorkspaceDocument = toml::from_str(source)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    let target = target.canonicalize()?;
    let mut edits = Vec::new();
    for list in [
        document.workspace.members,
        document.workspace.default_members,
    ]
    .into_iter()
    .flatten()
    {
        let mut retained = Vec::new();
        for member in list.get_ref() {
            let selected = canonicalize_allow_missing(Path::new(member), root, usize::MAX)?;
            if selected.starts_with(&target) {
                removed.push(member.clone());
            } else {
                retained.push(member);
            }
        }
        if retained.len() != list.get_ref().len() {
            let mut value = String::new();
            retained
                .serialize(toml::ser::ValueSerializer::new(&mut value))
                .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
            edits.push((list.span(), value));
        }
    }
    let mut updated = source.to_string();
    edits.sort_by_key(|(span, _)| std::cmp::Reverse(span.start));
    for (span, value) in edits {
        updated.replace_range(span, &value);
    }
    Ok(updated)
}

#[cfg(test)]
mod tests;
