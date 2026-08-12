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

/// Render an ASCII-bordered table with one space of cell padding.
#[must_use]
pub fn render_bordered_table<const N: usize>(
    headers: &[&str; N],
    rows: &[[String; N]],
    alignments: &[ColumnAlign; N],
) -> String {
    let widths = table_widths(headers, rows);
    let mut lines = Vec::with_capacity(rows.len() + 4);
    lines.push(render_table_border(&widths, '-'));
    lines.push(render_bordered_table_row(headers, &widths, alignments));
    lines.push(render_table_border(&widths, '='));
    lines.extend(
        rows.iter()
            .map(|row| render_bordered_table_row(row, &widths, alignments)),
    );
    lines.push(render_table_border(&widths, '-'));
    lines.join("\n")
}

/// Compute per-column display widths from headers and rows.
#[must_use]
pub fn table_widths<const N: usize>(headers: &[&str; N], rows: &[[String; N]]) -> [usize; N] {
    let mut widths = headers.map(display_width);

    for row in rows {
        for (index, cell) in row.iter().enumerate() {
            widths[index] = widths[index].max(display_width(cell));
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
            pad_cell(value, *width, *alignment)
        })
        .collect::<Vec<_>>()
        .join(COLUMN_GAP)
        .trim_end()
        .to_string()
}

/// Render one row with ASCII borders and one space of horizontal cell padding.
#[must_use]
pub fn render_bordered_table_row<const N: usize>(
    row: &[impl AsRef<str>],
    widths: &[usize; N],
    alignments: &[ColumnAlign; N],
) -> String {
    let cells = widths
        .iter()
        .zip(alignments)
        .enumerate()
        .map(|(index, (width, alignment))| {
            let value = row.get(index).map_or("", AsRef::as_ref);
            format!(" {} ", pad_cell(value, *width, *alignment))
        })
        .collect::<Vec<_>>()
        .join("|");
    format!("|{cells}|")
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

/// Render a horizontal border for an ASCII-bordered table.
#[must_use]
pub fn render_table_border<const N: usize>(widths: &[usize; N], fill: char) -> String {
    let segments = widths
        .iter()
        .map(|width| fill.to_string().repeat(width + 2))
        .collect::<Vec<_>>()
        .join("+");
    format!("+{segments}+")
}

fn pad_cell(value: &str, width: usize, alignment: ColumnAlign) -> String {
    let padding = width.saturating_sub(display_width(value));
    match alignment {
        ColumnAlign::Left => format!("{value}{}", " ".repeat(padding)),
        ColumnAlign::Right => format!("{}{value}", " ".repeat(padding)),
    }
}

fn display_width(value: &str) -> usize {
    let mut width = 0;
    let mut chars = value.chars().peekable();
    while let Some(character) = chars.next() {
        if character == '\u{1b}' && chars.next_if_eq(&'[').is_some() {
            for control in chars.by_ref() {
                if ('@'..='~').contains(&control) {
                    break;
                }
            }
            continue;
        }
        width += 1;
    }
    width
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bordered_table_keeps_ansi_cells_aligned() {
        let rows = [["\u{1b}[32mdone\u{1b}[0m".to_string(), "12".to_string()]];
        let table = render_bordered_table(
            &["STATUS", "COUNT"],
            &rows,
            &[ColumnAlign::Left, ColumnAlign::Right],
        );
        let lines = table.lines().collect::<Vec<_>>();

        assert_eq!(lines[0], "+--------+-------+");
        assert_eq!(display_width(lines[1]), display_width(lines[3]));
        assert_eq!(display_width(lines[3]), display_width(lines[4]));
    }

    #[test]
    fn bordered_table_adds_horizontal_cell_padding() {
        let table = render_bordered_table(
            &["A", "B"],
            &[["left".to_string(), "7".to_string()]],
            &[ColumnAlign::Left, ColumnAlign::Right],
        );

        assert!(table.contains("| left | 7 |"));
    }
}
