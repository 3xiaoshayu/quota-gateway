import { describe, expect, it } from 'bun:test';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';

// The upstream project shipped with "csp": null. The window only loads bundled
// assets and talks to Rust over IPC, so a strict policy costs nothing; this
// keeps it from quietly going back to null.
const config = JSON.parse(
  readFileSync(join(import.meta.dir, '..', 'src-tauri', 'tauri.conf.json'), 'utf8'),
) as { app: { security: { csp: Record<string, string> | null; devCsp: Record<string, string> | null } } };

function directives(policy: Record<string, string> | null | undefined) {
  expect(policy).toBeTruthy();
  return policy as Record<string, string>;
}

describe('content security policy', () => {
  it('locks the bundled window down to its own assets and IPC', () => {
    const csp = directives(config.app.security.csp);
    expect(csp['default-src']).toBe("'self'");
    expect(csp['script-src']).toBe("'self'");
    expect(csp['object-src']).toBe("'none'");
    expect(csp['base-uri']).toBe("'none'");
    expect(csp['form-action']).toBe("'none'");
    expect(csp['connect-src'].split(/\s+/).sort()).toEqual(['http://ipc.localhost', 'ipc:']);
  });

  it('never allows eval or remote script anywhere', () => {
    for (const policy of [config.app.security.csp, config.app.security.devCsp]) {
      for (const [directive, value] of Object.entries(directives(policy))) {
        expect(value, directive).not.toContain("'unsafe-eval'");
        if (directive === 'script-src') {
          expect(value).not.toContain("'unsafe-inline'");
          expect(value).not.toMatch(/https?:\/\/(?!localhost:1420\b)/);
        }
      }
    }
  });

  it('only relaxes the development policy for the Vite dev server', () => {
    const dev = directives(config.app.security.devCsp);
    const extra = new Set<string>();
    for (const value of Object.values(dev)) {
      for (const source of value.split(/\s+/)) {
        if (/^(https?|wss?):\/\//.test(source)) extra.add(source);
      }
    }
    expect([...extra].sort()).toEqual(['http://ipc.localhost', 'http://localhost:1420', 'ws://localhost:1420']);
  });
});
