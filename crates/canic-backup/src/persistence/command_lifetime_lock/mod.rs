//! Module: persistence::command_lifetime_lock
//!
//! Responsibility: prove one external command and its descendants are quiescent.
//! Does not own: operation ordering, command execution, or effect reconciliation.

use std::{
    fs, io,
    path::{Path, PathBuf},
    thread,
    time::{Duration, Instant},
};

#[cfg(unix)]
use std::os::fd::AsRawFd;

use super::file_lock::{self, FileLockError};

const COMMAND_QUIESCENCE_GRACE: Duration = Duration::from_millis(250);

#[derive(Clone, Copy, Debug)]
pub struct CommandLifetimeHandle {
    raw_fd: i32,
}

impl CommandLifetimeHandle {
    #[must_use]
    pub const fn raw_fd(self) -> i32 {
        self.raw_fd
    }
}

#[derive(Debug)]
pub enum CommandLifetimeLockError {
    InFlight { lock_path: String },
    UnsafeEntry { lock_path: String, kind: String },
    Io(io::Error),
}

#[derive(Debug)]
pub struct CommandLifetimeLock {
    file: fs::File,
    path: PathBuf,
}

impl CommandLifetimeLock {
    pub(crate) fn acquire(
        journal_path: &Path,
        operation_sequence: usize,
    ) -> Result<Self, CommandLifetimeLockError> {
        let path = command_lifetime_lock_path(journal_path, operation_sequence);
        let file = file_lock::acquire(&path).map_err(|error| project_error(&path, error))?;
        Ok(Self { file, path })
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    #[cfg(unix)]
    pub(crate) fn handle(&self) -> CommandLifetimeHandle {
        CommandLifetimeHandle {
            raw_fd: self.file.as_raw_fd(),
        }
    }

    #[cfg(not(unix))]
    pub(crate) const fn handle(&self) -> CommandLifetimeHandle {
        CommandLifetimeHandle { raw_fd: -1 }
    }

    pub(crate) fn finish(self) -> Result<(), CommandLifetimeLockError> {
        let Self { file, path } = self;
        drop(file);

        let deadline = Instant::now() + COMMAND_QUIESCENCE_GRACE;
        loop {
            match file_lock::acquire(&path) {
                Ok(probe) => {
                    drop(probe);
                    return Ok(());
                }
                Err(FileLockError::Locked) if Instant::now() < deadline => {
                    thread::sleep(Duration::from_millis(5));
                }
                Err(error) => return Err(project_error(&path, error)),
            }
        }
    }
}

fn project_error(path: &Path, error: FileLockError) -> CommandLifetimeLockError {
    match error {
        FileLockError::Locked => CommandLifetimeLockError::InFlight {
            lock_path: path.to_string_lossy().to_string(),
        },
        FileLockError::UnsafeEntry { kind } => CommandLifetimeLockError::UnsafeEntry {
            lock_path: path.to_string_lossy().to_string(),
            kind,
        },
        FileLockError::Io(error) => CommandLifetimeLockError::Io(error),
    }
}

fn command_lifetime_lock_path(journal_path: &Path, operation_sequence: usize) -> PathBuf {
    let mut path = journal_path.as_os_str().to_os_string();
    path.push(format!(".command-{operation_sequence}.lock"));
    PathBuf::from(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::temp_path;

    #[cfg(unix)]
    use rustix::io::{FdFlags, fcntl_getfd, fcntl_setfd};
    #[cfg(unix)]
    use std::{
        os::fd::BorrowedFd,
        os::unix::process::CommandExt,
        process::Command,
        thread,
        time::{Duration, Instant},
    };

    const CHILD_JOURNAL_ENV: &str = "CANIC_TEST_COMMAND_LOCK_CHILD_PATH";
    const CHILD_READY_ENV: &str = "CANIC_TEST_COMMAND_LOCK_CHILD_READY";

    #[cfg(unix)]
    #[test]
    fn command_lock_stays_close_on_exec_in_the_owner() {
        let journal_path = temp_path("canic-command-lifetime-lock");
        let lock = CommandLifetimeLock::acquire(&journal_path, 3).expect("acquire command lock");

        assert!(
            fcntl_getfd(&lock.file)
                .expect("read command lock descriptor flags")
                .contains(FdFlags::CLOEXEC)
        );
        lock.finish().expect("finish quiescent command");
        fs::remove_file(command_lifetime_lock_path(&journal_path, 3))
            .expect("remove command lock path");
    }

    #[cfg(unix)]
    #[test]
    fn owner_death_keeps_lock_until_direct_child_and_descendant_exit() {
        let journal_path = temp_path("canic-command-lifetime-owner-death");
        let ready_path = temp_path("canic-command-lifetime-ready");
        let mut owner = Command::new(std::env::current_exe().expect("resolve test executable"))
            .args([
                "--exact",
                "persistence::command_lifetime_lock::tests::command_lock_child_owner",
                "--nocapture",
            ])
            .env(CHILD_JOURNAL_ENV, &journal_path)
            .env(CHILD_READY_ENV, &ready_path)
            .spawn()
            .expect("spawn command lock owner");

        for _ in 0..500 {
            if ready_path.is_file() {
                break;
            }
            assert!(
                owner.try_wait().expect("inspect owner").is_none(),
                "command lock owner exited before readiness"
            );
            thread::sleep(Duration::from_millis(10));
        }
        assert!(
            ready_path.is_file(),
            "command lock owner did not become ready"
        );
        std::assert_matches!(
            CommandLifetimeLock::acquire(&journal_path, 7),
            Err(CommandLifetimeLockError::InFlight { .. })
        );

        owner.kill().expect("kill owner without unwinding");
        owner.wait().expect("reap owner");
        std::assert_matches!(
            CommandLifetimeLock::acquire(&journal_path, 7),
            Err(CommandLifetimeLockError::InFlight { .. })
        );

        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            match CommandLifetimeLock::acquire(&journal_path, 7) {
                Ok(lock) => {
                    lock.finish().expect("finish after descendant exit");
                    break;
                }
                Err(CommandLifetimeLockError::InFlight { .. }) if Instant::now() < deadline => {
                    thread::sleep(Duration::from_millis(10));
                }
                Err(error) => panic!("command tree did not become quiescent: {error:?}"),
            }
        }

        fs::remove_file(command_lifetime_lock_path(&journal_path, 7))
            .expect("remove command lock path");
        fs::remove_file(ready_path).expect("remove ready marker");
    }

    #[cfg(unix)]
    #[test]
    #[expect(
        clippy::zombie_processes,
        reason = "the owner is intentionally killed before it can reap the command tree"
    )]
    fn command_lock_child_owner() {
        let Some(journal_path) = std::env::var_os(CHILD_JOURNAL_ENV) else {
            return;
        };
        let ready_path = std::env::var_os(CHILD_READY_ENV).expect("child ready path");
        let lock =
            CommandLifetimeLock::acquire(Path::new(&journal_path), 7).expect("acquire child lock");
        let mut command = Command::new("sh");
        command.args(["-c", "sleep 2 & wait"]).process_group(0);
        inherit_command_lock(&mut command, lock.handle());
        let _command_tree = command.spawn().expect("spawn direct child and descendant");
        fs::write(ready_path, b"ready\n").expect("signal child readiness");
        loop {
            thread::sleep(Duration::from_secs(1));
        }
    }

    #[cfg(unix)]
    fn inherit_command_lock(command: &mut Command, handle: CommandLifetimeHandle) {
        let raw_fd = handle.raw_fd();
        // SAFETY: the child setup performs only fcntl on a descriptor kept open
        // by `lock` until this test process is killed.
        unsafe {
            command.pre_exec(move || {
                // SAFETY: `lock` keeps the raw descriptor valid through spawn.
                let fd = BorrowedFd::borrow_raw(raw_fd);
                let mut flags = fcntl_getfd(fd).map_err(errno_to_io)?;
                flags.remove(FdFlags::CLOEXEC);
                fcntl_setfd(fd, flags).map_err(errno_to_io)
            });
        }
    }

    #[cfg(unix)]
    fn errno_to_io(error: rustix::io::Errno) -> io::Error {
        io::Error::from_raw_os_error(error.raw_os_error())
    }
}
