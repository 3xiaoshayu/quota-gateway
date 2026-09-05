// Removes style rules from src/styles.css whose selectors require a class that
// nothing under src/ (tsx, ts, other css, index.html) ever mentions. The
// upstream stylesheet styled many pages this fork no longer has; Vite does not
// purge CSS, so those rules shipped with every build.
//
//   node scripts/prune-dead-css.mjs            # rewrite src/styles.css
//   node scripts/prune-dead-css.mjs --dry-run  # report only
//
// Classes inside :not(), :is(), :where() and :has() never make a selector
// unmatchable, so they are ignored when deciding. Class names composed at
// runtime from string fragments would be invisible to this check; the app only
// appends whole tokens ("error", "open", ...), which do appear as literals.
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const stylesPath = path.join(root, "src", "styles.css");
const dryRun = process.argv.includes("--dry-run");

function walk(dir, out = []) {
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const full = path.join(dir, entry.name);
    if (entry.isDirectory()) walk(full, out);
    else out.push(full);
  }
  return out;
}

const corpus = [
  ...walk(path.join(root, "src")).filter((file) => /\.(tsx?|css|html)$/.test(file) && path.resolve(file) !== stylesPath),
  path.join(root, "index.html"),
].map((file) => fs.readFileSync(file, "utf8")).join("\n");
const corpusTokens = new Set(corpus.match(/[\w-]+/g) || []);

// --- a small CSS parser: enough for this stylesheet ------------------------

function parseBlockChildren(text) {
  // Returns nodes covering `text` in order: comments, statements (ending in
  // ";"), and rules/at-rules with a {...} block.
  const nodes = [];
  let index = 0;
  const length = text.length;
  while (index < length) {
    const start = index;
    if (/\s/.test(text[index])) {
      while (index < length && /\s/.test(text[index])) index += 1;
      nodes.push({ type: "space", text: text.slice(start, index) });
      continue;
    }
    if (text.startsWith("/*", index)) {
      const end = text.indexOf("*/", index + 2);
      index = end === -1 ? length : end + 2;
      nodes.push({ type: "comment", text: text.slice(start, index) });
      continue;
    }
    // Prelude up to "{" or ";" at nesting depth 0, honouring strings/parens.
    let depth = 0;
    let quote = null;
    let braceAt = -1;
    let semicolonAt = -1;
    for (let i = index; i < length; i += 1) {
      const ch = text[i];
      if (quote) {
        if (ch === "\\") i += 1;
        else if (ch === quote) quote = null;
        continue;
      }
      if (ch === '"' || ch === "'") { quote = ch; continue; }
      if (ch === "(") depth += 1;
      else if (ch === ")") depth -= 1;
      else if (depth === 0 && ch === "{") { braceAt = i; break; }
      else if (depth === 0 && ch === ";") { semicolonAt = i; break; }
    }
    if (braceAt === -1) {
      const end = semicolonAt === -1 ? length : semicolonAt + 1;
      nodes.push({ type: "statement", text: text.slice(start, end) });
      index = end;
      continue;
    }
    // Find the matching close brace.
    let level = 0;
    let closeAt = -1;
    quote = null;
    for (let i = braceAt; i < length; i += 1) {
      const ch = text[i];
      if (quote) {
        if (ch === "\\") i += 1;
        else if (ch === quote) quote = null;
        continue;
      }
      if (ch === '"' || ch === "'") { quote = ch; continue; }
      if (text.startsWith("/*", i)) { const end = text.indexOf("*/", i + 2); i = end === -1 ? length : end + 1; continue; }
      if (ch === "{") level += 1;
      else if (ch === "}") { level -= 1; if (level === 0) { closeAt = i; break; } }
    }
    if (closeAt === -1) closeAt = length - 1;
    const prelude = text.slice(start, braceAt);
    const body = text.slice(braceAt + 1, closeAt);
    const trimmed = prelude.trim();
    if (trimmed.startsWith("@")) {
      const name = trimmed.match(/^@([\w-]+)/)[1];
      const nested = ["media", "supports", "layer", "container", "document", "scope"].includes(name);
      nodes.push({ type: "at", name, prelude, body, children: nested ? parseBlockChildren(body) : null });
    } else {
      nodes.push({ type: "rule", prelude, body });
    }
    index = closeAt + 1;
  }
  return nodes;
}

function splitTopLevel(selector, separator) {
  const parts = [];
  let depth = 0;
  let current = "";
  let quote = null;
  for (let i = 0; i < selector.length; i += 1) {
    const ch = selector[i];
    if (quote) {
      current += ch;
      if (ch === "\\") { current += selector[i + 1] || ""; i += 1; }
      else if (ch === quote) quote = null;
      continue;
    }
    if (ch === '"' || ch === "'") { quote = ch; current += ch; continue; }
    if (ch === "(") depth += 1;
    if (ch === ")") depth -= 1;
    if (ch === separator && depth === 0) { parts.push(current); current = ""; continue; }
    current += ch;
  }
  parts.push(current);
  return parts;
}

function stripFunctionalPseudos(selector) {
  // Removes the argument lists of :not(), :is(), :where(), :has().
  let out = "";
  let i = 0;
  while (i < selector.length) {
    const match = selector.slice(i).match(/^:(?:not|is|where|has)\(/);
    if (match) {
      let depth = 0;
      let j = i + match[0].length - 1;
      for (; j < selector.length; j += 1) {
        if (selector[j] === "(") depth += 1;
        if (selector[j] === ")") { depth -= 1; if (depth === 0) break; }
      }
      i = j + 1;
      continue;
    }
    out += selector[i];
    i += 1;
  }
  return out;
}

function requiredClasses(complexSelector) {
  const plain = stripFunctionalPseudos(complexSelector).replace(/\[[^\]]*\]/g, "").replace(/"(?:[^"\\]|\\.)*"|'(?:[^'\\]|\\.)*'/g, "");
  return [...plain.matchAll(/\.(-?[_a-zA-Z][\w-]*)/g)].map((m) => m[1]);
}

const removedClasses = new Set();
let droppedRules = 0;
let trimmedRules = 0;

function isUsed(className) {
  return corpusTokens.has(className);
}

function pruneNodes(nodes) {
  const kept = [];
  for (const node of nodes) {
    if (node.type === "rule") {
      const selectors = splitTopLevel(node.prelude, ",");
      const alive = selectors.filter((selector) => {
        const dead = requiredClasses(selector).filter((name) => !isUsed(name));
        for (const name of dead) removedClasses.add(name);
        return dead.length === 0;
      });
      if (alive.length === 0) { droppedRules += 1; continue; }
      if (alive.length !== selectors.length) {
        trimmedRules += 1;
        const leadingSpace = node.prelude.match(/^\s*/)[0];
        kept.push({ ...node, prelude: `${leadingSpace}${alive.map((s) => s.trim()).join(",\n")} ` });
        continue;
      }
      kept.push(node);
      continue;
    }
    if (node.type === "at" && node.children) {
      const children = pruneNodes(node.children);
      if (!children.some((child) => child.type === "rule" || child.type === "at" || child.type === "statement")) {
        droppedRules += 1;
        continue;
      }
      kept.push({ ...node, children });
      continue;
    }
    kept.push(node);
  }
  return kept;
}

function serialize(nodes) {
  return nodes.map((node) => {
    if (node.type === "rule") return `${node.prelude}{${node.body}}`;
    if (node.type === "at") return `${node.prelude}{${node.children ? serialize(node.children) : node.body}}`;
    return node.text;
  }).join("");
}

function collapseBlankRuns(text) {
  return text.replace(/\n{3,}/g, "\n\n");
}

const original = fs.readFileSync(stylesPath, "utf8");
const pruned = collapseBlankRuns(serialize(pruneNodes(parseBlockChildren(original))));
console.log(`styles.css: ${original.split("\n").length} -> ${pruned.split("\n").length} lines`);
console.log(`dropped ${droppedRules} rules/at-rules, trimmed ${trimmedRules} selector lists, ${removedClasses.size} unreferenced classes`);
if (!dryRun) {
  fs.writeFileSync(stylesPath, pruned);
  console.log("written");
}
