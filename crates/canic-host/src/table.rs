///
/// WhitespaceTable
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WhitespaceTable {
    headers: Vec<String>,
    rows: Vec<Vec<String>>,
}

impl WhitespaceTable {
    /// Build a whitespace-aligned table from header labels.
    #[must_use]
    pub fn new(headers: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            headers: headers.into_iter().map(Into::into).collect(),
            rows: Vec::new(),
        }
    }

    /// Append one row, padding missing cells as empty strings.
    pub fn push_row(&mut self, row: impl IntoIterator<Item = impl Into<String>>) {
        self.rows.push(row.into_iter().map(Into::into).collect());
    }

    /// Render the table using two spaces between columns.
    #[must_use]
    pub fn render(&self) -> String {
        let widths = self.column_widths();
        let mut lines = Vec::with_capacity(self.rows.len() + 1);
        lines.push(render_row(&self.headers, &widths));
        lines.extend(self.rows.iter().map(|row| render_row(row, &widths)));
        lines.join("\n")
    }

    // Compute character widths so box-drawing tree prefixes do not over-pad columns.
    fn column_widths(&self) -> Vec<usize> {
        (0..self.headers.len())
            .map(|index| {
                std::iter::once(self.headers.get(index).map_or("", String::as_str))
                    .chain(
                        self.rows
                            .iter()
                            .map(move |row| row.get(index).map_or("", String::as_str)),
                    )
                    .map(display_width)
                    .max()
                    .unwrap_or(0)
            })
            .collect()
    }
}

// Render one row with precomputed column widths.
fn render_row(row: &[String], widths: &[usize]) -> String {
    widths
        .iter()
        .enumerate()
        .map(|(index, width)| {
            let value = row.get(index).map_or("", String::as_str);
            format!("{value:<width$}")
        })
        .collect::<Vec<_>>()
        .join("  ")
        .trim_end()
        .to_string()
}

// Count display width by character for simple terminal-aligned tables.
fn display_width(value: &str) -> usize {
    value.chars().count()
}
