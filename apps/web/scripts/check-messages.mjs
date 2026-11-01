// Keeps every messages/*.json in step with en.json (Phase 24 Decision 9):
// the same leaf keys, and in each value the same {placeholders} and <tags>.
// Plain Node, no dependencies; run with `pnpm check:messages`.
import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";

const dir = join(import.meta.dirname, "..", "messages");
const REFERENCE = "en";

// { a: { b: "x" } } -> Map { "a.b" => "x" }
function flatten(value, prefix = "", out = new Map()) {
  for (const [key, child] of Object.entries(value)) {
    const path = prefix ? `${prefix}.${key}` : key;
    if (child !== null && typeof child === "object") flatten(child, path, out);
    else out.set(path, String(child));
  }
  return out;
}

// The argument names and rich-text tags a value uses. Taking only the
// identifier after "{" keeps ICU plural/select branch bodies out of it.
function tokens(value) {
  const args = [...value.matchAll(/\{\s*(\w+)/g)].map((m) => `{${m[1]}}`);
  const tags = [...value.matchAll(/<\/?(\w+)>/g)].map((m) => m[0]);
  return [...new Set([...args, ...tags])].sort().join(" ");
}

const load = (locale) => flatten(JSON.parse(readFileSync(join(dir, `${locale}.json`), "utf8")));

const locales = readdirSync(dir)
  .filter((file) => file.endsWith(".json"))
  .map((file) => file.slice(0, -".json".length))
  .sort();
const reference = load(REFERENCE);
let problems = 0;

for (const locale of locales.filter((l) => l !== REFERENCE)) {
  const messages = load(locale);
  const report = [];
  for (const key of reference.keys()) {
    if (!messages.has(key)) report.push(`missing ${key}`);
    else if (tokens(messages.get(key)) !== tokens(reference.get(key))) {
      report.push(`${key}: expected "${tokens(reference.get(key))}", got "${tokens(messages.get(key))}"`);
    }
  }
  for (const key of messages.keys()) {
    if (!reference.has(key)) report.push(`extra ${key}`);
  }
  if (report.length > 0) {
    console.error(`${locale}.json: ${report.length} problem(s)`);
    for (const line of report) console.error(`  ${line}`);
    problems += report.length;
  }
}

if (problems > 0) process.exit(1);
console.log(`messages OK: ${locales.join(", ")} (${reference.size} keys)`);
