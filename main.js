const { Transform } = require("stream");
const { CsvParser } = require("./index.js");

const defaults = {
  escape: '"',
  headers: null,
  mapHeaders: ({ header }) => header,
  mapValues: ({ value }) => value,
  newline: "\n",
  quote: '"',
  raw: false,
  separator: ",",
  skipComments: false,
  skipLines: 0,
  maxRowBytes: Number.MAX_SAFE_INTEGER,
  strict: false,
  outputByteOffset: false,
};

class CsvParserStream extends Transform {
  constructor(options = {}) {
    super({ objectMode: true, highWaterMark: 16 });

    // Handle array of headers as first argument (like original)
    if (Array.isArray(options)) {
      options = { headers: options };
    }

    // Merge with defaults
    this.options = Object.assign({}, defaults, options);

    // Store callbacks
    this.mapHeaders = this.options.mapHeaders;
    this.mapValues = this.options.mapValues;

    // Prepare options for native parser
    const nativeOptions = { ...this.options };
    delete nativeOptions.mapHeaders;
    delete nativeOptions.mapValues;

    // Convert null to appropriate defaults for native parser
    if (nativeOptions.headers === null) {
      delete nativeOptions.headers;
    }
    if (nativeOptions.skipLines === null) {
      nativeOptions.skipLines = 0;
    }

    try {
      this.parser = new CsvParser(nativeOptions);
    } catch (error) {
      // If native parser fails, emit error on next tick
      process.nextTick(() => this.emit("error", error));
      return;
    }

    this.headersEmitted = false;
    this.isFirstRowProcessed = false;
  }

  _transform(chunk, encoding, callback) {
    try {
      // Ensure chunk is a Buffer
      if (typeof chunk === "string") {
        chunk = Buffer.from(chunk, encoding || "utf8");
      }
      const rows = this.parser.push(chunk);
      this._processRows(rows);
      callback();
    } catch (error) {
      this._handleError(error, callback);
    }
  }

  _flush(callback) {
    try {
      const rows = this.parser.flush();
      this._processRows(rows);
      callback();
    } catch (error) {
      this._handleError(error, callback);
    }
  }

  _processRows(rows) {
    for (let row of rows) {
      // Emit headers event on first data row (if not already emitted)
      if (!this.headersEmitted && !this.isFirstRowProcessed) {
        const headers = this.parser.getHeaders();
        if (headers && this.options.headers !== false) {
          this.emit("headers", headers);
          this.headersEmitted = true;
        }
        this.isFirstRowProcessed = true;
      }

      // Process the row
      row = this._processRow(row);

      if (row !== null) {
        if (this.options.outputByteOffset) {
          this.push({ row, byteOffset: 0 }); // Native parser would need to provide actual offset
        } else {
          this.push(row);
        }
      }
    }
  }

  _processRow(row) {
    // Apply mapValues first
    if (this.mapValues !== defaults.mapValues) {
      const headers = this.parser.getHeaders();
      const processedRow = {};

      for (const [key, value] of Object.entries(row)) {
        const index = headers ? headers.indexOf(key) : -1;
        const newValue = this.mapValues({ header: key, index, value });
        processedRow[key] = newValue;
      }
      row = processedRow;
    }

    // Apply mapHeaders (column renaming/filtering)
    if (this.mapHeaders !== defaults.mapHeaders) {
      const headers = this.parser.getHeaders();
      if (headers) {
        const newRow = {};
        for (let i = 0; i < headers.length; i++) {
          const originalHeader = headers[i];
          const mappedHeader = this.mapHeaders({
            header: originalHeader,
            index: i,
          });

          // If mapHeaders returns null, skip this column
          if (mappedHeader !== null && mappedHeader !== undefined) {
            newRow[mappedHeader] = row[originalHeader];
          }
        }
        row = newRow;
      }
    }

    return row;
  }

  _handleError(error, callback) {
    // Convert specific error messages to appropriate error types
    if (error.message === "Row length does not match headers") {
      const rangeError = new RangeError(error.message);
      callback(rangeError);
    } else if (error.message && error.message.includes("nul byte")) {
      // Handle nul byte errors more gracefully
      const processError = new Error("Invalid CSV data: " + error.message);
      callback(processError);
    } else {
      callback(error);
    }
  }
}

// Export function that creates new parser instance (matches original API)
module.exports = function csv(options) {
  return new CsvParserStream(options);
};

// Also export the native parser class for advanced usage
module.exports.CsvParser = CsvParser;
