const COLUMN_GAP: &str = "   ";

///
/// ColumnAlign
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ColumnAlign {
    Left,
    Right,
}

/// Render a whitespace-aligned table with an underlined header row.
#[must_use]
pub fn render_table<const N: usize>(
    headers: &[&str; N],
    rows: &[[String; N]],
    alignments: &[ColumnAlign; N],
) -> String {
    let widths = table_widths(headers, rows);
    let mut lines = Vec::with_capacity(rows.len() + 2);
    lines.push(render_table_row(headers, &widths, alignments));
    lines.push(render_separator(&widths));
    lines.extend(
        rows.iter()
            .map(|row| render_table_row(row, &widths, alignments)),
    );
    lines.join("\n")
}

/// Compute per-column display widths from headers and rows.
#[must_use]
pub fn table_widths<const N: usize>(headers: &[&str; N], rows: &[[String; N]]) -> [usize; N] {
    let mut widths = headers.map(str::chars).map(Iterator::count);

    for row in rows {
        for (index, cell) in row.iter().enumerate() {
            widths[index] = widths[index].max(cell.chars().count());
        }
    }

    widths
}

/// Render one whitespace-aligned table row.
#[must_use]
pub fn render_table_row<const N: usize>(
    row: &[impl AsRef<str>],
    widths: &[usize; N],
    alignments: &[ColumnAlign; N],
) -> String {
    widths
        .iter()
        .zip(alignments)
        .enumerate()
        .map(|(index, (width, alignment))| {
            let value = row.get(index).map_or("", AsRef::as_ref);
            match alignment {
                ColumnAlign::Left => format!("{value:<width$}"),
                ColumnAlign::Right => format!("{value:>width$}"),
            }
        })
        .collect::<Vec<_>>()
        .join(COLUMN_GAP)
        .trim_end()
        .to_string()
}

/// Render the underline row for a whitespace-aligned table.
#[must_use]
pub fn render_separator<const N: usize>(widths: &[usize; N]) -> String {
    widths
        .iter()
        .map(|width| "-".repeat(*width))
        .collect::<Vec<_>>()
        .join(COLUMN_GAP)
}
