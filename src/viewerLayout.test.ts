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

  it("bounds the right tools rail so expanded sections scroll inside the sidebar", () => {
    const contentGridBlock = css.match(/\.content-grid\s*\{[^}]*\}/)?.[0] ?? "";
    const toolsSidebarBlock = css.match(/\.tools-sidebar\s*\{[^}]*\}/)?.[0] ?? "";
    const toolTreeBlock = css.match(/\.tool-tree\s*\{[^}]*\}/)?.[0] ?? "";

    expect(contentGridBlock).toMatch(/grid-template-rows:\s*minmax\(0,\s*1fr\)/);
    expect(contentGridBlock).toMatch(/height:\s*100%/);
    expect(toolsSidebarBlock).toMatch(/min-height:\s*0/);
    expect(toolsSidebarBlock).toMatch(/overflow:\s*hidden/);
    expect(toolTreeBlock).toMatch(/overflow-y:\s*auto/);
  });
});
