#!/usr/bin/env node

const { createReadStream } = require("fs");
const { basename } = require("path");
const { globby } = require("globby");
const chalk = require("chalk");
const table = require("text-table");
const timeSpan = require("time-span").default;
const strip = require("strip-ansi");

// Import both parsers
const fastCsv = require("./csv-parser.js");
const originalCsv = require("csv-parser");

const runBenchmark = async (csvParser, parserName) => {
  const paths = process.argv[2] || (await globby(["__test__/fixtures/**/*.csv"]));
  const rows = [];

  console.log(`\n🔥 Running ${parserName} benchmark...\n`);

  for (const path of [].concat(paths)) {
    await new Promise((resolve) => {
      let rowsParsed = 0;
      const end = timeSpan();

      createReadStream(path)
        .pipe(csvParser({ raw: true }))
        .on("data", (line) => {
          rowsParsed++;
        })
        .on("error", (err) => {
          const duration = end().toPrecision(2);
          const fileName = chalk.blue(basename(path));
          rows.push([
            "",
            fileName,
            chalk.red("ERROR"),
            chalk.red(`${err.message.substring(0, 30)}...`),
          ]);
          resolve();
        })
        .on("end", () => {
          const duration = end().toPrecision(2);
          const color =
            duration <= 10 ? "green" : duration > 100 ? "red" : "yellow";
          const fileName = chalk.blue(basename(path));
          rows.push(["", fileName, rowsParsed, chalk[color](`${duration}ms`)]);
          resolve();
        });
    });
  }

  rows.unshift(
    ["", "Filename", "Rows Parsed", "Duration"].map((h) =>
      chalk.dim.underline(h),
    ),
  );

  const results = table(rows, {
    align: ["l", "l", "r", "r"],
    stringLength(str) {
      return strip(str).length;
    },
  });

  console.log(results);
  return rows.slice(1); // Remove header row for comparison
};

const compareResults = (fastResults, originalResults) => {
  console.log("\n" + "=".repeat(80));
  console.log("📊 PERFORMANCE COMPARISON");
  console.log("=".repeat(80));

  const comparisons = [];

  for (let i = 0; i < Math.min(fastResults.length, originalResults.length); i++) {
    const fastRow = fastResults[i];
    const originalRow = originalResults[i];

    if (fastRow && originalRow && fastRow.length >= 4 && originalRow.length >= 4) {
      const fileName = strip(fastRow[1]);
      const fastDuration = parseFloat(strip(fastRow[3]).replace('ms', ''));
      const originalDuration = parseFloat(strip(originalRow[3]).replace('ms', ''));

      if (!isNaN(fastDuration) && !isNaN(originalDuration)) {
        const speedup = (originalDuration / fastDuration).toFixed(2);
        const improvement = (((originalDuration - fastDuration) / originalDuration) * 100).toFixed(1);

        let speedupColor = "green";
        if (speedup < 1) speedupColor = "red";
        else if (speedup < 2) speedupColor = "yellow";

        comparisons.push([
          "",
          fileName,
          `${originalDuration}ms`,
          `${fastDuration}ms`,
          chalk[speedupColor](`${speedup}x`),
          chalk[speedupColor](`${improvement}%`),
        ]);
      }
    }
  }

  if (comparisons.length > 0) {
    comparisons.unshift(
      ["", "File", "Original", "Rust", "Speedup", "Improvement"].map((h) =>
        chalk.dim.underline(h),
      ),
    );

    const comparisonTable = table(comparisons, {
      align: ["l", "l", "r", "r", "r", "r"],
      stringLength(str) {
        return strip(str).length;
      },
    });

    console.log(comparisonTable);

    // Calculate overall statistics
    const validComparisons = comparisons.slice(1).filter(row => {
      const speedup = parseFloat(strip(row[4]).replace('x', ''));
      return !isNaN(speedup);
    });

    if (validComparisons.length > 0) {
      const speedups = validComparisons.map(row => parseFloat(strip(row[4]).replace('x', '')));
      const avgSpeedup = (speedups.reduce((a, b) => a + b, 0) / speedups.length).toFixed(2);
      const maxSpeedup = Math.max(...speedups).toFixed(2);
      const minSpeedup = Math.min(...speedups).toFixed(2);

      console.log("\n" + "=".repeat(40));
      console.log("📈 SUMMARY STATISTICS");
      console.log("=".repeat(40));
      console.log(`Average Speedup: ${chalk.green(avgSpeedup + 'x')}`);
      console.log(`Maximum Speedup: ${chalk.green(maxSpeedup + 'x')}`);
      console.log(`Minimum Speedup: ${chalk.yellow(minSpeedup + 'x')}`);
      console.log(`Files Tested: ${validComparisons.length}`);

      if (parseFloat(avgSpeedup) > 1) {
        console.log(`\n🚀 ${chalk.green.bold('RUST IMPLEMENTATION IS FASTER!')}`);
        console.log(`🔥 On average, the Rust implementation is ${chalk.green.bold(avgSpeedup + 'x faster')} than the original!`);
      } else {
        console.log(`\n⚠️  ${chalk.yellow('Original implementation is faster on average')}`);
      }
    }
  }
};

const run = async () => {
  console.log(chalk.bold("🏁 CSV Parser Performance Comparison"));
  console.log("Comparing Rust implementation vs Original JavaScript csv-parser");

  // Run Fast CSV Parser (Rust)
  const fastResults = await runBenchmark(fastCsv, "Fast CSV Parser (Rust)");

  // Run Original CSV Parser (JavaScript)
  const originalResults = await runBenchmark(originalCsv, "Original csv-parser (JavaScript)");

  // Compare results
  compareResults(fastResults, originalResults);
};

run().catch(console.error);
