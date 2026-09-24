//! Module: support::build_lock::cancel
//!
//! Responsibility: allow SIGINT/SIGTERM to end only the synchronous lock waiter cleanly.
//! Does not own: compilation cancellation or process termination policy elsewhere.
//! Boundary: outside this scoped wait, signals retain their default termination behavior.

#[cfg(test)]
mod tests;

use signal_hook::{
    consts::{SIGINT, SIGTERM},
    flag,
};
use std::{
    io,
    sync::{
        Arc, Mutex, MutexGuard, OnceLock,
        atomic::{AtomicBool, Ordering},
    },
};

static SERIAL: Mutex<()> = Mutex::new(());
static SIGNALS: OnceLock<io::Result<Signals>> = OnceLock::new();

/// Process-wide handlers; unregistering a signal-hook callback does not restore defaults.
struct Signals {
    use_default: Arc<AtomicBool>,
    cancelled: Arc<AtomicBool>,
}

/// Exclusive scoped waiter, with conventional signal behavior restored on every exit.
pub(super) struct Cancellation {
    signals: &'static Signals,
    _serial: MutexGuard<'static, ()>,
}

impl Cancellation {
    pub(super) fn start() -> io::Result<Self> {
        let serial = SERIAL
            .lock()
            .map_err(|_| io::Error::other("lock cancellation guard poisoned"))?;
        let signals = SIGNALS
            .get_or_init(|| {
                let use_default = Arc::new(AtomicBool::new(true));
                let cancelled = Arc::new(AtomicBool::new(false));
                for signal in [SIGINT, SIGTERM] {
                    // Register default first; inactive waiters must never swallow termination.
                    flag::register_conditional_default(signal, Arc::clone(&use_default))?;
                    flag::register(signal, Arc::clone(&cancelled))?;
                }
                Ok(Signals {
                    use_default,
                    cancelled,
                })
            })
            .as_ref()
            .map_err(|error| io::Error::other(error.to_string()))?;
        signals.cancelled.store(false, Ordering::SeqCst);
        signals.use_default.store(false, Ordering::SeqCst);
        Ok(Self {
            signals,
            _serial: serial,
        })
    }

    pub(super) fn check(&self) -> io::Result<()> {
        if self.signals.cancelled.load(Ordering::SeqCst) {
            Err(io::Error::new(
                io::ErrorKind::Interrupted,
                "build lock wait cancelled",
            ))
        } else {
            Ok(())
        }
    }
}

impl Drop for Cancellation {
    fn drop(&mut self) {
        self.signals.use_default.store(true, Ordering::SeqCst);
    }
}
