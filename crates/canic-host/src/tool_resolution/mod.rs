//! Consumer error projection for SDK-owned executable filesystem resolution.
//!
//! Callers own search order and admission; this adapter reads only the current
//! working directory and preserves underlying filesystem failures.

use std::{
    env, io,
    path::{Path, PathBuf},
};

use ic_host_tools::tool::{ResolutionError, resolve_executable};

pub fn resolve(requested: &Path, directories: &[PathBuf]) -> Result<Option<PathBuf>, io::Error> {
    let cwd = env::current_dir()?;
    match resolve_executable(requested, &cwd, directories) {
        Ok(path) => Ok(Some(path)),
        Err(
            ResolutionError::NotFound
            | ResolutionError::NotRegularFile
            | ResolutionError::NotExecutable,
        ) => Ok(None),
        Err(ResolutionError::Io {
            directory: None,
            source,
        }) if source.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(ResolutionError::Io { source, .. }) => Err(source),
        Err(error) => Err(io::Error::new(io::ErrorKind::InvalidInput, error)),
    }
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs,
        os::unix::fs::{PermissionsExt, symlink},
    };

    #[test]
    fn search_projection_preserves_precedence_and_follows_symlinks() {
        let root = crate::test_support::temp_dir("canic-sdk-resolution");
        let first = root.join("first");
        let second = root.join("second");
        fs::create_dir_all(&first).unwrap();
        fs::create_dir_all(&second).unwrap();
        let target = second.join("payload");
        fs::write(&target, b"executable").unwrap();
        fs::set_permissions(&target, fs::Permissions::from_mode(0o755)).unwrap();
        symlink(&target, first.join("tool")).unwrap();
        fs::write(second.join("tool"), b"nonexecutable").unwrap();
        assert_eq!(
            resolve(Path::new("tool"), &[first.clone(), second.clone()]).unwrap(),
            Some(target)
        );
        fs::remove_file(first.join("tool")).unwrap();
        assert_eq!(resolve(Path::new("tool"), &[first, second]).unwrap(), None);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn literal_request_never_selects_a_same_named_search_candidate() {
        let root = crate::test_support::temp_dir("canic-sdk-literal");
        fs::create_dir_all(&root).unwrap();
        let candidate = root.join("tool");
        fs::write(&candidate, b"executable").unwrap();
        fs::set_permissions(&candidate, fs::Permissions::from_mode(0o755)).unwrap();
        assert_eq!(
            resolve(&root.join("missing/tool"), std::slice::from_ref(&root)).unwrap(),
            None
        );
        assert_eq!(resolve(&candidate, &[]).unwrap(), Some(candidate));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn filesystem_failure_stops_selection_instead_of_skipping_to_a_later_tool() {
        let root = crate::test_support::temp_dir("canic-sdk-error");
        let first = root.join("first");
        let second = root.join("second");
        fs::create_dir_all(&first).unwrap();
        fs::create_dir_all(&second).unwrap();
        symlink("tool", first.join("tool")).unwrap();
        fs::write(second.join("tool"), b"executable").unwrap();
        fs::set_permissions(second.join("tool"), fs::Permissions::from_mode(0o755)).unwrap();
        let expected = fs::metadata(first.join("tool")).unwrap_err().kind();
        assert_eq!(
            resolve(Path::new("tool"), &[first, second])
                .unwrap_err()
                .kind(),
            expected
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn empty_and_relative_search_directories_use_the_observed_working_directory() {
        const CHILD_ENV: &str = "CANIC_TEST_RESOLVER_CWD_CHILD";
        if env::var_os(CHILD_ENV).is_some() {
            let cwd = env::current_dir().unwrap();
            for directory in [PathBuf::new(), PathBuf::from(".")] {
                assert_eq!(
                    resolve(Path::new("tool"), &[directory]).unwrap(),
                    Some(cwd.join("tool"))
                );
            }
            assert_eq!(
                resolve(Path::new("tool"), &[PathBuf::from("relative")]).unwrap(),
                Some(cwd.join("relative/tool"))
            );
            return;
        }
        let root = crate::test_support::temp_dir("canic-sdk-cwd");
        fs::create_dir_all(root.join("relative")).unwrap();
        for path in [root.join("tool"), root.join("relative/tool")] {
            fs::write(&path, b"executable").unwrap();
            fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
        }
        let output = std::process::Command::new(env::current_exe().unwrap())
            .args(["--exact", "tool_resolution::tests::empty_and_relative_search_directories_use_the_observed_working_directory"])
            .current_dir(&root)
            .env(CHILD_ENV, "1")
            .output().unwrap();
        assert!(output.status.success(), "{output:?}");
        fs::remove_dir_all(root).unwrap();
    }
}
