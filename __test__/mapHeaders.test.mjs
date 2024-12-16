import test from "ava";
import { collect } from "./helpers/helper.mjs";

test("rename columns", async (t) => {
  const headers = { a: "x", b: "y", c: "z" };
  const mapHeaders = ({ header, index }) => {
    return headers[header];
  };

  const { error, lines } = await collect("basic", { mapHeaders });

  t.false(error, "no err");
  t.snapshot(lines[0], "first row");
  t.is(lines.length, 1, "1 row");
});

test("skip columns a and c", async (t) => {
  const mapHeaders = ({ header, index }) => {
    if (["a", "c"].indexOf(header) > -1) {
      return null;
    }
    return header;
  };

  const { error, lines } = await collect("basic", { mapHeaders });

  t.false(error, "no err");
  t.snapshot(lines[0], "first row");
  t.is(lines.length, 1, "1 row");
