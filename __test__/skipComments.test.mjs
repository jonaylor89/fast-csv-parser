import test from "ava";
import { collect } from "./helpers/helper.mjs";

test("comment", async (t) => {
  const { error, lines } = await collect("comment", { skipComments: true });

  t.false(error, "no err");
  t.snapshot(lines);
  t.is(lines.length, 1, "1 row");
});

test("custom comment", async (t) => {
  const { error, lines } = await collect("option-comment", {
    skipComments: "~",
  });

  t.false(error, "no err");
  t.snapshot(lines);
  t.is(lines.length, 1, "1 row");
});
