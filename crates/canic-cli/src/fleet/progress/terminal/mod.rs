//! Module: fleet::progress::terminal
//!
//! Responsibility: repaint a bounded panel on ordinary stderr terminals.
//! Does not own: terminal modes, signal handlers, progress or animation state.
//! Boundary: never hides the cursor or enters an alternate screen.

use std::io::{self, Write};

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

/// Previous frame's printable widths, needed after terminal reflow on resize.
#[derive(Default)]
pub(super) struct Painter {
    widths: Vec<usize>,
    partial: bool,
}

impl Painter {
    pub(super) fn clear(
        &mut self,
        writer: &mut impl Write,
        size: (usize, usize),
    ) -> io::Result<()> {
        let rows = self
            .widths
            .iter()
            .map(|width| (*width).max(1).div_ceil(size.0.max(1)))
            .sum::<usize>()
            .min(size.1.saturating_sub(1));
        if self.partial {
            write!(writer, "\r\x1b[2K")?;
            self.partial = false;
        }
        self.widths.clear();
        for _ in 0..rows {
            write!(writer, "\x1b[1A\r\x1b[2K")?;
        }
        writer.flush()
    }

    pub(super) fn paint(
        &mut self,
        writer: &mut impl Write,
        lines: &[String],
        size: (usize, usize),
    ) -> io::Result<()> {
        self.clear(writer, size)?;
        let width = size.0.saturating_sub(1).max(1);
        for line in lines.iter().take(size.1.saturating_sub(1)) {
            let line = bounded_line(line, width);
            self.partial = true;
            writer.write_all(format!("{line}\n").as_bytes())?;
            self.partial = false;
            self.widths.push(line.len());
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
