import { readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";

describe("desktop viewer layout CSS", () => {
  const css = readFileSync(join(process.cwd(), "src", "styles.css"), "utf8");

  it("does not force the desktop webview into page-scale shrinking", () => {
    const bodyBlock = css.match(/body\s*\{[^}]*\}/)?.[0] ?? "";
    const viewerRegionBlock = css.match(/\.viewer-region\s*\{[^}]*\}/)?.[0] ?? "";

    expect(bodyBlock).not.toMatch(/min-width\s*:/);
    expect(viewerRegionBlock).not.toMatch(/border\s*:/);
    expect(css).not.toMatch(/\.viewer-footer span,\n\.viewer-footer strong/);
    expect(css).toMatch(/\.viewer-footer > span,\n\.viewer-footer > strong/);
    expect(css).toMatch(/\.app-shell\s*\{[^}]*width:\s*100dvw/);
    expect(css).toMatch(/\.viewer-three-canvas\s*\{[^}]*position:\s*absolute/);
  });
});
