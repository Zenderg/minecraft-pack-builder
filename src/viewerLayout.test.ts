import { readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";

describe("desktop viewer layout CSS", () => {
  const css = [
    "styles.css",
    "styles/appShell.css",
    "styles/library.css",
    "styles/viewer.css",
    "styles/onboarding.css",
    "styles/importJob.css",
    "styles/settings.css",
  ]
    .map((file) => readFileSync(join(process.cwd(), "src", file), "utf8"))
    .join("\n")
    .replace(/\r\n/g, "\n");

  it("does not force the desktop webview into page-scale shrinking", () => {
    const bodyBlock = css.match(/body\s*\{[^}]*\}/)?.[0] ?? "";
    const viewerRegionBlock = css.match(/\.viewer-region\s*\{[^}]*\}/)?.[0] ?? "";

    expect(bodyBlock).not.toMatch(/min-width\s*:/);
    expect(viewerRegionBlock).not.toMatch(/border\s*:/);
    expect(css).not.toMatch(/\.viewer-footer span,\s*\.viewer-footer strong/);
    expect(css).toMatch(/\.viewer-footer > span,\s*\.viewer-footer > strong/);
    expect(css).not.toMatch(/\.viewer-footer-actions/);
    expect(css).not.toMatch(/\.viewer-footer-action/);
    expect(css).toMatch(/\.app-shell\s*\{[^}]*width:\s*100dvw/);
    expect(css).toMatch(/\.viewer-three-canvas\s*\{[^}]*position:\s*absolute/);
  });

  it("keeps the dark theme tokens after CSS file decomposition", () => {
    const rootBlock = css.match(/:root\s*\{[^}]*\}/)?.[0] ?? "";
    const bodyBlock = css.match(/body\s*\{[^}]*\}/)?.[0] ?? "";
    const htmlRootBlock = css.match(/html,\s*#root\s*\{[^}]*\}/)?.[0] ?? "";

    expect(rootBlock).toMatch(/color-scheme:\s*dark/);
    expect(rootBlock).toMatch(/--bg:\s*#0b0f10/);
    expect(rootBlock).toMatch(/--text:\s*#e8f0ec/);
    expect(rootBlock).toMatch(/--muted:\s*#93a19c/);
    expect(rootBlock).toMatch(/--accent:\s*#42d392/);
    expect(rootBlock).toMatch(/--accent-strong:\s*#71e7aa/);
    expect(bodyBlock).toMatch(/background:\s*var\(--bg\)/);
    expect(htmlRootBlock).toMatch(/height:\s*100%/);
    expect(htmlRootBlock).toMatch(/overflow:\s*hidden/);
  });

  it("uses a green sidebar status only for connected AI state", () => {
    const brandStatusBlock = css.match(/\.brand \.brand-status\s*\{[^}]*\}/)?.[0] ?? "";
    const connectedStatusBlock = css.match(/\.brand \.brand-status\.connected\s*\{[^}]*\}/)?.[0] ?? "";

    expect(brandStatusBlock).toMatch(/color:\s*var\(--warning\)/);
    expect(connectedStatusBlock).toMatch(/color:\s*var\(--accent-strong\)/);
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

  it("stacks create-scheme dimension controls vertically", () => {
    const dimensionGridBlock = css.match(/\.dimension-input-grid\s*\{[^}]*\}/)?.[0] ?? "";

    expect(dimensionGridBlock).toMatch(/grid-template-columns:\s*1fr/);
  });
});
