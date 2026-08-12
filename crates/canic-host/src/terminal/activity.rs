//! Module: terminal::activity
//!
//! Responsibility: keep one interactive activity line alive during blocking work.
//! Does not own: command execution, result reporting, or durable phase timing.
//! Boundary: non-interactive output is append-only; ANSI cursor control is TTY-only.

use super::TerminalStyle;
use std::{
    io::{self, Write},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

const FRAMES: [&str; 4] = ["|", "/", "-", "\\"];
const FRAME_INTERVAL: Duration = Duration::from_millis(120);

pub struct TerminalActivity {
    stop: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
    interactive: bool,
}

impl TerminalActivity {
    #[must_use]
    pub fn start(message: impl Into<String>) -> Self {
        let message = message.into();
        let style = TerminalStyle::detected();
        if !style.interactive() {
            println!("  .. {message}");
            return Self {
                stop: Arc::new(AtomicBool::new(true)),
                handle: None,
                interactive: false,
            };
        }

        let stop = Arc::new(AtomicBool::new(false));
        let worker_stop = Arc::clone(&stop);
        let handle = thread::spawn(move || {
            let started_at = Instant::now();
            let mut frame = 0;
            while !worker_stop.load(Ordering::Acquire) {
                let elapsed = started_at.elapsed().as_secs();
                print!(
                    "\r\u{1b}[2K  {} {}  {}",
                    style.warning(FRAMES[frame % FRAMES.len()]),
                    message,
                    style.muted(&format!("{elapsed}s"))
                );
                let _ = io::stdout().flush();
                frame += 1;
                thread::park_timeout(FRAME_INTERVAL);
            }
        });

        Self {
            stop,
            handle: Some(handle),
            interactive: true,
        }
    }

    pub fn finish(mut self) {
        self.stop_and_clear();
    }

    fn stop_and_clear(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(handle) = self.handle.take() {
            handle.thread().unpark();
            let _ = handle.join();
        }
        if self.interactive {
            print!("\r\u{1b}[2K");
            let _ = io::stdout().flush();
        }
    }
}

impl Drop for TerminalActivity {
    fn drop(&mut self) {
        self.stop_and_clear();
    }
}
