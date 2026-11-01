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

// Phase 25: the admin-editable homepage keys exist in en.json's HomePage,
// and the web's HOME_TEXT_KEYS matches the API's (apps/api/src/site_content.rs).
function quotedList(file, name) {
  const source = readFileSync(join(import.meta.dirname, file), "utf8");
  const body = new RegExp(`${name}[^=]*=\\s*&?\\[([^\\]]*)\\]`).exec(source)?.[1];
  if (body === undefined) throw new Error(`${name} not found in ${file}`);
  return [...body.matchAll(/"(\w+)"/g)].map((m) => m[1]);
}

const webKeys = quotedList("../lib/site-content.ts", "HOME_TEXT_KEYS");
const apiKeys = quotedList("../../api/src/site_content.rs", "HOME_TEXT_KEYS");
const homeReport = [];
for (const key of webKeys) {
  if (!reference.has(`HomePage.${key}`)) homeReport.push(`HOME_TEXT_KEYS has ${key}, missing from en.json HomePage`);
}
for (const key of webKeys.filter((k) => !apiKeys.includes(k))) homeReport.push(`${key} is in lib/site-content.ts only`);
for (const key of apiKeys.filter((k) => !webKeys.includes(k))) homeReport.push(`${key} is in site_content.rs only`);
if (homeReport.length > 0) {
  console.error(`HOME_TEXT_KEYS: ${homeReport.length} problem(s)`);
  for (const line of homeReport) console.error(`  ${line}`);
  problems += homeReport.length;
}

if (problems > 0) process.exit(1);
console.log(`messages OK: ${locales.join(", ")} (${reference.size} keys)`);
