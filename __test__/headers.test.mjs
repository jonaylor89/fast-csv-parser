import test from "ava";
import { collect } from "./helpers/helper.mjs";

test("custom escape character", async (t) => {
  const { error, lines } = await collect("option-escape", { escape: "\\" });

  t.false(error, "no err");
  t.snapshot(lines[0], "first row");
  t.snapshot(lines[1], "second row");
  t.snapshot(lines[2], "third row");
  t.is(lines.length, 3, "3 rows");
});

test("headers: false", async (t) => {
  const { error, lines } = await collect("no-headers", { headers: false });

  t.false(error, "no err");
  t.snapshot(lines);
});

test("headers option", async (t) => {
  const { error, lines } = await collect("headers", {
    headers: ["a", "b", "c"],
  });

  t.false(error, "no err");
  t.snapshot(lines);
});
