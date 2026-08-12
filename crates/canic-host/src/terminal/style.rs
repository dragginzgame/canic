//! Module: terminal::style
//!
//! Responsibility: select terminal-safe styling for operator output.
//! Does not own: workflow state, progress timing, or table layout.
//! Boundary: color ANSI is emitted only for an interactive non-dumb terminal without `NO_COLOR`.

use std::io::{self, IsTerminal};

const RESET: &str = "\u{1b}[0m";
const BOLD_CYAN: &str = "\u{1b}[1;36m";
const DIM: &str = "\u{1b}[2m";
const GREEN: &str = "\u{1b}[32m";
const YELLOW: &str = "\u{1b}[33m";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TerminalStyle {
    interactive: bool,
    color: bool,
}

impl TerminalStyle {
    #[must_use]
    pub fn detected() -> Self {
        let interactive =
            io::stdout().is_terminal() && std::env::var("TERM").map_or(true, |term| term != "dumb");
        Self {
            interactive,
            color: interactive && std::env::var_os("NO_COLOR").is_none(),
        }
    }

    #[must_use]
    pub const fn interactive(self) -> bool {
        self.interactive
    }

    #[must_use]
    pub fn heading(self, text: &str) -> String {
        self.paint(BOLD_CYAN, text)
    }

    #[must_use]
    pub fn success(self, text: &str) -> String {
        self.paint(GREEN, text)
    }

    #[must_use]
    pub fn warning(self, text: &str) -> String {
        self.paint(YELLOW, text)
    }

    #[must_use]
    pub fn muted(self, text: &str) -> String {
        self.paint(DIM, text)
    }

    pub fn print_section(self, title: &str, detail: &str) {
        println!(
            "{} {}  {}",
            self.heading("==>"),
            self.heading(title),
            self.muted(detail)
        );
    }

    fn paint(self, code: &str, text: &str) -> String {
        if self.color {
            format!("{code}{text}{RESET}")
        } else {
            text.to_string()
        }
    }
}

#[cfg(test)]
impl TerminalStyle {
    const fn ansi_for_tests() -> Self {
        Self {
            interactive: true,
            color: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ansi_style_colors_status_without_changing_text() {
        let style = TerminalStyle::ansi_for_tests();

        assert_eq!(style.success("done"), "\u{1b}[32mdone\u{1b}[0m");
    }
}
