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
pub struct JsCsvParserOptions {
  pub escape: Option<String>,
  pub quote: Option<String>,
  pub separator: Option<String>,
  pub newline: Option<String>,
  pub raw: Option<bool>,
  pub strict: Option<bool>,
  pub max_row_bytes: Option<i64>,
  pub headers: Option<Vec<String>>,
  pub skip_comments: Option<JsUnknown>,
  pub skip_lines: Option<i64>,
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
}

#[napi]
impl CsvParser {
  #[napi(constructor)]
  pub fn new(options: Option<JsCsvParserOptions>) -> Result<Self> {
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

      CsvParserOptions {
        escape: js_opts.escape.map(|s| s.as_bytes()[0]).unwrap_or(b'"'),
        quote: js_opts.quote.map(|s| s.as_bytes()[0]).unwrap_or(b'"'),
        separator: js_opts.separator.map(|s| s.as_bytes()[0]).unwrap_or(b','),
        newline: js_opts.newline.map(|s| s.as_bytes()[0]).unwrap_or(b'\n'),
        raw: js_opts.raw.unwrap_or(false),
        strict: js_opts.strict.unwrap_or(false),
        max_row_bytes: js_opts.max_row_bytes.unwrap_or(i64::MAX),
        headers: js_opts.headers,
        skip_comments,
        skip_lines: js_opts.skip_lines,
      }
    } else {
      CsvParserOptions::default()
    };

    Ok(Self {
      inner: RustCsvParser::new(opts),
      buffer: Vec::new(),
    })
  }

  #[napi]
  pub fn push(&mut self, env: Env, chunk: Buffer) -> Result<Vec<Object>> {
    self.buffer.extend_from_slice(&chunk);

    let mut rows = Vec::new();
    let mut start = 0;
    let mut last_newline = 0;

    for (i, &byte) in self.buffer.iter().enumerate() {
      if byte == self.inner.options.newline {
        if let Ok(Some(row)) = self.inner.parse_line(&self.buffer, start, i + 1) {
          let obj = row_to_js_object(&row, &env)?;
          rows.push(obj);
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
  pub fn finish(&mut self, env: Env, cb: JsFunction) -> Result<Vec<Object>> {
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
        let obj = row_to_js_object(&row, &env)?;
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
    enc: String,
    cb: JsFunction,
  ) -> Result<Vec<Object>> {
    self.buffer.extend_from_slice(&chunk);
    let mut rows = Vec::new();
    let mut start = 0;
    let mut last_newline = 0;

    for (i, &byte) in self.buffer.iter().enumerate() {
      if byte == self.inner.options.newline {
        if let Ok(Some(row)) = self.inner.parse_line(&self.buffer, start, i + 1) {
          let obj = row_to_js_object(&row, &env)?;
          rows.push(obj);
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
        let obj = row_to_js_object(&row, &env)?;
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
