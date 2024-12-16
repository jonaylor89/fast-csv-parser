import test from "ava";
import { collect } from "./helpers/helper.mjs";

test("maxRowBytes", async (t) => {
  const { error, lines } = await collect("option-maxRowBytes", {
    maxRowBytes: 200,
  });

  t.is(error.message, "Row exceeds the maximum size", "strict row size");
  t.is(lines.length, 4576, "4576 rows before error");
});
