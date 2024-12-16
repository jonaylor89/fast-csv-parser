import test from "ava";
import { collect } from "./helpers/helper.mjs";

test("strict: false - more columns", async (t) => {
  const { error, lines } = await collect("strict-false-more-columns", {
    strict: false,
  });

  const headersFirstLine = Object.keys(lines[0]);
  const headersBrokenLine = Object.keys(lines[1]);
  const headersLastLine = Object.keys(lines[2]);

  t.false(error, "no err");
  t.deepEqual(headersFirstLine, headersLastLine);
  t.deepEqual(headersBrokenLine, ["a", "b", "c", "_3"]);
  t.snapshot(lines[0], "first row");
  t.snapshot(lines[1], "broken row");
  t.snapshot(lines[2], "last row");
  t.is(lines.length, 3, "3 rows");
  t.is(headersBrokenLine.length, 4, "4 columns");
});

test("strict: false - less columns", async (t) => {
  const { error, lines } = await collect("strict-false-less-columns", {
    strict: false,
  });

  const headersFirstLine = Object.keys(lines[0]);
  const headersBrokenLine = Object.keys(lines[1]);
  const headersLastLine = Object.keys(lines[2]);

  t.false(error, "no err");
  t.deepEqual(headersFirstLine, headersLastLine);
  t.deepEqual(headersBrokenLine, ["a", "b"]);
  t.snapshot(lines[0], "first row");
  t.snapshot(lines[1], "broken row");
  t.snapshot(lines[2], "last row");
  t.is(lines.length, 3, "3 rows");
  t.is(headersBrokenLine.length, 2, "2 columns");
});
