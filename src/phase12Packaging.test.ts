import { describe, expect, it } from "vitest";
import { readFileSync } from "node:fs";

describe("phase 12 packaging and updater configuration", () => {
  it("configures Tauri updater artifacts and GitHub Releases latest.json endpoint", () => {
    const config = JSON.parse(readFileSync("src-tauri/tauri.conf.json", "utf8"));

    expect(config.bundle.targets).toEqual(["app", "dmg", "msi", "nsis", "appimage", "deb", "rpm"]);
    expect(config.bundle.createUpdaterArtifacts).toBe(true);
    expect(config.plugins.updater.pubkey).toMatch(/^[A-Za-z0-9+/=]+$/);
    expect(config.plugins.updater.endpoints).toContain(
      "https://github.com/Zenderg/minecraft-pack-builder/releases/latest/download/latest.json",
    );
    expect(config.plugins.updater.windows.installMode).toBe("passive");
  });

  it("keeps release metadata and CI grouped by platform", () => {
    const latestTemplate = JSON.parse(readFileSync("docs/validation/phase-12-latest.template.json", "utf8"));
    const releaseWorkflow = readFileSync(".github/workflows/release.yml", "utf8");

    expect(latestTemplate.platforms).toHaveProperty("darwin-x86_64");
    expect(latestTemplate.platforms).toHaveProperty("darwin-aarch64");
    expect(latestTemplate.platforms).toHaveProperty("windows-x86_64");
    expect(latestTemplate.platforms).toHaveProperty("linux-x86_64");
    expect(releaseWorkflow).toContain("macos-latest");
    expect(releaseWorkflow).toContain("windows-latest");
    expect(releaseWorkflow).toContain("ubuntu-22.04");
    expect(releaseWorkflow).toContain("latest.json");
    expect(releaseWorkflow).toContain("Swatinem/rust-cache@v2");
    expect(releaseWorkflow).toContain("branches:");
    expect(releaseWorkflow).toContain("- main");
    expect(releaseWorkflow).toContain("Upload push build artifacts");
  });

  it("keeps the release workflow on the free unsigned macOS path", () => {
    const releaseWorkflow = readFileSync(".github/workflows/release.yml", "utf8");

    expect(releaseWorkflow).toContain("TAURI_SIGNING_PRIVATE_KEY");
    expect(releaseWorkflow).not.toContain("APPLE_CERTIFICATE");
    expect(releaseWorkflow).not.toContain("APPLE_ID");
    expect(releaseWorkflow).not.toContain("APPLE_TEAM_ID");
  });
});
