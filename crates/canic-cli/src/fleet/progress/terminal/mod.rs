//! Module: fleet::progress::terminal
//!
//! Responsibility: own an isolated progress screen and restore the ordinary terminal.
//! Does not own: progress, cancellation, deployment or animation state.
//! Boundary: signals retain default termination after terminal cleanup.

use std::{
    io::{self, Write},
    os::fd::BorrowedFd,
    sync::{
        OnceLock,
        atomic::{AtomicBool, Ordering},
    },
};

const ENTER: &[u8] = b"\x1b[?1049h";
const LEAVE: &[u8] = b"\x1b[?1049l";
static ACTIVE: AtomicBool = AtomicBool::new(false);
static INTERRUPTED: AtomicBool = AtomicBool::new(false);
static CLEANUP: OnceLock<io::Result<()>> = OnceLock::new();

pub(super) fn install_cleanup() -> io::Result<()> {
    INTERRUPTED.store(false, Ordering::SeqCst);
    CLEANUP
        .get_or_init(|| {
            for signal in [signal_hook::consts::SIGINT, signal_hook::consts::SIGTERM] {
                // Only atomic state and an async-signal-safe write run in the handler.
                unsafe {
                    signal_hook::low_level::register(signal, move || {
                        restore();
                        let _ = signal_hook::low_level::emulate_default_handler(signal);
                    })?;
                }
            }
            let previous = std::panic::take_hook();
            std::panic::set_hook(Box::new(move |info| {
                restore();
                previous(info);
            }));
            Ok(())
        })
        .as_ref()
        .map_err(|error| io::Error::other(error.to_string()))
        .copied()
}

fn restore() {
    if ACTIVE.swap(false, Ordering::SeqCst) {
        INTERRUPTED.store(true, Ordering::SeqCst);
        // stderr is process-owned; borrowing its descriptor performs no allocation
        // or locking, including when interruption occurs during a repaint.
        let stderr = unsafe { BorrowedFd::borrow_raw(2) };
        let _ = rustix::io::write(stderr, LEAVE);
    }
}

pub(super) fn supports_live(is_terminal: bool, term: Option<&str>, no_color: bool) -> bool {
    is_terminal && !no_color && term.is_some_and(|term| term != "dumb" && !term.is_empty())
}

pub(super) fn size() -> (usize, usize) {
    rustix::termios::tcgetwinsize(io::stderr()).map_or((80, 24), |size| {
        (
            usize::from(size.ws_col).max(1),
            usize::from(size.ws_row).max(1),
        )
    })
}

/// One alternate-screen lifetime, independent of terminal dimensions.
#[derive(Default)]
pub(super) struct Painter {
    active: bool,
}

impl Painter {
    pub(super) fn clear(
        &mut self,
        writer: &mut impl Write,
        _size: (usize, usize),
    ) -> io::Result<()> {
        if self.active {
            writer.write_all(LEAVE)?;
            self.active = false;
            ACTIVE.store(false, Ordering::SeqCst);
        }
        writer.flush()
    }

    pub(super) fn paint(
        &mut self,
        writer: &mut impl Write,
        lines: &[String],
        size: (usize, usize),
    ) -> io::Result<()> {
        if INTERRUPTED.load(Ordering::SeqCst) {
            return Err(io::ErrorKind::Interrupted.into());
        }
        if !self.active {
            self.active = true;
            ACTIVE.store(true, Ordering::SeqCst);
            writer.write_all(ENTER)?;
        }
        writer.write_all(b"\x1b[H\x1b[2J")?;
        let width = size.0.saturating_sub(1).max(1);
        for (index, line) in lines.iter().take(size.1.max(1)).enumerate() {
            if index != 0 {
                writer.write_all(b"\r\n")?;
            }
            let line = bounded_line(line, width);
            writer.write_all(line.as_bytes())?;
        }
        writer.flush()
    }
}

fn bounded_line(value: &str, width: usize) -> String {
    let mut text = value
        .chars()
        .map(|ch| {
            if ch.is_ascii() && !ch.is_control() {
                ch
            } else {
                '?'
            }
        })
        .take(width)
        .collect::<String>();
    if value.chars().count() > width && width >= 3 {
        text.truncate(width - 3);
        text.push_str("...");
    }
    text
}
