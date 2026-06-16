use super::super::ReleaseSetEntry;
use std::{
    io::{self, IsTerminal, Write as IoWrite},
    time::Duration,
};

///
/// StageProgress
///
pub(super) struct StageProgress {
    interactive: bool,
    completed_rows: usize,
}

impl StageProgress {
    // Create a terminal-aware release-set progress renderer.
    pub(super) fn new() -> Self {
        Self {
            interactive: io::stdout().is_terminal(),
            completed_rows: 0,
        }
    }

    // Print the staging header with an interactive chunk bar when available.
    pub(super) fn print_header(&self) {
        if self.interactive {
            println!("{}", chunk_progress_line("-", 0, 0));
        }
        println!("{:<16} {:>10}", "CANISTER", "ELAPSED");
    }

    // Start one release row at zero uploaded chunks for interactive terminals.
    pub(super) fn start_entry(
        &self,
        entry: &ReleaseSetEntry,
        chunk_count: usize,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if self.interactive {
            self.write_interactive_row(&entry.role, 0, chunk_count)?;
        }
        Ok(())
    }

    // Update one release row after a chunk has been durably published.
    pub(super) fn update_entry(
        &self,
        entry: &ReleaseSetEntry,
        uploaded_chunks: usize,
        chunk_count: usize,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if self.interactive {
            self.write_interactive_row(&entry.role, uploaded_chunks, chunk_count)?;
        }
        Ok(())
    }

    // Leave the completed chunk state visible before printing the canister timing row.
    pub(super) fn finish_entry(
        &self,
        entry: &ReleaseSetEntry,
        chunk_count: usize,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if self.interactive {
            self.write_interactive_row(&entry.role, chunk_count, chunk_count)?;
        }
        Ok(())
    }

    // Print one completed canister timing row below the live chunk bar.
    pub(super) fn print_completed_entry(&mut self, entry: &ReleaseSetEntry, elapsed: Duration) {
        println!("{:<16} {:>9.2}s", entry.role, elapsed.as_secs_f64());
        self.completed_rows += 1;
    }

    // Rewrite the top chunk-progress line without disturbing completed rows.
    fn write_interactive_row(
        &self,
        role: &str,
        uploaded_chunks: usize,
        chunk_count: usize,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let distance = self.completed_rows + 2;
        print!("\x1b[{distance}A\r\x1b[2K");
        print!(
            "{}",
            chunk_progress_line(role, uploaded_chunks, chunk_count)
        );
        print!("\x1b[{distance}B\r");
        io::stdout().flush()?;
        Ok(())
    }
}

// Render the single live chunk-progress row.
fn chunk_progress_line(role: &str, uploaded_chunks: usize, chunk_count: usize) -> String {
    format!(
        "{:<16} {:<18}",
        "CHUNKS",
        format!("{role} {}", progress_bar(uploaded_chunks, chunk_count, 10))
    )
}

// Render a fixed-width ASCII progress bar for chunk uploads.
fn progress_bar(current: usize, total: usize, width: usize) -> String {
    if total == 0 || width == 0 {
        return "[] 0/0".to_string();
    }

    let filled = current.saturating_mul(width).div_ceil(total);
    let filled = filled.min(width);
    format!(
        "[{}{}] {current}/{total}",
        "#".repeat(filled),
        " ".repeat(width - filled)
    )
}
