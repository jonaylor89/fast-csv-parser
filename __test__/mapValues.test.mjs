import test from "ava";
import { collect } from "./helpers/helper.mjs";

test("map values", async (t) => {
  const headers = [];
  const indexes = [];
  const mapValues = ({ header, index, value }) => {
    headers.push(header);
    indexes.push(index);
    return parseInt(value, 10);
  };

  const { error, lines } = await collect("basic", { mapValues });

  t.false(error, "no err");
  t.snapshot(lines[0], "first row");
  t.is(lines.length, 1, "1 row");
  t.snapshot(headers, "headers");
  t.snapshot(indexes, "indexes");
});

test("map last empty value", async (t) => {
  const mapValues = ({ value }) => {
    return value === "" ? null : value;
  };

  const { error, lines } = await collect("empty-columns", {
    mapValues,
    headers: ["date", "name", "location"],
  });

  t.false(error, "no err");
  t.is(lines.length, 2, "2 rows");
  t.is(lines[0].name, null, "name is mapped");
  t.is(lines[0].location, null, "last value mapped");
});
