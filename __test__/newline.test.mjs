import test from "ava";
import { collect } from "./helpers/helper.mjs";

test("newline", async (t) => {
  const { error, lines } = await collect("option-newline", { newline: "X" });

  t.false(error, "no err");
  t.snapshot(lines[0], "first row");
  t.snapshot(lines[1], "second row");
  t.snapshot(lines[2], "third row");
  t.is(lines.length, 3, "3 rows");
});
