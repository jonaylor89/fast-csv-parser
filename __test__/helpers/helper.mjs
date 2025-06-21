import * as fs from "fs";
import * as path from "path";
import csv from "../../csv-parser.js";
import { fileURLToPath } from "url";

// Create __dirname equivalent for ES modules
const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

const read = fs.createReadStream;

// helpers
export function fixture(name) {
  return path.join(__dirname, "../fixtures", name);
}

// Convert collect to return a Promise
export function collect(file, opts) {
  return new Promise((resolve) => {
    const data = read(fixture(`${file}.csv`));
    const lines = [];
    const parser = csv(opts);

    data
      .pipe(parser)
      .on("data", (line) => {
        lines.push(line);
      })
      .on("error", (err) => {
        resolve({ error: err, lines });
      })
      .on("end", () => {
        resolve({ error: false, lines });
      });
  });
}
