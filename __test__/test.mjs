import test from "ava";
import { collect } from "./helpers/helper.mjs";
import csv from "../main.js";
import { Buffer } from "buffer";

const eol = "\n";

test("simple csv", async (t) => {
  const { error, lines } = await collect("basic");

  t.false(error, "no err");
  t.snapshot(lines[0], "first row");
  t.is(lines.length, 1, "1 row");
});

test("supports strings", async (t) => {
  const parser = csv();
  const lines = [];

  const promise = new Promise((resolve) => {
    parser.on("data", (data) => {
      lines.push(data);
    });
    parser.on("end", () => {
      resolve();
    });
  });

  parser.write(Buffer.from("hello\n"));
  parser.write(Buffer.from("world\n"));
  parser.end();

  await promise;

  t.snapshot(lines[0]);
  t.is(lines.length, 1);
});

test("newlines in a cell", async (t) => {
  const { error, lines } = await collect("newlines");

  t.false(error, "no err");
  t.snapshot(lines[0], "first row");
  t.snapshot(lines[1], "second row");
  t.snapshot(lines[2], "fourth row");
  t.is(lines.length, 3, "3 rows");
});

test("raw escaped quotes", async (t) => {
  const { error, lines } = await collect("escape-quotes");

  t.false(error, "no err");
  t.snapshot(lines[0], "first row");
  t.snapshot(lines[1], "second row");
  t.snapshot(lines[2], "third row");
  t.is(lines.length, 3, "3 rows");
});

test("raw escaped quotes and newlines", async (t) => {
  const { error, lines } = await collect("quotes+newlines");

  t.false(error, "no err");
  t.snapshot(lines[0], "first row");
  t.snapshot(lines[1], "second row");
  t.snapshot(lines[2], "third row");
  t.is(lines.length, 3, "3 rows");
});

test("line with comma in quotes", async (t) => {
  const headers = Buffer.from("a,b,c,d,e\n");
  const line = Buffer.from('John,Doe,120 any st.,"Anytown, WW",08123\n');
  const correct = JSON.stringify({
    a: "John",
    b: "Doe",
    c: "120 any st.",
    d: "Anytown, WW",
    e: "08123",
  });

  const parser = csv();
  const lines = [];

  const promise = new Promise((resolve) => {
    parser.on("data", (data) => {
      lines.push(data);
    });
    parser.on("end", () => {
      resolve();
    });
  });

  parser.write(headers);
  parser.write(line);
  parser.end();

  await promise;

  t.is(JSON.stringify(lines[0]), correct);
});

test("line with newline in quotes", async (t) => {
  const headers = Buffer.from("a,b,c\n");
  const line = Buffer.from(`1,"ha ${eol}""ha"" ${eol}ha",3\n`);
  const correct = JSON.stringify({
    a: "1",
    b: `ha ${eol}"ha" ${eol}ha`,
    c: "3",
  });

  const parser = csv();
  const lines = [];

  const promise = new Promise((resolve) => {
    parser.on("data", (data) => {
      lines.push(data);
    });
    parser.on("end", () => {
      resolve();
    });
  });

  parser.write(headers);
  parser.write(line);
  parser.end();

  await promise;

  t.is(JSON.stringify(lines[0]), correct);
});

test("cell with comma in quotes", async (t) => {
  const headers = Buffer.from("a\n");
  const cell = Buffer.from('"Anytown, WW"\n');
  const correct = "Anytown, WW";

  const parser = csv();
  const lines = [];

  const promise = new Promise((resolve) => {
    parser.on("data", (data) => {
      lines.push(data);
    });
    parser.on("end", () => {
      resolve();
    });
  });

  parser.write(headers);
  parser.write(cell);
  parser.end();

  await promise;

  t.is(lines[0].a, correct);
});

test("cell with newline", async (t) => {
  const headers = Buffer.from("a\n");
  const cell = Buffer.from(`"why ${eol}hello ${eol}there"\n`);
  const correct = `why ${eol}hello ${eol}there`;

  const parser = csv();
  const lines = [];

  const promise = new Promise((resolve) => {
    parser.on("data", (data) => {
      lines.push(data);
    });
    parser.on("end", () => {
      resolve();
    });
  });

  parser.write(headers);
  parser.write(cell);
  parser.end();

  await promise;

  t.is(lines[0].a, correct);
});

test("cell with escaped quote in quotes", async (t) => {
  const headers = Buffer.from("a\n");
  const cell = Buffer.from('"ha ""ha"" ha"\n');
  const correct = 'ha "ha" ha';

  const parser = csv();
  const lines = [];

  const promise = new Promise((resolve) => {
    parser.on("data", (data) => {
      lines.push(data);
    });
    parser.on("end", () => {
      resolve();
    });
  });

  parser.write(headers);
  parser.write(cell);
  parser.end();

  await promise;

  t.is(lines[0].a, correct);
});

test("cell with multibyte character", async (t) => {
  const headers = Buffer.from("a\n");
  const cell = Buffer.from("this ʤ is multibyte\n");
  const correct = "this ʤ is multibyte";

  const parser = csv();
  const lines = [];

  const promise = new Promise((resolve) => {
    parser.on("data", (data) => {
      lines.push(data);
    });
    parser.on("end", () => {
      resolve();
    });
  });

  parser.write(headers);
  parser.write(cell);
  parser.end();

  await promise;

  t.is(lines[0].a, correct, "multibyte character is preserved");
});

test("geojson", async (t) => {
  const { error, lines } = await collect("geojson");

  t.false(error, "no err");
  const lineObj = {
    type: "LineString",
    coordinates: [
      [102.0, 0.0],
      [103.0, 1.0],
      [104.0, 0.0],
      [105.0, 1.0],
    ],
  };
  t.deepEqual(JSON.parse(lines[1].geojson), lineObj, "linestrings match");
});

test("empty columns", async (t) => {
  const { error, lines } = await collect("empty-columns", ["a", "b", "c"]);

  t.false(error, "no err");

  function testLine(row) {
    t.is(Object.keys(row).length, 3, "Split into three columns");
    t.truthy(/^2007-01-0\d$/.test(row.a), "First column is a date");
    t.truthy(row.b !== undefined, "Empty column is in line");
    t.is(row.b.length, 0, "Empty column is empty");
    t.truthy(row.c !== undefined, "Empty column is in line");
    t.is(row.c.length, 0, "Empty column is empty");
  }

  lines.forEach(testLine);
});

test("process all rows", async (t) => {
  const { error, lines } = await collect("large-dataset", {});

  t.false(error, "no err");
  t.is(lines.length, 7268, "7268 rows");
});
