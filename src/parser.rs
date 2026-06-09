#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParsedTable {
    pub markdown: String,
    pub row_count: usize,
    pub column_count: usize,
    pub warnings: Vec<String>,
}

pub fn parse_ascii_table(input: &str, first_row_is_header: bool) -> ParsedTable {
    let groups = collect_row_groups(input);
    let mut warnings = Vec::new();

    if groups.is_empty() {
        return ParsedTable {
            markdown: String::new(),
            row_count: 0,
            column_count: 0,
            warnings: vec!["No ASCII table rows were found.".to_string()],
        };
    }

    let mut rows: Vec<Vec<String>> = groups.into_iter().map(collapse_group).collect();
    rows.retain(|row| row.iter().any(|cell| !cell.trim().is_empty()));

    if rows.is_empty() {
        return ParsedTable {
            markdown: String::new(),
            row_count: 0,
            column_count: 0,
            warnings: vec!["The table only contained borders or empty cells.".to_string()],
        };
    }

    let column_count = rows.iter().map(Vec::len).max().unwrap_or(0);
    for row in &mut rows {
        row.resize(column_count, String::new());
    }

    let (header, body) = if first_row_is_header {
        (rows[0].clone(), rows[1..].to_vec())
    } else {
        let header = (1..=column_count)
            .map(|index| format!("Column {index}"))
            .collect();
        (header, rows)
    };

    if body.is_empty() {
        warnings.push("Only a header row was found.".to_string());
    }

    ParsedTable {
        markdown: render_markdown_table(&header, &body),
        row_count: body.len(),
        column_count,
        warnings,
    }
}

fn collect_row_groups(input: &str) -> Vec<Vec<String>> {
    let mut groups = Vec::new();
    let mut current = Vec::new();
    let mut saw_separator = false;

    for line in input.lines() {
        let trimmed = line.trim();

        if is_horizontal_rule(trimmed) {
            saw_separator = true;
            if !current.is_empty() {
                groups.push(std::mem::take(&mut current));
            }
            continue;
        }

        if is_cell_line(trimmed) {
            push_cell_line(&mut groups, &mut current, trimmed);
        }
    }

    if !current.is_empty() {
        groups.push(current);
    }

    if saw_separator {
        groups
    } else {
        groups
            .into_iter()
            .flat_map(|group| group.into_iter().map(|line| vec![line]))
            .collect()
    }
}

fn push_cell_line(groups: &mut Vec<Vec<String>>, current: &mut Vec<String>, line: &str) {
    if !current.is_empty() && starts_new_row(line) {
        groups.push(std::mem::take(current));
    }

    current.push(line.to_string());
}

fn starts_new_row(line: &str) -> bool {
    let Some(delimiter) = detect_cell_delimiter(line) else {
        return false;
    };

    line.trim_matches(delimiter)
        .split(delimiter)
        .next()
        .is_some_and(|cell| !cell.trim().is_empty())
}

fn is_horizontal_rule(line: &str) -> bool {
    let mut has_ascii_corner = false;
    let mut has_box_drawing = false;
    let mut has_rule = false;

    for ch in line.chars() {
        if ch.is_whitespace() {
            continue;
        }

        if is_ascii_rule_char(ch) {
            has_ascii_corner |= ch == '+';
            has_rule |= matches!(ch, '-' | '=');
            continue;
        }

        if is_box_drawing_char(ch) {
            has_box_drawing = true;
            has_rule |= is_horizontal_box_rule_char(ch);
            continue;
        }

        return false;
    }

    has_rule && (has_ascii_corner || has_box_drawing)
}

fn is_cell_line(line: &str) -> bool {
    detect_cell_delimiter(line).is_some()
}

fn detect_cell_delimiter(line: &str) -> Option<char> {
    ['|', '│', '┃', '║'].into_iter().find(|delimiter| {
        line.starts_with(*delimiter)
            && line.ends_with(*delimiter)
            && line.matches(*delimiter).count() >= 2
    })
}

fn is_ascii_rule_char(ch: char) -> bool {
    matches!(ch, '+' | '-' | '=' | ':')
}

fn is_box_drawing_char(ch: char) -> bool {
    ('\u{2500}'..='\u{257f}').contains(&ch)
}

fn is_horizontal_box_rule_char(ch: char) -> bool {
    matches!(ch, '─' | '━' | '═' | '╌' | '╍' | '┄' | '┅' | '┈' | '┉')
}

fn collapse_group(lines: Vec<String>) -> Vec<String> {
    let parsed_lines: Vec<Vec<String>> = lines
        .iter()
        .map(|line| {
            let delimiter = detect_cell_delimiter(line).unwrap_or('|');

            line.trim_matches(delimiter)
                .split(delimiter)
                .map(|cell| cell.trim().to_string())
                .collect()
        })
        .collect();

    let column_count = parsed_lines.iter().map(Vec::len).max().unwrap_or(0);

    (0..column_count)
        .map(|column| {
            parsed_lines
                .iter()
                .filter_map(|line| line.get(column))
                .map(|cell| cell.trim())
                .filter(|cell| !cell.is_empty())
                .map(escape_markdown_cell)
                .collect::<Vec<_>>()
                .join("<br>")
        })
        .collect()
}

fn render_markdown_table(header: &[String], body: &[Vec<String>]) -> String {
    let mut lines = Vec::with_capacity(body.len() + 2);
    lines.push(render_markdown_row(header));
    lines.push(render_markdown_row(&vec!["---".to_string(); header.len()]));

    for row in body {
        lines.push(render_markdown_row(row));
    }

    lines.join("\n")
}

fn render_markdown_row(cells: &[String]) -> String {
    format!("| {} |", cells.join(" | "))
}

fn escape_markdown_cell(cell: &str) -> String {
    cell.replace('|', r"\|")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_basic_ascii_table() {
        let input = r#"
+------+-------+
| Name | Value |
+------+-------+
| A    | 1     |
| B    | 2     |
+------+-------+
"#;

        let parsed = parse_ascii_table(input, true);

        assert_eq!(
            parsed.markdown,
            "| Name | Value |\n| --- | --- |\n| A | 1 |\n| B | 2 |"
        );
        assert_eq!(parsed.row_count, 2);
        assert_eq!(parsed.column_count, 2);
    }

    #[test]
    fn joins_multiline_cells_with_breaks() {
        let input = r#"
+------+---------+
| Name | Notes   |
+------+---------+
| A    | one     |
|      | two     |
+------+---------+
"#;

        let parsed = parse_ascii_table(input, true);

        assert_eq!(
            parsed.markdown,
            "| Name | Notes |\n| --- | --- |\n| A | one<br>two |"
        );
    }

    #[test]
    fn can_generate_header_when_missing() {
        let input = r#"
+---+---+
| A | 1 |
+---+---+
"#;

        let parsed = parse_ascii_table(input, false);

        assert_eq!(
            parsed.markdown,
            "| Column 1 | Column 2 |\n| --- | --- |\n| A | 1 |"
        );
    }

    #[test]
    fn escapes_pipe_characters_inside_cells() {
        let input = r#"
+------+--------+
| Name | Regex  |
+------+--------+
| A    | x | y  |
+------+--------+
"#;

        let parsed = parse_ascii_table(input, true);

        assert_eq!(
            parsed.markdown,
            "| Name | Regex |  |\n| --- | --- | --- |\n| A | x | y |"
        );
    }

    #[test]
    fn converts_unicode_box_drawing_table() {
        let input = r#"
  ┌──────────────┬─────────────────────┐
  │ 행성         │ 분류                │
  ├──────────────┼─────────────────────┤
  │ ① 수성       │ 암석형              │
  ├──────────────┼─────────────────────┤
  │ ② 금성       │ 암석형              │
  ├──────────────┼─────────────────────┤
  │ ③ 지구       │ 암석형 · 생명체     │
  ├──────────────┼─────────────────────┤
  │ ④ 화성       │ 암석형              │
  ├──────────────┼─────────────────────┤
  │ ⑤ 목성       │ 가스형              │
  ├──────────────┼─────────────────────┤
  │ ⑥ 토성       │ 가스형 (고리)       │
  ├──────────────┼─────────────────────┤
  │ ⑦ 천왕성     │ 얼음형              │
  └──────────────┴─────────────────────┘
"#;

        let parsed = parse_ascii_table(input, true);

        assert_eq!(
            parsed.markdown,
            "| 행성 | 분류 |\n| --- | --- |\n| ① 수성 | 암석형 |\n| ② 금성 | 암석형 |\n| ③ 지구 | 암석형 · 생명체 |\n| ④ 화성 | 암석형 |\n| ⑤ 목성 | 가스형 |\n| ⑥ 토성 | 가스형 (고리) |\n| ⑦ 천왕성 | 얼음형 |"
        );
        assert_eq!(parsed.row_count, 7);
        assert_eq!(parsed.column_count, 2);
    }
}
