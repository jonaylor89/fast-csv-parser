import test from "ava";
import { collect } from "./helpers/helper.mjs";

test("headers: false, numeric column names", async (t) => {
  const { error, lines } = await collect("basic", { headers: false });

  t.false(error, "no err");
  t.snapshot(lines, "lines");
  t.is(lines.length, 2, "2 rows");
});
