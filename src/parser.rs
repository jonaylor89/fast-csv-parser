use color_eyre::eyre::{eyre, Result};
use csv::ReaderBuilder;
use std::collections::HashMap;

#[derive(Debug)]
pub struct CsvParserState {
  line_number: u64,
  headers_processed: bool,
}

#[derive(Debug)]
pub enum SkipComments {
  Boolean(bool),
  String(String),
}

pub struct CsvParserOptions {
  pub(crate) escape: u8,
  pub(crate) quote: u8,
  pub(crate) separator: u8,
  pub(crate) newline: u8,
  pub(crate) _raw: bool,
  pub(crate) strict: bool,
  pub(crate) max_row_bytes: i64,
  pub(crate) headers: Option<Vec<String>>, // None = auto-detect, Some(empty) = no headers/numeric, Some(vec) = custom
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
      _raw: false,
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
      line_number: 0,
      headers_processed: false,
    }
  }
}

pub struct CsvParser {
  pub(crate) state: CsvParserState,
  pub(crate) options: CsvParserOptions,
  pub(crate) headers: Option<Vec<String>>,
  _reader_builder: ReaderBuilder,
}

impl CsvParser {
  pub fn new(mut options: CsvParserOptions) -> Self {
    // Set escape to quote if not defined
    if options.escape == 0 {
      options.escape = options.quote;
    }

    let mut reader_builder = ReaderBuilder::new();
    reader_builder
      .delimiter(options.separator)
      .quote(options.quote)
      .flexible(true) // Allow records with different lengths
      .has_headers(false); // We'll handle headers manually

    // Only set escape if it's different from quote to avoid double processing
    if options.escape != options.quote {
      reader_builder.escape(None); // Disable CSV crate's escape processing
    } else {
      reader_builder.escape(Some(options.escape));
    }

    // Configure terminator based on newline option
    match options.newline {
      b'\n' => reader_builder.terminator(csv::Terminator::CRLF),
      b'\r' => reader_builder.terminator(csv::Terminator::Any(b'\r')),
      other => reader_builder.terminator(csv::Terminator::Any(other)),
    };

    let mut state = CsvParserState::new();

    // Handle headers option
    let headers = if let Some(ref option_headers) = options.headers {
      if !option_headers.is_empty() {
        // Custom headers provided, don't parse first line as headers
        state.headers_processed = true;
        Some(option_headers.clone())
      } else {
        // headers: false - will be set to numeric when first row is encountered
        None
      }
    } else {
      // headers not specified - will be auto-detected
      None
    };

    Self {
      state,
      options,
      headers,
      _reader_builder: reader_builder,
    }
  }

  pub fn parse_line(
    &mut self,
    buffer: &[u8],
    start: usize,
    end: usize,
  ) -> Result<Option<HashMap<String, String>>> {
    if start >= end {
      return Ok(None);
    }

    // Extract the line data and trim newlines
    let mut line_end = end;
    if line_end > start && buffer[line_end - 1] == self.options.newline {
      line_end -= 1;
    }
    if line_end > start && buffer[line_end - 1] == b'\r' {
      line_end -= 1;
    }

    if start >= line_end {
      return Ok(None);
    }

    let line_data = &buffer[start..line_end];

    // Handle skip comments
    if self.should_skip_comment(line_data) {
      return Ok(None);
    }

    // Skip lines if needed
    if let Some(skip_lines) = self.options.skip_lines {
      if self.state.line_number < skip_lines as u64 {
        self.state.line_number += 1;
        return Ok(None);
      }
    }

    // Check maxRowBytes
    if line_data.len() > self.options.max_row_bytes as usize {
      return Err(eyre!("Row exceeds the maximum size"));
    }

    // Parse the line as CSV
    let cells = self.parse_csv_line(line_data)?;

    // Handle headers on first row
    if !self.state.headers_processed {
      self.state.headers_processed = true;
      match &self.options.headers {
        None => {
          // Auto-detect headers from first row
          self.headers = Some(cells);
          self.state.line_number += 1;
          return Ok(None);
        }
        Some(ref headers) if headers.is_empty() => {
          // headers: false - generate numeric column names
          let numeric_headers: Vec<String> = (0..cells.len()).map(|i| i.to_string()).collect();
          self.headers = Some(numeric_headers);
          // Process this row as data
        }
        Some(headers) => {
          // Use provided custom headers
          self.headers = Some(headers.clone());
          // Process this row as data
        }
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

    self.state.line_number += 1;
    Ok(Some(self.write_row(cells)?))
  }

  fn parse_csv_line(&self, line_data: &[u8]) -> Result<Vec<String>> {
    // Always use our custom parsing to handle escape characters properly
    self.parse_line_with_custom_escape(line_data)
  }

  fn parse_line_with_custom_escape(&self, line_data: &[u8]) -> Result<Vec<String>> {
    let line_str = String::from_utf8_lossy(line_data);
    let mut cells = Vec::new();
    let mut current_cell = String::new();
    let mut in_quotes = false;
    let mut chars = line_str.chars().peekable();

    let quote_char = self.options.quote as char;
    let sep_char = self.options.separator as char;
    let escape_char = self.options.escape as char;

    while let Some(ch) = chars.next() {
      if ch == quote_char {
        if in_quotes {
          // Check if this is a double quote (standard CSV escaping)
          if escape_char == quote_char {
            if let Some(&next_ch) = chars.peek() {
              if next_ch == quote_char {
                // Double quote escaping - consume both and add single quote
                chars.next();
                current_cell.push(quote_char);
                continue;
              }
            }
          }
          // End of quoted section
          in_quotes = false;
        } else {
          // Start of quoted section
          in_quotes = true;
        }
      } else if ch == escape_char && in_quotes && escape_char != quote_char {
        // Only handle custom escape characters when they're different from quote
        if let Some(&next_ch) = chars.peek() {
          if next_ch == quote_char {
            // Escaped quote - consume both and add quote to cell
            chars.next();
            current_cell.push(quote_char);
            continue;
          }
        }
        // Not an escaped quote, treat as regular character
        current_cell.push(ch);
      } else if ch == sep_char && !in_quotes {
        // End of cell
        cells.push(current_cell.clone());
        current_cell.clear();
      } else {
        current_cell.push(ch);
      }
    }

    // Add final cell
    cells.push(current_cell);

    Ok(cells)
  }

  fn write_row(&self, cells: Vec<String>) -> Result<HashMap<String, String>> {
    let mut row = HashMap::new();
    let headers = match &self.headers {
      Some(h) => h,
      None => return Err(eyre!("No headers defined")),
    };

    // Handle strict mode
    if self.options.strict && cells.len() != headers.len() {
      return Err(eyre!("Row length does not match headers"));
    }

    for (index, cell) in cells.into_iter().enumerate() {
      if let Some(header) = headers.get(index) {
        if !header.is_empty() && header != "_" {
          row.insert(header.clone(), cell);
        }
      } else if !self.options.strict {
        // Only add extra columns if not in strict mode
        row.insert(format!("_{}", index), cell);
      }
    }

    Ok(row)
  }

  fn should_skip_comment(&self, line_data: &[u8]) -> bool {
    match &self.options.skip_comments {
      Some(SkipComments::Boolean(true)) => {
        let trimmed_start = line_data
          .iter()
          .position(|&x| !x.is_ascii_whitespace())
          .unwrap_or(0);
        line_data.get(trimmed_start) == Some(&b'#')
      }
      Some(SkipComments::String(char_str)) => {
        let trimmed_start = line_data
          .iter()
          .position(|&x| !x.is_ascii_whitespace())
          .unwrap_or(0);
        line_data.get(trimmed_start) == Some(&char_str.as_bytes()[0])
      }
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
    assert!(result.is_none()); // First line is headers

    let result = parser.parse_line(input, 9, 17).unwrap();
    assert_eq!(
      result.expect("Failed to parse line"),
      HashMap::from([
        ("name".to_string(), "John".to_string()),
        ("age".to_string(), "30".to_string())
      ])
    );
  }

  #[test]
  fn test_quoted_fields() {
    let options = CsvParserOptions::default();
    let mut parser = CsvParser::new(options);

    let input = b"name,description\n\"John\",\"Software Engineer\"\n\"Jane\",\"Data Scientist\"";
    let result = parser.parse_line(input, 0, 16).unwrap();
    assert!(result.is_none());

    let result = parser.parse_line(input, 17, 42).unwrap();
    assert_eq!(
      result.expect("Failed to parse line"),
      HashMap::from([
        ("name".to_string(), "John".to_string()),
        ("description".to_string(), "Software Engineer".to_string())
      ])
    );
  }

  #[test]
  fn test_escaped_quotes() {
    let options = CsvParserOptions::default();
    let mut parser = CsvParser::new(options);

    let input = b"text\n\"Hello \"\"World\"\"\"";
    let result = parser.parse_line(input, 0, 4).unwrap();
    assert!(result.is_none());

    let result = parser.parse_line(input, 5, 21).unwrap();
    assert_eq!(
      result.expect("Failed to parse line"),
      HashMap::from([("text".to_string(), "Hello \"World\"".to_string())])
    );
  }

  #[test]
  fn test_custom_separator() {
    let mut options = CsvParserOptions::default();
    options.separator = b';';
    let mut parser = CsvParser::new(options);

    let input = b"name;age\nJohn;30\nJane;25";
    let result = parser.parse_line(input, 0, 8).unwrap();
    assert!(result.is_none());

    let result = parser.parse_line(input, 9, 17).unwrap();
    assert_eq!(
      result.expect("Failed to parse line"),
      HashMap::from([
        ("name".to_string(), "John".to_string()),
        ("age".to_string(), "30".to_string())
      ])
    );
  }

  #[test]
  fn test_strict_mode() {
    let mut options = CsvParserOptions::default();
    options.strict = true;
    let mut parser = CsvParser::new(options);

    let input = b"a,b\n1,2,3";
    let result = parser.parse_line(input, 0, 3).unwrap();
    assert!(result.is_none());

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
    assert!(result.is_none());

    let result = parser.parse_line(input, 4, 12).unwrap();
    assert!(result.is_none());

    let result = parser.parse_line(input, 13, 16).unwrap();
    assert_eq!(
      result.expect("Failed to parse line"),
      HashMap::from([
        ("a".to_string(), "1".to_string()),
        ("b".to_string(), "2".to_string())
      ])
    );
  }

  #[test]
  fn test_custom_headers() {
    let mut options = CsvParserOptions::default();
    options.headers = Some(vec!["col1".to_string(), "col2".to_string()]);
    let mut parser = CsvParser::new(options);

    let input = b"1,2\n3,4";
    let result = parser.parse_line(input, 0, 3).unwrap();
    assert_eq!(
      result.expect("Failed to parse line"),
      HashMap::from([
        ("col1".to_string(), "1".to_string()),
        ("col2".to_string(), "2".to_string())
      ])
    );

    let result = parser.parse_line(input, 4, 7).unwrap();
    assert_eq!(
      result.expect("Failed to parse line"),
      HashMap::from([
        ("col1".to_string(), "3".to_string()),
        ("col2".to_string(), "4".to_string())
      ])
    );
  }

  #[test]
  fn test_csv_crate_integration() {
    // Test complex CSV features enabled by csv crate
    let options = CsvParserOptions::default();
    let mut parser = CsvParser::new(options);

    // Test with complex quoted fields containing separators
    let input = b"name,description\n\"John Doe\",\"Software Engineer, loves coding\"\n\"Jane Smith\",\"Data Scientist\"";

    // Find newline positions
    let mut newlines = Vec::new();
    for (i, &byte) in input.iter().enumerate() {
      if byte == b'\n' {
        newlines.push(i);
      }
    }

    // Parse headers (from 0 to first newline)
    let result = parser.parse_line(input, 0, newlines[0] + 1).unwrap();
    assert!(result.is_none()); // Headers consumed

    // Parse first data row (from first newline+1 to second newline)
    let result = parser
      .parse_line(input, newlines[0] + 1, newlines[1] + 1)
      .unwrap();
    assert_eq!(
      result.expect("Failed to parse complex quoted line"),
      HashMap::from([
        ("name".to_string(), "John Doe".to_string()),
        (
          "description".to_string(),
          "Software Engineer, loves coding".to_string()
        )
      ])
    );

    // Parse second data row (from second newline+1 to end)
    let result = parser
      .parse_line(input, newlines[1] + 1, input.len())
      .unwrap();
    assert_eq!(
      result.expect("Failed to parse second line"),
      HashMap::from([
        ("name".to_string(), "Jane Smith".to_string()),
        ("description".to_string(), "Data Scientist".to_string())
      ])
    );
  }

  #[test]
  fn test_csv_with_mixed_quoting() {
    // Test CSV with some quoted and some unquoted fields
    let options = CsvParserOptions::default();
    let mut parser = CsvParser::new(options);

    let input = b"id,name,value\n1,\"John, Jr.\",100\n2,Jane,200";

    // Find newline positions
    let mut newlines = Vec::new();
    for (i, &byte) in input.iter().enumerate() {
      if byte == b'\n' {
        newlines.push(i);
      }
    }

    // Parse headers (from 0 to first newline)
    let result = parser.parse_line(input, 0, newlines[0] + 1).unwrap();
    assert!(result.is_none());

    // Parse mixed quoted/unquoted row (from first newline+1 to second newline)
    let result = parser
      .parse_line(input, newlines[0] + 1, newlines[1] + 1)
      .unwrap();
    assert_eq!(
      result.expect("Failed to parse mixed quoted line"),
      HashMap::from([
        ("id".to_string(), "1".to_string()),
        ("name".to_string(), "John, Jr.".to_string()),
        ("value".to_string(), "100".to_string())
      ])
    );

    // Parse unquoted row (from second newline+1 to end)
    let result = parser
      .parse_line(input, newlines[1] + 1, input.len())
      .unwrap();
    assert_eq!(
      result.expect("Failed to parse unquoted line"),
      HashMap::from([
        ("id".to_string(), "2".to_string()),
        ("name".to_string(), "Jane".to_string()),
        ("value".to_string(), "200".to_string())
      ])
    );
  }
}
