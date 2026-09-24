use super::*;
use std::{os::unix::process::ExitStatusExt, process::Command};

#[test]
fn signals_cancel_only_the_scoped_wait_and_terminate_normally_afterwards() {
    const CHILD: &str = "CANIC_LOCK_CANCEL_TEST";
    if let Ok(mode) = std::env::var(CHILD) {
        let wait = Cancellation::start().unwrap();
        if mode == "cancel" {
            signal_hook::low_level::raise(SIGINT).unwrap();
            assert_eq!(wait.check().unwrap_err().kind(), io::ErrorKind::Interrupted);
            drop(wait);
            let next = Cancellation::start().unwrap();
            assert!(next.check().is_ok());
            drop(next);
        } else {
            drop(wait);
            signal_hook::low_level::raise(SIGTERM).unwrap();
            panic!("SIGTERM must retain default termination outside the wait");
        }
        return;
    }
    for (mode, signal) in [("cancel", None), ("after", Some(SIGTERM))] {
        let status = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", std::thread::current().name().unwrap()])
            .env(CHILD, mode)
            .status()
            .unwrap();
        assert_eq!(status.signal(), signal);
        if signal.is_none() {
            assert!(status.success());
        }
    }
}
