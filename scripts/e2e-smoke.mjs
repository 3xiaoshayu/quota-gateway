// End-to-end smoke test: start the portable build in a throw-away copy of the
// portable directory, let it install and start its bundled core on a free
// port, then drive the WebView2 window over the Chrome DevTools Protocol and
// walk the four pages. Unit tests cannot see a Tauri command wired wrong, a
// CSP that blocks an asset, or a page that throws on mount; this can.
//
//   bun run build && ./build.ps1     # produce bin-work first
//   node scripts/e2e-smoke.mjs
//
// Only the executable, the version marker and the bundled core archive are
// copied, so the run never touches bin-work's real config.yaml or auth files;
// the app writes a fresh config into the sandbox. Set E2E_PORTABLE_DIR to test
// another portable directory.
import { spawn } from "node:child_process";
import fs from "node:fs";
import http from "node:http";
import net from "node:net";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright-core";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const source = path.resolve(process.env.E2E_PORTABLE_DIR || path.join(root, "bin-work"));
const STARTUP_TIMEOUT_MS = 60_000;
const CORE_TIMEOUT_MS = 120_000;
const STEP_TIMEOUT_MS = 30_000;
const NAV_PAGES = ["home", "oauth", "agents", "config"];

function log(message) {
  console.log(`[e2e] ${message}`);
}

function freePort() {
  return new Promise((resolve, reject) => {
    const server = net.createServer();
    server.unref();
    server.on("error", reject);
    server.listen(0, "127.0.0.1", () => {
      const { port } = server.address();
      server.close(() => resolve(port));
    });
  });
}

function fetchJson(url) {
  return new Promise((resolve, reject) => {
    const request = http.get(url, (response) => {
      let body = "";
      response.setEncoding("utf8");
      response.on("data", (chunk) => { body += chunk; });
      response.on("end", () => {
        try { resolve(JSON.parse(body)); } catch (error) { reject(error); }
      });
    });
    request.on("error", reject);
    request.setTimeout(2000, () => request.destroy(new Error("timed out")));
  });
}

async function waitFor(check, timeoutMs, label) {
  const deadline = Date.now() + timeoutMs;
  let lastError = null;
  while (Date.now() < deadline) {
    try {
      const value = await check();
      if (value) return value;
    } catch (error) {
      lastError = error;
    }
    await new Promise((resolve) => setTimeout(resolve, 250));
  }
  throw new Error(`${label} did not happen within ${timeoutMs} ms${lastError ? `: ${lastError.message}` : ""}`);
}

function makeSandbox(corePort) {
  const exe = path.join(source, "Quota Gateway.exe");
  if (!fs.existsSync(exe)) throw new Error(`portable build not found: ${exe} (run build.ps1 first)`);
  const archive = fs.readdirSync(path.join(source, "cpa-core")).find((name) => /^CLIProxyAPI_.*\.zip$/.test(name));
  if (!archive) throw new Error(`no bundled core archive in ${path.join(source, "cpa-core")}`);

  const sandbox = fs.mkdtempSync(path.join(os.tmpdir(), "quota-gateway-e2e-"));
  fs.mkdirSync(path.join(sandbox, "cpa-core"));
  for (const name of ["Quota Gateway.exe", "core-version.txt", "portable-app.json"]) {
    fs.copyFileSync(path.join(source, name), path.join(sandbox, name));
  }
  fs.copyFileSync(path.join(source, "cpa-core", archive), path.join(sandbox, "cpa-core", archive));
  // Everything else takes its default; the port must not collide with a
  // gateway that is really running on this machine.
  fs.writeFileSync(path.join(sandbox, "config.toml"), [
    'locale = "zh-CN"',
    `port = ${corePort}`,
    "run-on-startup = false",
    "start-core-on-launch = true",
    "silent-start = false",
    "",
  ].join("\n"));
  return { sandbox, exe: path.join(sandbox, "Quota Gateway.exe") };
}

function killTree(pid) {
  return new Promise((resolve) => {
    const killer = spawn("taskkill", ["/PID", String(pid), "/T", "/F"], { stdio: "ignore", windowsHide: true });
    killer.on("exit", () => resolve());
    killer.on("error", () => resolve());
  });
}

async function removeWithRetry(dir) {
  for (let attempt = 0; attempt < 10; attempt += 1) {
    try {
      fs.rmSync(dir, { recursive: true, force: true });
      return;
    } catch {
      await new Promise((resolve) => setTimeout(resolve, 500));
    }
  }
}

async function main() {
  const corePort = await freePort();
  const cdpPort = await freePort();
  const { sandbox, exe } = makeSandbox(corePort);
  log(`sandbox ${sandbox}, core port ${corePort}, CDP port ${cdpPort}`);

  const child = spawn(exe, [], {
    cwd: sandbox,
    env: {
      ...process.env,
      // WebView2 runs one browser process per user data folder and ignores
      // the arguments of later attachers, so a gateway that is really running
      // would swallow the debugging port; the sandbox gets its own folder.
      WEBVIEW2_USER_DATA_FOLDER: path.join(sandbox, "webview-data"),
      WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${cdpPort}`,
    },
    stdio: ["ignore", "pipe", "pipe"],
    windowsHide: false,
  });
  const processOutput = [];
  child.stdout.on("data", (chunk) => processOutput.push(String(chunk)));
  child.stderr.on("data", (chunk) => processOutput.push(String(chunk)));
  let exited = false;
  child.on("exit", (code) => {
    exited = true;
    log(`app exited with ${code}`);
  });

  const problems = [];
  let probingCsp = false;
  let browser = null;
  let failed = false;
  try {
    const cdpUrl = `http://127.0.0.1:${cdpPort}`;
    await waitFor(async () => {
      if (exited) throw new Error("the app exited before the CDP endpoint came up");
      const version = await fetchJson(`${cdpUrl}/json/version`);
      return !!version?.webSocketDebuggerUrl;
    }, STARTUP_TIMEOUT_MS, "CDP endpoint");
    browser = await chromium.connectOverCDP(cdpUrl);
    const page = await waitFor(async () => (
      browser.contexts().flatMap((context) => context.pages()).find((item) => item.url() !== "about:blank") || null
    ), STARTUP_TIMEOUT_MS, "main window page");
    log(`window url ${page.url()}`);
    page.on("pageerror", (error) => problems.push(`pageerror: ${error.message}`));
    page.on("console", (message) => {
      if (message.type() !== "error") return;
      if (probingCsp && /Content Security Policy|Refused to execute inline script/i.test(message.text())) return;
      problems.push(`console.error: ${message.text()}`);
    });

    await page.waitForSelector(".app-shell", { timeout: STARTUP_TIMEOUT_MS });
    const nav = page.locator("nav.nav-section button");
    await waitFor(async () => (await nav.count()) === NAV_PAGES.length, STEP_TIMEOUT_MS, `${NAV_PAGES.length} navigation entries`);
    log("shell rendered");

    // A build that lost its Content-Security-Policy would still render fine,
    // so probe the policy itself. Code run over the DevTools protocol is
    // exempt from CSP, but a <script> element it inserts is not: with
    // script-src 'self' the inline script must be refused. Chromium logs that
    // refusal as a console error, the one expected error in this run.
    probingCsp = true;
    const inlineScriptRan = await page.evaluate(() => new Promise((resolve) => {
      const script = document.createElement("script");
      script.textContent = "window.__e2eInlineScriptRan = true;";
      document.head.appendChild(script);
      setTimeout(() => resolve(window.__e2eInlineScriptRan === true), 300);
    }));
    probingCsp = false;
    if (inlineScriptRan) {
      throw new Error("the window has no effective Content-Security-Policy: an inline script was allowed to run");
    }
    log("content security policy is enforced");

    // The bundled core is unpacked and started on launch; OAuth and agent
    // pages unlock once it reports running.
    await waitFor(async () => {
      for (let index = 0; index < NAV_PAGES.length; index += 1) {
        if (await nav.nth(index).isDisabled()) return false;
      }
      return true;
    }, CORE_TIMEOUT_MS, "bundled core running (all pages unlocked)");
    const home = await page.locator("main.content").innerText();
    if (!/运行中/.test(home)) throw new Error(`home page does not report a running core:\n${home.slice(0, 400)}`);
    log("bundled core installed and running");

    for (let index = 0; index < NAV_PAGES.length; index += 1) {
      await nav.nth(index).click();
      await waitFor(async () => {
        const active = await page.locator("nav.nav-section button.active").innerText();
        return active === (await nav.nth(index).innerText());
      }, STEP_TIMEOUT_MS, `${NAV_PAGES[index]} page becomes active`);
      await waitFor(async () => (await page.locator("main.content").innerText()).trim().length > 0, STEP_TIMEOUT_MS, `${NAV_PAGES[index]} page renders`);
      // Give effects that fire on mount (status polls, invoke calls) a moment
      // to surface errors before moving on.
      await new Promise((resolve) => setTimeout(resolve, 1500));
      if (await page.locator(".close-dialog-backdrop").count()) throw new Error("the close dialog appeared without a close request");
      log(`${NAV_PAGES[index]} page renders`);
    }

    if (problems.length) {
      throw new Error(`renderer reported problems:\n${problems.join("\n")}`);
    }
    log("OK");
  } catch (error) {
    failed = true;
    throw error;
  } finally {
    try { await browser?.close(); } catch {}
    if (!exited) await killTree(child.pid);
    if (failed && processOutput.length) {
      console.error("[e2e] app output (tail):");
      console.error(processOutput.join("").slice(-4000));
    }
    await removeWithRetry(sandbox);
  }
}

main().catch((error) => {
  console.error(`[e2e] FAILED: ${error.message}`);
  process.exitCode = 1;
});
