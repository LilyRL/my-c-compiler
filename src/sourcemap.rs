pub struct LineMap {
    entries: Vec<(usize, Option<String>, u32)>,
}

impl LineMap {
    pub fn new(source: &str) -> (Self, String) {
        let mut entries = Vec::new();
        let mut display = String::with_capacity(source.len());

        let mut file: Option<String> = None;
        let mut line: u32 = 0;
        let mut offset = 0;

        for text in source.split_inclusive('\n') {
            match parse_marker(text) {
                Some((marker_line, marker_file)) => {
                    if marker_line == 0 {
                        file = None;
                        line = 0;
                    } else {
                        file = Some(marker_file);
                        line = marker_line;
                    }

                    let content = text.trim_end_matches('\n');
                    display.push_str(&" ".repeat(content.len()));
                    display.push_str(&text[content.len()..]);
                }
                None => {
                    entries.push((offset, file.clone(), line));
                    display.push_str(text);
                    line += 1;
                }
            }

            offset += text.len();
        }

        (Self { entries }, display)
    }

    pub fn locate(&self, source: &str, offset: usize) -> Option<(&str, u32, u32)> {
        let index = self
            .entries
            .partition_point(|&(at, ..)| at <= offset)
            .checked_sub(1)?;
        let (line_start, file, line) = &self.entries[index];

        let file = file.as_deref()?;
        let column = source[*line_start..offset].chars().count() as u32 + 1;

        Some((file, *line, column))
    }
}

fn parse_marker(line: &str) -> Option<(u32, String)> {
    let rest = line.trim_start().strip_prefix('#')?.trim_start();

    let digits = rest.len() - rest.trim_start_matches(|c: char| c.is_ascii_digit()).len();
    if digits == 0 {
        return None;
    }

    let (number, rest) = rest.split_at(digits);
    let quoted = rest.trim_start().strip_prefix('"')?;
    let end = quoted.trim_end().rfind('"')?;

    Some((number.parse().ok()?, quoted[..end].to_string()))
}
