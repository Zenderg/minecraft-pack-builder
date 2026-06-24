import { describe, expect, it } from "vitest";

import { formatBackendError } from "./backendErrors";

describe("formatBackendError", () => {
  it("shows plain Error messages without the JavaScript Error prefix", () => {
    expect(formatBackendError(new Error("Could not export scheme"))).toBe(
      "Could not export scheme",
    );
  });

  it("uses structured backend message, recovery action, and diagnostic path", () => {
    expect(
      formatBackendError({
        message: "Could not export scheme.",
        recoveryMessage: "Choose another destination.",
        diagnosticPath: "/tmp/mpb/diagnostics/export-scheme-10-schem.json",
      }),
    ).toBe(
      "Could not export scheme. Choose another destination. Diagnostic report: /tmp/mpb/diagnostics/export-scheme-10-schem.json",
    );
  });

  it("turns unknown thrown objects into a useful generic message", () => {
    expect(formatBackendError({ unexpected: true })).toBe(
      "The operation failed. Check diagnostics and try again.",
    );
  });
});
