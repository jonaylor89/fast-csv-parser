import test from "ava";
import { collect } from "./helpers/helper.mjs";

test("maxRowBytes", async (t) => {
  const { error, lines } = await collect("option-maxRowBytes", {
    maxRowBytes: 190,
  });

  t.is(error.message, "Row exceeds the maximum size", "strict row size");
  t.true(lines.length > 1000, "many rows before error");
});
