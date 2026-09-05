// Removes zh-CN entries whose key no code under src/ mentions. Keys built at
// runtime from a template literal prefix (t(`agents.oauthLoginRequired.${x}`))
// are kept by prefix. tsc then guards the result: every remaining t('...') must
// still resolve to a key.
//
//   node scripts/prune-unused-i18n.mjs            # rewrite the locale file
//   node scripts/prune-unused-i18n.mjs --dry-run  # report only
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const localePath = path.join(root, "src", "i18n", "locales", "zh-CN.ts");
const dryRun = process.argv.includes("--dry-run");

function walk(dir, out = []) {
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const full = path.join(dir, entry.name);
    if (entry.isDirectory()) walk(full, out);
    else out.push(full);
  }
  return out;
}

const code = walk(path.join(root, "src"))
  .filter((file) => /\.(tsx?)$/.test(file) && !file.includes(`${path.sep}locales${path.sep}`))
  .map((file) => fs.readFileSync(file, "utf8"))
  .join("\n");
const dynamicPrefixes = [...code.matchAll(/`([a-zA-Z]+\.[a-zA-Z.]*)\$\{/g)].map((m) => m[1]);

const source = fs.readFileSync(localePath, "utf8");
const eol = source.includes("\r\n") ? "\r\n" : "\n";
const lines = source.split(/\r?\n/);
const kept = [];
const removed = [];
for (const line of lines) {
  const match = line.match(/^\s*'([a-zA-Z0-9_.-]+)':/);
  if (!match) { kept.push(line); continue; }
  const key = match[1];
  const used = code.includes(`'${key}'`) || code.includes(`"${key}"`) || dynamicPrefixes.some((prefix) => key.startsWith(prefix));
  if (used) kept.push(line);
  else removed.push(key);
}
console.log(`zh-CN: ${lines.filter((l) => /^\s*'[^']+':/.test(l)).length} keys, removing ${removed.length} (dynamic prefixes kept: ${dynamicPrefixes.join(", ") || "none"})`);
if (!dryRun) {
  fs.writeFileSync(localePath, kept.join(eol));
  console.log("written");
}
