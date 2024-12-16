import test from "ava";
import { collect } from "./helpers/helper.mjs";

test("skip lines", async (t) => {
  const { error, lines } = await collect("bad-data", { skipLines: 2 });

  t.false(error, "no err");
  t.is(lines.length, 1, "1 row");
  t.is(
    JSON.stringify(lines[0]),
    JSON.stringify({ yes: "ok", yup: "ok", yeah: "ok!" }),
  );
});

test("skip lines with headers", async (t) => {
  const { error, lines } = await collect("bad-data", {
    headers: ["s", "p", "h"],
    skipLines: 2,
  });

  t.false(error, "no err");
  t.is(lines.length, 2, "2 rows");
  t.is(
    JSON.stringify(lines[0]),
    JSON.stringify({ s: "yes", p: "yup", h: "yeah" }),
  );
  t.is(
    JSON.stringify(lines[1]),
    JSON.stringify({ s: "ok", p: "ok", h: "ok!" }),
  );
});
