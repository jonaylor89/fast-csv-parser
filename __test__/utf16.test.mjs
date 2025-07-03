import test from "ava";
import fs from "fs";
import { fileURLToPath } from "url";
import { dirname, join } from "path";
import csv from "../main.js";

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

test("utf16 › UTF-16 LE encoding", (t) => {
  return new Promise((resolve, reject) => {
    const results = [];
    const filePath = join(__dirname, "fixtures", "bench", "utf16.csv");

    fs.createReadStream(filePath)
      .pipe(csv())
      .on("data", (data) => {
        results.push(data);
      })
      .on("end", () => {
        try {
          t.is(results.length, 2, "Should parse 2 rows");
          t.deepEqual(
            results[0],
            { a: "1", b: "2", c: "3" },
            "First row should be parsed correctly",
          );
          t.deepEqual(
            results[1],
            { a: "4", b: "5", c: "ʤ" },
            "Second row should include Unicode character",
          );
          resolve();
        } catch (error) {
          reject(error);
        }
      })
      .on("error", reject);
  });
});

test("utf16 › UTF-16 BE encoding", (t) => {
  return new Promise((resolve, reject) => {
    const results = [];
    const filePath = join(__dirname, "fixtures", "bench", "utf16-big.csv");

    fs.createReadStream(filePath)
      .pipe(csv())
      .on("data", (data) => {
        results.push(data);
      })
      .on("end", () => {
        try {
          t.is(results.length, 2, "Should parse 2 rows");
          t.deepEqual(
            results[0],
            { a: "1", b: "2", c: "3" },
            "First row should be parsed correctly",
          );
          t.deepEqual(
            results[1],
            { a: "4", b: "5", c: "ʤ" },
            "Second row should include Unicode character",
          );
          resolve();
        } catch (error) {
          reject(error);
        }
      })
      .on("error", reject);
  });
});

test("utf16 › UTF-16 LE with custom options", (t) => {
  return new Promise((resolve, reject) => {
    const results = [];
    const filePath = join(__dirname, "fixtures", "bench", "utf16.csv");

    fs.createReadStream(filePath)
      .pipe(
        csv({
          mapValues: ({ header, value }) => {
            if (header === "c" && value === "ʤ") {
              return "unicode-char";
            }
            return value;
          },
        }),
      )
      .on("data", (data) => {
        results.push(data);
      })
      .on("end", () => {
        try {
          t.is(results.length, 2, "Should parse 2 rows");
          t.deepEqual(
            results[0],
            { a: "1", b: "2", c: "3" },
            "First row should be parsed correctly",
          );
          t.deepEqual(
            results[1],
            { a: "4", b: "5", c: "unicode-char" },
            "Second row should have mapped Unicode character",
          );
          resolve();
        } catch (error) {
          reject(error);
        }
      })
      .on("error", reject);
  });
});

test("utf16 › UTF-16 BE with headers event", (t) => {
  return new Promise((resolve, reject) => {
    const results = [];
    let headers = null;
    const filePath = join(__dirname, "fixtures", "bench", "utf16-big.csv");

    fs.createReadStream(filePath)
      .pipe(csv())
      .on("headers", (headerArray) => {
        headers = headerArray;
      })
      .on("data", (data) => {
        results.push(data);
      })
      .on("end", () => {
        try {
          t.deepEqual(
            headers,
            ["a", "b", "c"],
            "Headers should be detected correctly",
          );
          t.is(results.length, 2, "Should parse 2 rows");
          t.deepEqual(
            results[0],
            { a: "1", b: "2", c: "3" },
            "First row should be parsed correctly",
          );
          t.deepEqual(
            results[1],
            { a: "4", b: "5", c: "ʤ" },
            "Second row should include Unicode character",
          );
          resolve();
        } catch (error) {
          reject(error);
        }
      })
      .on("error", reject);
  });
});
