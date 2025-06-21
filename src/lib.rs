#![deny(clippy::all)]

use napi::{
  bindgen_prelude::{Buffer, Object, Result},
  Env, Error, JsFunction, JsUnknown, Status, ValueType,
};
use parser::{CsvParser as RustCsvParser, CsvParserOptions, SkipComments};
use std::collections::HashMap;

mod parser;

#[macro_use]
extern crate napi_derive;

#[napi(object)]
#[derive(Default)]
pub struct JsCsvParserOptions {
  pub escape: Option<String>,
  pub quote: Option<String>,
  pub separator: Option<String>,
  pub newline: Option<String>,
  pub raw: Option<bool>,
  pub strict: Option<bool>,
  pub max_row_bytes: Option<i64>,
  pub headers: Option<JsUnknown>,
  pub skip_comments: Option<JsUnknown>,
  pub skip_lines: Option<i64>,
  pub map_headers: Option<JsFunction>,
  pub map_values: Option<JsFunction>,
}

#[napi(object)]
pub struct ParsedRow {
  #[napi(writable = true)]
  pub values: Vec<String>,
}

#[napi]
pub struct CsvParser {
  inner: RustCsvParser,
  buffer: Vec<u8>,
  pending_error: Option<String>,
}

#[napi]
impl CsvParser {
  #[napi(constructor)]
  pub fn new(_env: Env, options: Option<JsCsvParserOptions>) -> Result<Self> {
    let opts = if let Some(js_opts) = options {
      let skip_comments: Option<SkipComments> = if let Some(skip_comments) = js_opts.skip_comments {
        let value_type = skip_comments.get_type()?;

        match value_type {
          ValueType::Boolean => {
            let js_bool: napi::JsBoolean = unsafe { skip_comments.cast() };
            let value = js_bool.get_value()?;
            Some(SkipComments::Boolean(value))
          }
          ValueType::String => {
            let js_string: napi::JsString = unsafe { skip_comments.cast() };
            let utf8 = js_string.into_utf8()?;
            let value = utf8.as_str()?;
            Some(SkipComments::String(value.to_string()))
          }
          _ => None,
        }
      } else {
        None
      };

      // let map_headers: Option<ThreadsafeFunction<()>> = js_opts.map_headers.map(|f| {
      //   let func = f.into_threadsafe_function()?;
      //   func
      // });
      // let map_values: Option<TheadsafeFunction<()>> = js_opts.map_values.map(|f| {
      //   let func = f.into_threadsafe_function()?;
      //   func
      // });

      CsvParserOptions {
        escape: js_opts.escape.map(|s| s.as_bytes()[0]).unwrap_or(b'"'),
        quote: js_opts.quote.map(|s| s.as_bytes()[0]).unwrap_or(b'"'),
        separator: js_opts.separator.map(|s| s.as_bytes()[0]).unwrap_or(b','),
        newline: js_opts.newline.map(|s| s.as_bytes()[0]).unwrap_or(b'\n'),
        raw: js_opts.raw.unwrap_or(false),
        strict: js_opts.strict.unwrap_or(false),
        max_row_bytes: js_opts.max_row_bytes.unwrap_or(i64::MAX),
        headers: if let Some(headers_val) = js_opts.headers {
          let value_type = headers_val.get_type()?;
          match value_type {
            ValueType::Boolean => {
              let js_bool: napi::JsBoolean = unsafe { headers_val.cast() };
              let value = js_bool.get_value()?;
              if value {
                // headers: true means auto-detect headers from first row
                None
              } else {
                // headers: false means no headers, use numeric column names
                Some(vec![])
              }
            }
            ValueType::Object => {
              // Assume it's an array
              let js_array: napi::JsObject = unsafe { headers_val.cast() };
              let length: u32 = js_array.get_named_property::<napi::JsNumber>("length")?.get_uint32()?;
              let mut headers = Vec::new();
              for i in 0..length {
                let element: napi::JsString = js_array.get_element(i)?;
                let utf8 = element.into_utf8()?;
                headers.push(utf8.as_str()?.to_string());
              }
              Some(headers)
            }
            _ => None,
          }
        } else {
          None
        },
        skip_comments,
        skip_lines: js_opts.skip_lines,
      }
    } else {
      CsvParserOptions::default()
    };

    Ok(Self {
      inner: RustCsvParser::new(opts),
      buffer: Vec::new(),
      pending_error: None,
    })
  }

  #[napi]
  pub fn push(&mut self, env: Env, chunk: Buffer) -> Result<Vec<Object>> {
    // Check if there's a pending error from previous call
    if let Some(error_msg) = self.pending_error.take() {
      return Err(Error::from_reason(error_msg));
    }

    self.buffer.extend_from_slice(&chunk);

    let mut rows = Vec::new();
    let mut start = 0;
    let mut last_newline = 0;

    let mut is_quoted = false;
    let mut i = 0;
    while i < self.buffer.len() {
      let byte = self.buffer[i];
      // Track quote state to avoid treating quoted newlines as row separators
      if byte == self.inner.options.quote {
        if !is_quoted {
          is_quoted = true;
        } else if i + 1 < self.buffer.len() && self.buffer[i + 1] == self.inner.options.quote {
          // Skip escaped quote - advance past both quote characters
          i += 2;
          continue;
        } else {
          is_quoted = false;
        }
      }
      
      if byte == self.inner.options.newline && !is_quoted {
        match self.inner.parse_line(&self.buffer, start, i + 1) {
          Ok(Some(row)) => {
            let obj = row_to_js_object_ordered(&row, &self.inner.headers, &env)?;
            rows.push(obj);
            last_newline = i + 1;
          }
          Ok(None) => {
            // No row to process (e.g., header line or comment)
            last_newline = i + 1;
          }
          Err(e) => {
            // Remove processed data up to this point
            if last_newline > 0 {
              self.buffer = self.buffer[last_newline..].to_vec();
            }
            // If we have valid rows, store the error for next call and return the rows
            if !rows.is_empty() {
              self.pending_error = Some(e.to_string());
              return Ok(rows);
            }
            return Err(Error::from_reason(e.to_string()));
          }
        }
        start = i + 1;
      }
      i += 1;
    }

    // Remove processed data from buffer
    if last_newline > 0 {
      self.buffer = self.buffer[last_newline..].to_vec();
    }

    Ok(rows)
  }

  #[napi]
  pub fn finish(&mut self, env: Env, _cb: JsFunction) -> Result<Vec<Object>> {
    if self.buffer.is_empty() {
      return Ok(Vec::new());
    }

    let result = self
      .inner
      .parse_line(&self.buffer, 0, self.buffer.len())
      .map_err(|e| Error::from_reason(e.to_string()))?;

    self.buffer.clear();

    match result {
      Some(row) => {
        let obj = row_to_js_object_ordered(&row, &self.inner.headers, &env)?;
        Ok(vec![obj])
      }
      None => Ok(Vec::new()),
    }
  }

  #[napi]
  pub fn get_headers(&self) -> Option<Vec<String>> {
    self.inner.headers.clone()
  }

  #[napi]
  pub fn transform(
    &mut self,
    env: Env,
    chunk: Buffer,
    _enc: String,
    _cb: JsFunction,
  ) -> Result<Vec<Object>> {
    self.buffer.extend_from_slice(&chunk);
    let mut rows = Vec::new();
    let mut start = 0;
    let mut last_newline = 0;

    for (i, &byte) in self.buffer.iter().enumerate() {
      if byte == self.inner.options.newline {
        match self.inner.parse_line(&self.buffer, start, i + 1) {
          Ok(Some(row)) => {
            let obj = row_to_js_object_ordered(&row, &self.inner.headers, &env)?;
            rows.push(obj);
          }
          Ok(None) => {
            // No row to process (e.g., header line or comment)
          }
          Err(e) => {
            return Err(Error::from_reason(e.to_string()));
          }
        }
        start = i + 1;
        last_newline = i + 1;
      }
    }

    // Remove processed data from buffer
    if last_newline > 0 {
      self.buffer = self.buffer[last_newline..].to_vec();
    }

    Ok(rows)
  }

  #[napi]
  pub fn flush(&mut self, env: Env) -> Result<Vec<Object>> {
    // Check if there's a pending error from previous call
    if let Some(error_msg) = self.pending_error.take() {
      return Err(Error::from_reason(error_msg));
    }

    if self.buffer.is_empty() {
      return Ok(Vec::new());
    }

    let result = self
      .inner
      .parse_line(&self.buffer, 0, self.buffer.len())
      .map_err(|e| Error::new(Status::GenericFailure, e.to_string()))?;

    self.buffer.clear();

    match result {
      Some(row) => {
        let obj = row_to_js_object_ordered(&row, &self.inner.headers, &env)?;
        Ok(vec![obj])
      }
      None => Ok(Vec::new()),
    }
  }
}

// Helper function to convert HashMap to JS Object
fn row_to_js_object(row: &HashMap<String, String>, env: &Env) -> Result<Object> {
  let mut obj = env.create_object()?;
  for (key, value) in row {
    obj.set(key, value)?;
  }
  Ok(obj)
}

// Helper function to convert HashMap to JS Object with ordered properties
fn row_to_js_object_ordered(row: &HashMap<String, String>, headers: &Option<Vec<String>>, env: &Env) -> Result<Object> {
  let mut obj = env.create_object()?;
  let mut added_keys = std::collections::HashSet::new();
  
  if let Some(header_vec) = headers {
    // Add properties in header order first
    for header in header_vec {
      if let Some(value) = row.get(header) {
        obj.set(header, value)?;
        added_keys.insert(header.clone());
      }
    }
    
    // Add any remaining properties that weren't in headers (like _3, _4, etc.)
    for (key, value) in row {
      if !added_keys.contains(key) {
        obj.set(key, value)?;
      }
    }
  } else {
    // Fallback to unordered if no headers available
    for (key, value) in row {
      obj.set(key, value)?;
    }
  }
  
  Ok(obj)
}
