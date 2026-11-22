const { readFileSync, writeFileSync } = require("node:fs");
const [version, output] = process.argv.slice(2);

const changelog = readFileSync("CHANGELOG.md", "utf8").replace(/\r\n/g, "\n");
const headings = [...changelog.matchAll(/^## (.+)$/gm)];
const matching = headings.filter(({ 1: title }) => title.split(" - ")[0] === "[" + version + "]");
if (matching.length !== 1) {
  throw new Error("Expected one CHANGELOG.md section for " + version);
}
const heading = matching[0];
const next = headings[headings.indexOf(heading) + 1];
const notes = changelog.slice(heading.index + heading[0].length, next?.index).trim();
if (!notes) throw new Error("Empty changelog section for " + version);
writeFileSync(output, notes + "\n");
