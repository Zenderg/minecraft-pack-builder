type StructuredBackendError = {
  message?: unknown;
  recoveryMessage?: unknown;
  diagnosticPath?: unknown;
};

const fallbackMessage = "The operation failed. Check diagnostics and try again.";

export function formatBackendError(error: unknown): string {
  if (typeof error === "string") {
    return error.trim() || fallbackMessage;
  }

  if (error instanceof Error) {
    return error.message.trim() || fallbackMessage;
  }

  if (isStructuredBackendError(error)) {
    const message = stringValue(error.message);
    const recoveryMessage = stringValue(error.recoveryMessage);
    const diagnosticPath = stringValue(error.diagnosticPath);
    const parts = [message, recoveryMessage].filter(Boolean);
    if (diagnosticPath) {
      parts.push(`Diagnostic report: ${diagnosticPath}`);
    }
    return parts.join(" ").trim() || fallbackMessage;
  }

  return fallbackMessage;
}

function isStructuredBackendError(error: unknown): error is StructuredBackendError {
  return typeof error === "object" && error !== null;
}

function stringValue(value: unknown): string {
  return typeof value === "string" ? value.trim() : "";
}
