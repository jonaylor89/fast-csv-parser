import test from "ava";
import { CsvParser } from "../index.js";

test("random test", (t) => {
  // Example usage
  const parser = new CsvParser({
    separator: ",",
    quote: '"',
    headers: ["name", "age"],
  });

  // Push data chunks
  parser.push(Buffer.from("John,30\nJane,25\nBob,"));

  // Push more data
  parser.push(Buffer.from("45\n"));

  // Finish parsing
  parser.finish();

  // Get headers
  const headers = parser.getHeaders();

  t.deepEqual(headers, ["name", "age"]);
});
