//! Module: persistence::artifact_commit
//!
//! Responsibility: durably publish one verified snapshot artifact directory.
//! Does not own: snapshot download, journal transitions, or manifest creation.
//! Boundary: accepts journal-bound sibling paths and expected checksum bytes.

use crate::persistence::PersistenceError;

use std::path::Path;

/// Result of publishing a temporary artifact or recovering its published tree.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ArtifactCommitOutcome {
    Published,
    Recovered,
}

#[cfg(all(
    test,
    any(target_os = "linux", target_os = "android", target_vendor = "apple")
))]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ArtifactCommitBarrier {
    BeforePublication,
    AfterPublicationSync,
}

/// Durably publish or recover one checksum-verified snapshot directory.
pub fn commit_artifact_directory(
    temporary: &Path,
    canonical: &Path,
    expected_checksum: &str,
) -> Result<ArtifactCommitOutcome, PersistenceError> {
    #[cfg(any(target_os = "linux", target_os = "android", target_vendor = "apple"))]
    {
        supported::commit_with_hook(temporary, canonical, expected_checksum, |_, _| Ok(()))
    }

    #[cfg(not(any(target_os = "linux", target_os = "android", target_vendor = "apple")))]
    {
        let _ = (temporary, canonical, expected_checksum);
        Err(PersistenceError::ArtifactCommitUnsupportedPlatform {
            platform: std::env::consts::OS,
        })
    }
}

#[cfg(all(
    test,
    any(target_os = "linux", target_os = "android", target_vendor = "apple")
))]
pub fn commit_artifact_directory_at_barriers(
    temporary: &Path,
    canonical: &Path,
    expected_checksum: &str,
    mut at_barrier: impl FnMut(ArtifactCommitBarrier),
) -> Result<ArtifactCommitOutcome, PersistenceError> {
    supported::commit_with_hook(temporary, canonical, expected_checksum, |step, _| {
        match step {
            supported::ArtifactCommitStep::Publication => {
                at_barrier(ArtifactCommitBarrier::BeforePublication);
            }
            supported::ArtifactCommitStep::PublicationDurable => {
                at_barrier(ArtifactCommitBarrier::AfterPublicationSync);
            }
            _ => {}
        }
        Ok(())
    })
}

#[cfg(any(target_os = "linux", target_os = "android", target_vendor = "apple"))]
mod supported {
    use super::ArtifactCommitOutcome;
    use crate::{artifacts::ArtifactChecksum, persistence::PersistenceError};

    use std::{
        ffi::OsStr,
        fs::{self, File},
        io::{self, Seek, SeekFrom},
        os::unix::ffi::OsStrExt,
        path::{Path, PathBuf},
    };

    use rustix::{
        fd::{AsFd, OwnedFd},
        fs::{self as unix_fs, AtFlags, Dir, FileType, Mode, OFlags, RenameFlags},
    };

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub(super) enum ArtifactCommitStep {
        RegularFileSync,
        NestedDirectorySync,
        RootDirectorySync,
        Publication,
        ParentDirectorySync,
        PublicationDurable,
    }

    pub(super) fn commit_with_hook(
        temporary: &Path,
        canonical: &Path,
        expected_checksum: &str,
        mut at_step: impl FnMut(ArtifactCommitStep, &Path) -> io::Result<()>,
    ) -> Result<ArtifactCommitOutcome, PersistenceError> {
        let (parent, temporary_name, canonical_name) = sibling_paths(temporary, canonical)?;
        let temporary_exists = path_exists_without_following(temporary)?;
        let canonical_exists = path_exists_without_following(canonical)?;
        let parent_fd = open_parent_directory(parent)?;

        match (temporary_exists, canonical_exists) {
            (true, false) => {
                let checksum = sync_tree(&parent_fd, temporary_name, temporary, &mut at_step)?;
                checksum.verify(expected_checksum)?;
                at_step(ArtifactCommitStep::Publication, canonical)?;
                unix_fs::renameat_with(
                    &parent_fd,
                    temporary_name,
                    &parent_fd,
                    canonical_name,
                    RenameFlags::NOREPLACE,
                )
                .map_err(errno_to_io)?;
                at_step(ArtifactCommitStep::ParentDirectorySync, parent)?;
                unix_fs::fsync(&parent_fd).map_err(errno_to_io)?;
                at_step(ArtifactCommitStep::PublicationDurable, canonical)?;
                Ok(ArtifactCommitOutcome::Published)
            }
            (false, true) => {
                let checksum = sync_tree(&parent_fd, canonical_name, canonical, &mut at_step)?;
                checksum.verify(expected_checksum)?;
                at_step(ArtifactCommitStep::ParentDirectorySync, parent)?;
                unix_fs::fsync(&parent_fd).map_err(errno_to_io)?;
                at_step(ArtifactCommitStep::PublicationDurable, canonical)?;
                Ok(ArtifactCommitOutcome::Recovered)
            }
            (true, true) => Err(PersistenceError::ArtifactCommitPathConflict {
                temporary: temporary.display().to_string(),
                canonical: canonical.display().to_string(),
            }),
            (false, false) => Err(PersistenceError::ArtifactCommitPathMissing {
                temporary: temporary.display().to_string(),
                canonical: canonical.display().to_string(),
            }),
        }
    }

    fn sibling_paths<'a>(
        temporary: &'a Path,
        canonical: &'a Path,
    ) -> Result<(&'a Path, &'a OsStr, &'a OsStr), PersistenceError> {
        let invalid_paths = || PersistenceError::ArtifactCommitPathMismatch {
            temporary: temporary.display().to_string(),
            canonical: canonical.display().to_string(),
        };
        let temporary_parent = temporary.parent().ok_or_else(&invalid_paths)?;
        let canonical_parent = canonical.parent().ok_or_else(&invalid_paths)?;
        let temporary_name = temporary.file_name().ok_or_else(&invalid_paths)?;
        let canonical_name = canonical.file_name().ok_or_else(&invalid_paths)?;
        if temporary == canonical || temporary_parent != canonical_parent {
            return Err(invalid_paths());
        }

        Ok((temporary_parent, temporary_name, canonical_name))
    }

    fn path_exists_without_following(path: &Path) -> io::Result<bool> {
        match fs::symlink_metadata(path) {
            Ok(_) => Ok(true),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
            Err(error) => Err(error),
        }
    }

    fn open_parent_directory(path: &Path) -> io::Result<OwnedFd> {
        unix_fs::open(
            path,
            OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC,
            Mode::empty(),
        )
        .map_err(errno_to_io)
    }

    fn sync_tree(
        parent_fd: &impl AsFd,
        directory_name: &OsStr,
        display_root: &Path,
        at_step: &mut impl FnMut(ArtifactCommitStep, &Path) -> io::Result<()>,
    ) -> Result<ArtifactChecksum, PersistenceError> {
        let directory_fd = unix_fs::openat(
            parent_fd,
            directory_name,
            OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            Mode::empty(),
        )
        .map_err(errno_to_io)?;
        let mut checksums = Vec::new();
        sync_directory(
            &directory_fd,
            Path::new(""),
            display_root,
            &mut checksums,
            at_step,
        )?;
        Ok(ArtifactChecksum::from_relative_file_checksums(checksums))
    }

    fn sync_directory(
        directory_fd: &OwnedFd,
        relative_directory: &Path,
        display_root: &Path,
        checksums: &mut Vec<(PathBuf, ArtifactChecksum)>,
        at_step: &mut impl FnMut(ArtifactCommitStep, &Path) -> io::Result<()>,
    ) -> Result<(), PersistenceError> {
        let mut directory = Dir::read_from(directory_fd).map_err(errno_to_io)?;
        while let Some(entry) = directory.read() {
            let entry = entry.map_err(errno_to_io)?;
            let name_bytes = entry.file_name().to_bytes();
            if matches!(name_bytes, b"." | b"..") {
                continue;
            }

            let name = OsStr::from_bytes(name_bytes);
            let relative_path = relative_directory.join(name);
            let display_path = display_root.join(&relative_path);
            let observed =
                unix_fs::statat(directory_fd, entry.file_name(), AtFlags::SYMLINK_NOFOLLOW)
                    .map_err(errno_to_io)?;
            let observed_type = FileType::from_raw_mode(observed.st_mode);
            match observed_type {
                FileType::RegularFile => {
                    let child_fd = open_child(directory_fd, entry.file_name())?;
                    ensure_opened_type(&child_fd, FileType::RegularFile, &display_path)?;
                    let mut file = File::from(child_fd);
                    at_step(ArtifactCommitStep::RegularFileSync, &display_path)?;
                    file.sync_all()?;
                    file.seek(SeekFrom::Start(0))?;
                    checksums.push((relative_path, ArtifactChecksum::from_reader(&mut file)?));
                }
                FileType::Directory => {
                    let child_fd = open_child(directory_fd, entry.file_name())?;
                    ensure_opened_type(&child_fd, FileType::Directory, &display_path)?;
                    sync_directory(&child_fd, &relative_path, display_root, checksums, at_step)?;
                }
                kind => return Err(unsupported_entry(&display_path, kind)),
            }
        }

        let (step, display_path) = if relative_directory.as_os_str().is_empty() {
            (
                ArtifactCommitStep::RootDirectorySync,
                display_root.to_path_buf(),
            )
        } else {
            (
                ArtifactCommitStep::NestedDirectorySync,
                display_root.join(relative_directory),
            )
        };
        at_step(step, &display_path)?;
        unix_fs::fsync(directory_fd).map_err(errno_to_io)?;
        Ok(())
    }

    fn open_child(parent: &impl AsFd, name: &std::ffi::CStr) -> io::Result<OwnedFd> {
        unix_fs::openat(
            parent,
            name,
            OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
            Mode::empty(),
        )
        .map_err(errno_to_io)
    }

    fn ensure_opened_type(
        fd: &impl AsFd,
        expected: FileType,
        path: &Path,
    ) -> Result<(), PersistenceError> {
        let opened = unix_fs::fstat(fd).map_err(errno_to_io)?;
        let actual = FileType::from_raw_mode(opened.st_mode);
        if actual == expected {
            Ok(())
        } else {
            Err(unsupported_entry(path, actual))
        }
    }

    fn unsupported_entry(path: &Path, kind: FileType) -> PersistenceError {
        PersistenceError::UnsupportedArtifactEntry {
            path: path.display().to_string(),
            kind: format!("{kind:?}"),
        }
    }

    fn errno_to_io(error: rustix::io::Errno) -> io::Error {
        io::Error::from_raw_os_error(error.raw_os_error())
    }
}

#[cfg(all(
    test,
    any(target_os = "linux", target_os = "android", target_vendor = "apple")
))]
mod tests {
    use super::{ArtifactCommitOutcome, supported::*};
    use crate::{
        artifacts::ArtifactChecksum, persistence::PersistenceError, test_support::temp_dir,
    };

    use std::{fs, io};

    #[test]
    fn publishes_and_recovers_the_exact_verified_tree() {
        let root = temp_dir("canic-backup-artifact-commit");
        let temporary = root.join("snapshot.tmp");
        let canonical = root.join("snapshot");
        write_tree(&temporary);
        let expected = checksum(&temporary);

        let published = commit_with_hook(&temporary, &canonical, &expected, |_, _| Ok(()))
            .expect("publish artifact");
        assert_eq!(published, ArtifactCommitOutcome::Published);
        assert!(!temporary.exists());
        assert_eq!(checksum(&canonical), expected);

        let recovered = commit_with_hook(&temporary, &canonical, &expected, |_, _| Ok(()))
            .expect("recover artifact");
        assert_eq!(recovered, ArtifactCommitOutcome::Recovered);
        assert_eq!(checksum(&canonical), expected);

        fs::remove_dir_all(root).expect("remove temp root");
    }

    #[test]
    fn rejects_existing_destinations_and_unsupported_entries() {
        let conflict_root = temp_dir("canic-backup-artifact-conflict");
        let conflict_temporary = conflict_root.join("snapshot.tmp");
        let conflict_canonical = conflict_root.join("snapshot");
        write_tree(&conflict_temporary);
        write_tree(&conflict_canonical);
        let expected = checksum(&conflict_temporary);

        let conflict = commit_with_hook(
            &conflict_temporary,
            &conflict_canonical,
            &expected,
            |_, _| Ok(()),
        )
        .expect_err("existing destination must reject");
        std::assert_matches!(
            conflict,
            PersistenceError::ArtifactCommitPathConflict { .. }
        );

        let symlink_root = temp_dir("canic-backup-artifact-symlink");
        let symlink_temporary = symlink_root.join("snapshot.tmp");
        let symlink_canonical = symlink_root.join("snapshot");
        write_tree(&symlink_temporary);
        std::os::unix::fs::symlink(
            symlink_temporary.join("root.txt"),
            symlink_temporary.join("linked.txt"),
        )
        .expect("create artifact symlink");

        let symlink =
            commit_with_hook(&symlink_temporary, &symlink_canonical, &expected, |_, _| {
                Ok(())
            })
            .expect_err("symlink must reject");
        std::assert_matches!(symlink, PersistenceError::UnsupportedArtifactEntry { .. });

        fs::remove_dir_all(conflict_root).expect("remove conflict root");
        fs::remove_dir_all(symlink_root).expect("remove symlink root");
    }

    #[test]
    fn publication_race_cannot_replace_a_new_destination() {
        let root = temp_dir("canic-backup-artifact-publication-race");
        let temporary = root.join("snapshot.tmp");
        let canonical = root.join("snapshot");
        write_tree(&temporary);
        let expected = checksum(&temporary);

        let error = commit_with_hook(&temporary, &canonical, &expected, |step, _| {
            if step == ArtifactCommitStep::Publication {
                fs::create_dir(&canonical)?;
                fs::write(canonical.join("other.txt"), b"other")?;
            }
            Ok(())
        })
        .expect_err("atomic no-replace must reject a publication race");

        std::assert_matches!(
            error,
            PersistenceError::Io(ref source)
                if source.kind() == io::ErrorKind::AlreadyExists
        );
        assert!(temporary.exists());
        assert_eq!(
            fs::read(canonical.join("other.txt")).expect("read raced destination"),
            b"other"
        );

        fs::remove_dir_all(root).expect("remove temp root");
    }

    #[test]
    fn changed_temporary_tree_cannot_be_published_under_an_old_checksum() {
        let root = temp_dir("canic-backup-artifact-changed-before-publication");
        let temporary = root.join("snapshot.tmp");
        let canonical = root.join("snapshot");
        write_tree(&temporary);
        let expected = checksum(&temporary);
        fs::write(temporary.join("root.txt"), b"changed snapshot")
            .expect("change temporary artifact");

        let error = commit_with_hook(&temporary, &canonical, &expected, |_, _| Ok(()))
            .expect_err("changed temporary artifact must reject");

        std::assert_matches!(
            error,
            PersistenceError::Checksum(
                crate::artifacts::ArtifactChecksumError::ChecksumMismatch { .. }
            )
        );
        assert!(temporary.exists());
        assert!(!canonical.exists());

        fs::remove_dir_all(root).expect("remove changed temporary root");
    }

    #[test]
    fn injected_commit_failures_never_expose_a_partial_tree() {
        let steps = [
            ArtifactCommitStep::RegularFileSync,
            ArtifactCommitStep::NestedDirectorySync,
            ArtifactCommitStep::RootDirectorySync,
            ArtifactCommitStep::Publication,
            ArtifactCommitStep::ParentDirectorySync,
            ArtifactCommitStep::PublicationDurable,
        ];

        for step in steps {
            let root = temp_dir(&format!("canic-backup-artifact-failure-{step:?}"));
            let temporary = root.join("snapshot.tmp");
            let canonical = root.join("snapshot");
            write_tree(&temporary);
            let expected = checksum(&temporary);
            let mut failed = false;

            let error = commit_with_hook(&temporary, &canonical, &expected, |current, _| {
                if current == step && !failed {
                    failed = true;
                    return Err(io::Error::other("injected commit failure"));
                }
                Ok(())
            })
            .expect_err("injected step must fail");
            std::assert_matches!(error, PersistenceError::Io(_));

            if matches!(
                step,
                ArtifactCommitStep::ParentDirectorySync | ArtifactCommitStep::PublicationDurable
            ) {
                assert!(!temporary.exists());
                assert_eq!(checksum(&canonical), expected);
                let recovered = commit_with_hook(&temporary, &canonical, &expected, |_, _| Ok(()))
                    .expect("recover post-publication artifact");
                assert_eq!(recovered, ArtifactCommitOutcome::Recovered);
            } else {
                assert!(temporary.exists());
                assert!(!canonical.exists());
            }

            fs::remove_dir_all(root).expect("remove failure root");
        }
    }

    fn write_tree(root: &std::path::Path) {
        fs::create_dir_all(root.join("nested")).expect("create artifact tree");
        fs::write(root.join("root.txt"), b"root snapshot").expect("write root artifact");
        fs::write(root.join("nested/state.bin"), b"nested snapshot")
            .expect("write nested artifact");
    }

    fn checksum(path: &std::path::Path) -> String {
        ArtifactChecksum::from_directory(path)
            .expect("checksum artifact")
            .hash
    }
}
