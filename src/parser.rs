#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParsedTable {
    pub markdown: String,
    pub row_count: usize,
    pub column_count: usize,
    pub warnings: Vec<String>,
}

pub fn parse_ascii_table(input: &str, first_row_is_header: bool) -> ParsedTable {
    let mut lines_out: Vec<String> = Vec::new();
    let mut warnings = Vec::new();
    let mut total_rows = 0;
    let mut max_columns = 0;
    let mut table_count = 0;

    for segment in split_segments(input) {
        let table = match segment {
            Segment::Passthrough(line) => {
                lines_out.push(line.to_string());
                continue;
            }
            Segment::BoxTable(block) => convert_table_block(&block, first_row_is_header),
            Segment::PipeTable { rows, separator } => {
                format_table(rows, first_row_is_header, Some(separator))
            }
        };

        if let Some(table) = table {
            lines_out.extend(table.markdown.split('\n').map(str::to_string));
            total_rows += table.row_count;
            max_columns = max_columns.max(table.column_count);
            warnings.extend(table.warnings);
            table_count += 1;
        }
    }

    if table_count == 0 {
        return ParsedTable {
            markdown: String::new(),
            row_count: 0,
            column_count: 0,
            warnings: vec!["No box-table rows were found.".to_string()],
        };
    }

    ParsedTable {
        markdown: tidy_blank_lines(lines_out),
        row_count: total_rows,
        column_count: max_columns,
        warnings,
    }
}

enum Segment<'a> {
    Passthrough(&'a str),
    BoxTable(Vec<&'a str>),
    PipeTable {
        rows: Vec<Vec<String>>,
        separator: Vec<String>,
    },
}

/// Splits box tables, Markdown pipe tables and surrounding text in document order.
fn split_segments(mut input: &str) -> Vec<Segment<'_>> {
    let mut segments = Vec::new();
    let mut box_lines = Vec::<&str>::new();
    let mut at_line_start = true;

    while !input.is_empty() {
        let inline = input.trim_start_matches(is_inline_space);
        if !box_lines
            .first()
            .is_some_and(|line| is_horizontal_rule(line))
            && let Some((start, table, rest)) = find_pipe_table(inline, at_line_start)
        {
            if !box_lines.is_empty() {
                segments.push(Segment::BoxTable(std::mem::take(&mut box_lines)));
            }
            let prefix = inline[..start].trim_end();
            if !prefix.is_empty() {
                segments.push(Segment::Passthrough(prefix));
            }
            segments.push(Segment::Passthrough(""));
            segments.push(table);
            segments.push(Segment::Passthrough(""));
            input = rest;
            at_line_start = false;
            continue;
        }

        let (line, rest) = input.split_once('\n').unwrap_or((input, ""));
        let trimmed = line.trim();
        if is_horizontal_rule(trimmed) || is_cell_line(trimmed) {
            box_lines.push(trimmed);
        } else {
            if !box_lines.is_empty() {
                segments.push(Segment::BoxTable(std::mem::take(&mut box_lines)));
            }
            segments.push(Segment::Passthrough(line.trim_end()));
        }
        input = rest;
        at_line_start = true;
    }

    if !box_lines.is_empty() {
        segments.push(Segment::BoxTable(box_lines));
    }
    segments
}

fn is_inline_space(ch: char) -> bool {
    ch != '\n' && ch.is_whitespace()
}

fn pipe_offsets(line: &str) -> impl Iterator<Item = usize> + '_ {
    let mut escaped = false;
    line.bytes()
        .enumerate()
        .take_while(|&(_, byte)| byte != b'\n')
        .filter_map(move |(offset, byte)| {
            let is_pipe = byte == b'|' && !escaped;
            escaped = byte == b'\\' && !escaped;
            is_pipe.then_some(offset)
        })
}

/// Iterates over cells after the opening pipe, yielding the end of each cell
/// and its trimmed contents. Escaped pipes are cell text, not boundaries.
fn pipe_cells(row: &str) -> impl Iterator<Item = (usize, &str)> {
    let mut start = 1;
    pipe_offsets(row).skip(1).map(move |end| {
        let cell = row[start..end].trim();
        start = end + 1;
        (start, cell)
    })
}

fn is_separator_cell(cell: &str) -> bool {
    let marker = cell.strip_prefix(':').unwrap_or(cell);
    let marker = marker.strip_suffix(':').unwrap_or(marker);
    !marker.is_empty() && marker.bytes().all(|byte| byte == b'-')
}

fn ends_row(input: &str) -> bool {
    input.is_empty() || input.starts_with(['|', '\n'])
}

fn pipe_separator_end(separator: &str, width: usize) -> Option<usize> {
    let mut end = 0;
    let mut cells = pipe_cells(separator);
    for _ in 0..width {
        let (offset, cell) = cells.next()?;
        if !is_separator_cell(cell) {
            return None;
        }
        end = offset;
    }

    ends_row(separator[end..].trim_start_matches(is_inline_space)).then_some(end)
}

struct PipeHeader<'a> {
    width: usize,
    row: &'a str,
    separator: &'a str,
    rest: &'a str,
}

/// Search from the separator back to its header. This lets headings themselves
/// contain pipes without trying each of those pipes as a possible table start.
fn flat_pipe_header(line: &str) -> Option<(usize, PipeHeader<'_>)> {
    let mut offsets = Vec::<usize>::new();
    let mut width = 0;
    let mut pipes = pipe_offsets(line).peekable();
    while let Some(end) = pipes.next() {
        if let Some(&prev) = offsets.last() {
            if is_separator_cell(line[prev + 1..end].trim()) {
                width += 1;
                if let Some(start) = offsets.len().checked_sub(2 * width + 1) {
                    let header_end = offsets[start + width];
                    let separator_start = offsets[start + width + 1];
                    if line[header_end + 1..separator_start].trim().is_empty()
                        && !pipes
                            .peek()
                            .is_some_and(|&next| is_separator_cell(line[end + 1..next].trim()))
                    {
                        let start = offsets[start];
                        return Some((
                            start,
                            PipeHeader {
                                width,
                                row: &line[start..header_end + 1],
                                separator: &line[separator_start..end + 1],
                                rest: &line[end + 1..],
                            },
                        ));
                    }
                }
            } else {
                width = 0;
            }
        }
        offsets.push(end);
    }
    None
}

fn line_pipe_header(input: &str) -> Option<PipeHeader<'_>> {
    let (line, separator) = input.split_once('\n')?;
    let separator = separator.trim_start_matches(is_inline_space);
    if !separator.starts_with('|') {
        return None;
    }
    let header = line.trim_end();
    let (index, (end, _)) = pipe_cells(header).enumerate().last()?;
    if end != header.len() {
        return None;
    }
    let width = index + 1;
    let (separator, rest) = separator.split_at(pipe_separator_end(separator, width)?);
    Some(PipeHeader {
        width,
        row: header,
        separator,
        rest,
    })
}

fn find_pipe_table(input: &str, at_line_start: bool) -> Option<(usize, Segment<'_>, &str)> {
    let (start, header) = if at_line_start
        && input.starts_with('|')
        && let Some(header) = line_pipe_header(input)
    {
        (0, header)
    } else {
        flat_pipe_header(input)?
    };
    let PipeHeader {
        width,
        row,
        separator,
        mut rest,
    } = header;

    let read_row = |row| {
        pipe_cells(row)
            .take(width)
            .map(|(_, cell)| cell.to_string())
            .collect()
    };
    let mut rows = vec![read_row(row)];
    // A later separator can establish a different width. Keep its position
    // as the length of the suffix, so advancing through rows needs no rescan.
    let next_flat_header = |text: &str| flat_pipe_header(text).map(|(start, _)| text.len() - start);
    let mut next_flat = next_flat_header(rest);
    loop {
        let gap = rest.trim_start_matches(is_inline_space);
        let on_new_line = gap.starts_with('\n');
        let row = gap
            .strip_prefix('\n')
            .map_or(gap, |rest| rest.trim_start_matches(is_inline_space));
        if !row.starts_with('|') {
            break;
        }
        if on_new_line {
            // A full physical row can contain both empty and dash-only cells
            // that would otherwise look like a folded header and separator.
            if let Some((end, _)) = pipe_cells(row).nth(width - 1)
                && {
                    let tail = row[end..].trim_start_matches(is_inline_space);
                    tail.is_empty() || tail.starts_with('\n')
                }
            {
                rows.push(read_row(&row[..end]));
                rest = &row[end..];
                continue;
            }
            if line_pipe_header(row).is_some() {
                break;
            }
            next_flat = next_flat_header(row);
        }
        let limit = next_flat.map_or(row.len(), |len| row.len() - len);
        if limit == 0 {
            break;
        }
        let Some(end) = pipe_body_end(row, width, on_new_line && next_flat.is_none()) else {
            break;
        };
        rows.push(read_row(&row[..end]));
        rest = &row[end..];
        if end > limit {
            // A candidate inside the row cannot be an opening delimiter.
            next_flat = next_flat_header(rest);
        }
    }
    Some((
        start,
        Segment::PipeTable {
            rows,
            separator: read_row(separator),
        },
        rest,
    ))
}

fn pipe_body_end(row: &str, width: usize, on_new_line: bool) -> Option<usize> {
    if !on_new_line {
        return pipe_cells(row).nth(width - 1).map(|(end, _)| end);
    }

    let mut fixed_end = None;
    for (index, (end, _)) in pipe_cells(row).enumerate() {
        let cells = index + 1;
        let tail = row[end..].trim_start_matches(is_inline_space);
        if cells == width {
            fixed_end = Some(end);
        }
        if tail.starts_with('|') {
            // After a complete row, a pair of pipes can open a flattened row.
            if cells >= width {
                break;
            }
        } else if tail.is_empty() || tail.starts_with('\n') {
            return Some(end);
        }
    }
    fixed_end
}

fn convert_table_block(block: &[&str], first_row_is_header: bool) -> Option<ParsedTable> {
    let groups = collect_row_groups(block);
    if groups.is_empty() {
        return None;
    }

    let mut rows: Vec<Vec<String>> = groups.into_iter().map(collapse_group).collect();
    rows.retain(|row| row.iter().any(|cell| !cell.trim().is_empty()));
    format_table(rows, first_row_is_header, None)
}

fn format_table(
    mut rows: Vec<Vec<String>>,
    first_row_is_header: bool,
    separator: Option<Vec<String>>,
) -> Option<ParsedTable> {
    if rows.is_empty() {
        return None;
    }

    let column_count = rows.iter().map(Vec::len).max().unwrap_or(0);
    for row in &mut rows {
        row.resize(column_count, String::new());
    }

    let mut warnings = Vec::new();
    let (header, body) = if first_row_is_header {
        (std::mem::take(&mut rows[0]), &rows[1..])
    } else {
        let header = (1..=column_count)
            .map(|index| format!("Column {index}"))
            .collect();
        (header, rows.as_slice())
    };

    if body.is_empty() {
        warnings.push("Only a header row was found.".to_string());
    }

    Some(ParsedTable {
        markdown: render_markdown_table(
            &header,
            body,
            separator.as_deref().filter(|_| first_row_is_header),
        ),
        row_count: body.len(),
        column_count,
        warnings,
    })
}

/// Drops leading/trailing blank lines and collapses runs of blanks to one, so a
/// single table renders with no surrounding whitespace and stacked blocks stay
/// separated by exactly one blank line.
fn tidy_blank_lines(lines: Vec<String>) -> String {
    let mut cleaned: Vec<String> = Vec::new();

    for line in lines {
        if line.trim().is_empty() {
            if cleaned.is_empty() || cleaned.last().is_some_and(|last| last.trim().is_empty()) {
                continue;
            }
            cleaned.push(String::new());
        } else {
            cleaned.push(line);
        }
    }

    while cleaned.last().is_some_and(|last| last.trim().is_empty()) {
        cleaned.pop();
    }

    cleaned.join("\n")
}

fn collect_row_groups<'a>(input: &[&'a str]) -> Vec<Vec<&'a str>> {
    let mut groups = Vec::new();
    let mut current = Vec::new();
    let mut saw_separator = false;

    for &line in input {
        if is_horizontal_rule(line) {
            saw_separator = true;
            if !current.is_empty() {
                groups.push(std::mem::take(&mut current));
            }
            continue;
        }

        if is_cell_line(line) {
            push_cell_line(&mut groups, &mut current, line);
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

fn push_cell_line<'a>(groups: &mut Vec<Vec<&'a str>>, current: &mut Vec<&'a str>, line: &'a str) {
    if !current.is_empty() && starts_new_row(line) {
        groups.push(std::mem::take(current));
    }

    current.push(line);
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

fn collapse_group(lines: Vec<&str>) -> Vec<String> {
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

fn render_markdown_table(
    header: &[String],
    body: &[Vec<String>],
    separator: Option<&[String]>,
) -> String {
    let mut lines = Vec::with_capacity(body.len() + 2);
    lines.push(render_markdown_row(header));
    lines.push(match separator {
        Some(separator) => render_markdown_row(separator),
        None => {
            let mut rule = String::with_capacity(1 + 6 * header.len());
            rule.push('|');
            for _ in header {
                rule.push_str(" --- |");
            }
            rule
        }
    });

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

/// Renders generated Markdown (headings + GitHub-style pipe tables) to an HTML
/// string for the live preview. Table support is enabled explicitly.
pub fn markdown_to_html(markdown: &str) -> String {
    let mut options = pulldown_cmark::Options::empty();
    options.insert(pulldown_cmark::Options::ENABLE_TABLES);

    let parser = pulldown_cmark::Parser::new_ext(markdown, options);
    let mut html = String::new();
    pulldown_cmark::html::push_html(&mut html, parser);
    html
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

    #[test]
    fn converts_document_with_headings_and_two_tables() {
        let input = "\
# Unicode box table

┌────────────┬───────────────┐
│ Animal     │ Role          │
├────────────┼───────────────┤
│ Box beaver │ Parser mascot │
│ Data otter │ Reviewer      │
└────────────┴───────────────┘

# ASCII table

+------------+---------------+
| Animal     | Role          |
+------------+---------------+
| Box beaver | Parser mascot |
| Data otter | Reviewer      |
+------------+---------------+
";

        let parsed = parse_ascii_table(input, true);

        assert_eq!(
            parsed.markdown,
            "# Unicode box table\n\n| Animal | Role |\n| --- | --- |\n| Box beaver | Parser mascot |\n| Data otter | Reviewer |\n\n# ASCII table\n\n| Animal | Role |\n| --- | --- |\n| Box beaver | Parser mascot |\n| Data otter | Reviewer |"
        );
        assert_eq!(parsed.row_count, 4);
        assert_eq!(parsed.column_count, 2);
    }

    #[test]
    fn converts_flattened_pipe_tables_with_an_inline_heading() {
        let input = "| 날짜 | 시간 | 일정 | 대상 | |---|---|---|---| | 월 | 21:00–22:00 | 図書館 | 가족 | | 화 | 17:10–18:10 | 公園 | 친구 | **👨‍👩‍👧‍👦 가족 일정** | 날짜 | 일정 | 라벨 | |---|---|---| | 수 | 영화 | 학교 |";
        let expected = "\
| 날짜 | 시간 | 일정 | 대상 |
| --- | --- | --- | --- |
| 월 | 21:00–22:00 | 図書館 | 가족 |
| 화 | 17:10–18:10 | 公園 | 친구 |

**👨‍👩‍👧‍👦 가족 일정**

| 날짜 | 일정 | 라벨 |
| --- | --- | --- |
| 수 | 영화 | 학교 |";

        let parsed = parse_ascii_table(input, true);

        assert_eq!(parsed.markdown, expected);
        assert_eq!(parsed.row_count, 3);
        assert_eq!(parsed.column_count, 4);
        assert!(parsed.warnings.is_empty());
        assert_eq!(
            markdown_to_html(&parsed.markdown)
                .matches("<table>")
                .count(),
            2
        );
    }

    #[test]
    fn keeps_empty_cells_and_escaped_pipes_in_pipe_tables() {
        let header = r"| 名 | | メモ |";
        let separator = "| :--- | ---: | :---: |";
        let body = r"| | 値 | a \| b |";
        let expected = format!("| 名 |  | メモ |\n{separator}\n|  | 値 | a \\| b |");

        for gap in [" ", "", "\n", "\r\n"] {
            let input = [header, separator, body].join(gap);
            let parsed = parse_ascii_table(&input, true);

            assert_eq!(parsed.markdown, expected);
            assert_eq!(parsed.row_count, 1);
            assert_eq!(parsed.column_count, 3);
            assert_eq!(
                markdown_to_html(&parsed.markdown).matches("a | b").count(),
                1
            );
        }
    }

    #[test]
    fn generates_headers_and_separates_adjacent_flattened_tables() {
        let parsed = parse_ascii_table("|A||---||1||B||C||---|---|---||2|3|4|", false);

        assert_eq!(
            parsed.markdown,
            "| Column 1 |\n| --- |\n| A |\n| 1 |\n\n| Column 1 | Column 2 | Column 3 |\n| --- | --- | --- |\n| B |  | C |\n| 2 | 3 | 4 |"
        );
        assert_eq!(parsed.row_count, 4);
    }

    #[test]
    fn keeps_pipes_in_inline_headings() {
        let parsed = parse_ascii_table("|A||---||1| **📅 AM | PM** |B||---||2|", true);
        assert_eq!(
            parsed.markdown,
            "| A |\n| --- |\n| 1 |\n\n**📅 AM | PM**\n\n| B |\n| --- |\n| 2 |"
        );
    }

    #[test]
    fn prefers_real_line_breaks_and_pads_short_rows() {
        let parsed = parse_ascii_table(
            "| A | | --- | |\n|---|---|---|---|\n| 1 |\n| 2 | 3 | 4 | 5 |",
            true,
        );
        assert_eq!(
            parsed.markdown,
            "| A |  | --- |  |\n| --- | --- | --- | --- |\n| 1 |  |  |  |\n| 2 | 3 | 4 | 5 |"
        );
        assert_eq!(parsed.row_count, 2);
    }

    #[test]
    fn keeps_dash_cells_in_a_complete_physical_row() {
        let parsed = parse_ascii_table("|A|B|C|D|\n|---|---|---|---|\n|x||---||", true);

        assert_eq!(
            parsed.markdown,
            "| A | B | C | D |\n| --- | --- | --- | --- |\n| x |  | --- |  |"
        );
        assert_eq!(parsed.row_count, 1);
    }

    #[test]
    fn does_not_split_flat_body_rows_at_dash_cells() {
        let parsed = parse_ascii_table("|A|B||---|---||1|2||---|4||C||---||3|", true);

        assert_eq!(
            parsed.markdown,
            "| A | B |\n| --- | --- |\n| 1 | 2 |\n| --- | 4 |\n\n| C |\n| --- |\n| 3 |"
        );
        assert_eq!(parsed.row_count, 3);
    }

    #[test]
    fn keeps_header_only_tables_before_inline_headings() {
        let parsed = parse_ascii_table("|A||---| **📅 Next** |B||---||2|", true);

        assert_eq!(
            parsed.markdown,
            "| A |\n| --- |\n\n**📅 Next**\n\n| B |\n| --- |\n| 2 |"
        );
        assert_eq!(parsed.warnings, ["Only a header row was found."]);
    }

    #[test]
    fn keeps_dashes_as_cell_text_inside_a_bordered_box() {
        let parsed = parse_ascii_table("+-----+\n| A   |\n| --- |\n+-----+", true);

        assert_eq!(parsed.markdown, "| A |\n| --- |\n| --- |");
        assert_eq!(parsed.row_count, 1);
    }

    #[test]
    fn renders_markdown_preview_html() {
        let html = markdown_to_html("# Title\n\n| A | B |\n| --- | --- |\n| 1 | 2 |");

        assert!(html.contains("<h1>Title</h1>"));
        assert!(html.contains("<table>"));
        assert!(html.contains("<th>A</th>"));
        assert!(html.contains("<td>1</td>"));
    }
}
