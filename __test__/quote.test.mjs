import test from "ava";
import { collect } from "./helpers/helper.mjs";

test("custom quote character", async (t) => {
  const { error, lines } = await collect("option-quote", { quote: "'" });

  t.false(error, "no err");
  t.snapshot(lines[0], "first row");
  t.snapshot(lines[1], "second row");
  t.is(lines.length, 2, "2 rows");
});

test("custom quote and escape character", async (t) => {
  const { error, lines } = await collect("option-quote-escape", {
    quote: "'",
    escape: "\\",
  });

  t.false(error, "no err");
  t.snapshot(lines[0], "first row");
  t.snapshot(lines[1], "second row");
  t.snapshot(lines[2], "third row");
  t.is(lines.length, 3, "3 rows");
});

test("quote many", async (t) => {
  const { error, lines } = await collect("option-quote-many", { quote: "'" });

  t.false(error, "no err");
  t.snapshot(lines[0], "first row");
  t.snapshot(lines[1], "second row");
  t.snapshot(lines[2], "third row");
  t.is(lines.length, 3, "3 rows");
});
