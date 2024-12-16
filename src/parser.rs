use color_eyre::eyre::{eyre, Result};

#[derive(Debug)]
pub struct CsvParserState {
  escaped: bool,
  quoted: bool,
  first: bool,
  line_number: u64,
  previous_end: usize,
  row_length: usize,
}

#[derive(Debug)]
pub enum SkipComments {
  Boolean(bool),
  String(String),
}

#[derive(Debug)]
pub struct CsvParserOptions {
  pub(crate) escape: u8,
  pub(crate) quote: u8,
  pub(crate) separator: u8,
  pub(crate) newline: u8,
  pub(crate) raw: bool,
  pub(crate) strict: bool,
  pub(crate) max_row_bytes: i64,
  pub(crate) headers: Option<Vec<String>>,
  pub(crate) skip_comments: Option<SkipComments>,
  pub(crate) skip_lines: Option<i64>,
}

impl Default for CsvParserOptions {
  fn default() -> Self {
    Self {
      escape: b'"',
      quote: b'"',
      separator: b',',
      newline: b'\n',
      raw: false,
      strict: false,
      max_row_bytes: i64::MAX,
      headers: None,
      skip_comments: None,
      skip_lines: None,
    }
  }
}

impl CsvParserState {
  pub fn new() -> Self {
    Self {
      escaped: false,
      quoted: false,
      first: true,
      line_number: 0,
      previous_end: 0,
      row_length: 0,
    }
  }
}

#[napi]
pub struct CsvParser {
  pub(crate) state: CsvParserState,
  pub(crate) options: CsvParserOptions,
  pub(crate) headers: Option<Vec<String>>,
}

impl CsvParser {
  pub fn new(options: CsvParserOptions) -> Self {
    Self {
      state: CsvParserState::new(),
      headers: options.headers.clone(),
      options,
    }
  }

  pub fn parse_cell<'a>(&self, buffer: &'a [u8], start: usize, end: usize) -> Result<&'a [u8]> {
    let mut start = start;
    let mut end = end;

    // Handle quoted cells
    if buffer[start] == self.options.quote && buffer[end - 1] == self.options.quote {
      start += 1;
      end -= 1;
    }

    let mut y = start;
    let mut result = Vec::with_capacity(end - start);

    for i in start..end {
      // Handle escape characters
      if buffer[i] == self.options.escape && i + 1 < end && buffer[i + 1] == self.options.quote {
        continue;
      }

      if y != i {
        result.push(buffer[i]);
      }
      y += 1;
    }

    Ok(&buffer[start..y])
  }

  pub fn parse_line(&mut self, buffer: &[u8], start: usize, end: usize) -> Result<Vec<String>> {
    let mut end = end - 1; // trim newline
    if buffer[end - 1] == b'\r' {
      end -= 1;
    }

    let mut cells = Vec::new();
    let mut is_quoted = false;
    let mut offset = start;

    // Handle skip comments
    if self.should_skip_comment(buffer, start) {
      return Ok(vec![]);
    }

    for i in start..end {
      let is_starting_quote = !is_quoted && buffer[i] == self.options.quote;
      let is_ending_quote = is_quoted
        && buffer[i] == self.options.quote
        && i + 1 <= end
        && buffer[i + 1] == self.options.separator;
      let is_escape = is_quoted
        && buffer[i] == self.options.escape
        && i + 1 < end
        && buffer[i + 1] == self.options.quote;

      if is_starting_quote || is_ending_quote {
        is_quoted = !is_quoted;
        continue;
      } else if is_escape {
        continue;
      }

      if buffer[i] == self.options.separator && !is_quoted {
        let cell = self.parse_cell(buffer, offset, i)?;
        cells.push(String::from_utf8_lossy(cell).into_owned());
        offset = i + 1;
      }
    }

    // Handle last cell
    if offset < end {
      let cell = self.parse_cell(buffer, offset, end)?;
      cells.push(String::from_utf8_lossy(cell).into_owned());
    }

    // Handle trailing comma
    if buffer[end - 1] == self.options.separator {
      cells.push(String::new());
    }

    // Handle headers
    if self.state.first {
      self.state.first = false;
      if self.headers.is_none() {
        self.headers = Some(cells.clone());
        return Ok(vec![]);
      }
    }

    // Validate row length if strict mode is enabled
    if self.options.strict {
      if let Some(headers) = &self.headers {
        if cells.len() != headers.len() {
          return Err(eyre!("Row length does not match headers"));
        }
      }
    }

    Ok(cells)
  }

  fn should_skip_comment(&self, buffer: &[u8], start: usize) -> bool {
    match &self.options.skip_comments {
      Some(SkipComments::Boolean(true)) => buffer[start] == b'#',
      Some(SkipComments::String(char)) => buffer[start] == char.as_bytes()[0],
      _ => false,
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_basic_parsing() {
    let options = CsvParserOptions::default();
    let mut parser = CsvParser::new(options);

    let input = b"name,age\nJohn,30\nJane,25";
    let result = parser.parse_line(input, 0, 8).unwrap();
    assert!(result.is_empty()); // First line is headers

    let result = parser.parse_line(input, 9, 17).unwrap();
    assert_eq!(result, vec!["John", "30"]);
  }

  #[test]
  fn test_quoted_fields() {
    let options = CsvParserOptions::default();
    let mut parser = CsvParser::new(options);

    let input = b"name,description\n\"John\",\"Software Engineer\"\n\"Jane\",\"Data Scientist\"";
    let result = parser.parse_line(input, 0, 16).unwrap();
    assert!(result.is_empty());

    let result = parser.parse_line(input, 17, 42).unwrap();
    assert_eq!(result, vec!["John", "Software Engineer"]);
  }

  #[test]
  fn test_escaped_quotes() {
    let options = CsvParserOptions::default();
    let mut parser = CsvParser::new(options);

    let input = b"text\n\"Hello \"\"World\"\"\"";
    let result = parser.parse_line(input, 0, 4).unwrap();
    assert!(result.is_empty());

    let result = parser.parse_line(input, 5, 21).unwrap();
    assert_eq!(result, vec!["Hello \"World\""]);
  }

  #[test]
  fn test_custom_separator() {
    let mut options = CsvParserOptions::default();
    options.separator = b';';
    let mut parser = CsvParser::new(options);

    let input = b"name;age\nJohn;30\nJane;25";
    let result = parser.parse_line(input, 0, 8).unwrap();
    assert!(result.is_empty());

    let result = parser.parse_line(input, 9, 17).unwrap();
    assert_eq!(result, vec!["John", "30"]);
  }

  #[test]
  fn test_strict_mode() {
    let mut options = CsvParserOptions::default();
    options.strict = true;
    let mut parser = CsvParser::new(options);

    let input = b"a,b\n1,2,3";
    let result = parser.parse_line(input, 0, 3).unwrap();
    assert!(result.is_empty());

    let result = parser.parse_line(input, 4, 9);
    assert!(result.is_err());
  }

  #[test]
  fn test_skip_comments() {
    let mut options = CsvParserOptions::default();
    options.skip_comments = Some(SkipComments::Boolean(true));
    let mut parser = CsvParser::new(options);

    let input = b"a,b\n#comment\n1,2";
    let result = parser.parse_line(input, 0, 3).unwrap();
    assert!(result.is_empty());

    let result = parser.parse_line(input, 4, 12).unwrap();
    assert!(result.is_empty());

    let result = parser.parse_line(input, 13, 16).unwrap();
    assert_eq!(result, vec!["1", "2"]);
  }

  #[test]
  fn test_custom_comment_char() {
    let mut options = CsvParserOptions::default();
    options.skip_comments = Some(SkipComments::String("~".to_string()));
    let mut parser = CsvParser::new(options);

    let input = b"a,b\n~comment\n1,2";
    let result = parser.parse_line(input, 0, 3).unwrap();
    assert!(result.is_empty());

    let result = parser.parse_line(input, 4, 12).unwrap();
    assert!(result.is_empty());
  }

  #[test]
  fn test_empty_fields() {
    let options = CsvParserOptions::default();
    let mut parser = CsvParser::new(options);

    let input = b"a,b,c\n,,\n1,,2";
    let result = parser.parse_line(input, 0, 5).unwrap();
    assert!(result.is_empty());

    let result = parser.parse_line(input, 6, 8).unwrap();
    assert_eq!(result, vec!["", "", ""]);

    let result = parser.parse_line(input, 9, 13).unwrap();
    assert_eq!(result, vec!["1", "", "2"]);
  }

  #[test]
  fn test_trailing_comma() {
    let options = CsvParserOptions::default();
    let mut parser = CsvParser::new(options);

    let input = b"a,b,\n1,2,";
    let result = parser.parse_line(input, 0, 5).unwrap();
    assert!(result.is_empty());

    let result = parser.parse_line(input, 6, 10).unwrap();
    assert_eq!(result, vec!["1", "2", ""]);
  }

  #[test]
  fn test_crlf_endings() {
    let options = CsvParserOptions::default();
    let mut parser = CsvParser::new(options);

    let input = b"a,b\r\n1,2\r\n";
    let result = parser.parse_line(input, 0, 4).unwrap();
    assert!(result.is_empty());

    let result = parser.parse_line(input, 5, 9).unwrap();
    assert_eq!(result, vec!["1", "2"]);
  }

  #[test]
  fn test_custom_headers() {
    let mut options = CsvParserOptions::default();
    options.headers = Some(vec!["col1".to_string(), "col2".to_string()]);
    let mut parser = CsvParser::new(options);

    let input = b"1,2\n3,4";
    let result = parser.parse_line(input, 0, 3).unwrap();
    assert_eq!(result, vec!["1", "2"]);

    let result = parser.parse_line(input, 4, 7).unwrap();
    assert_eq!(result, vec!["3", "4"]);
  }
}