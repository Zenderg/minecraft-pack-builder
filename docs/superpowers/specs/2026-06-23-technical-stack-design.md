# Minecraft Pack Builder: Technical Stack Specification

Date: 2026-06-23

## 1. Summary

Minecraft Pack Builder v1 will be built as a cross-platform desktop application using:

- Tauri for the desktop shell, packaging, native integration, and Rust-to-frontend bridge;
- Rust as the authoritative application core;
- TypeScript, React, and Vite for the frontend application;
- Three.js for GPU-accelerated 3D rendering through WebGL2;
- SQLite for local structured data;
- local app data directories for modpack archives, extracted assets, render caches, and temporary files;
- an official MCP-compatible local server inside the application for external AI clients.

The central architectural decision is that Rust owns domain truth. TypeScript and React are presentation and interaction layers. They may hold transient UI state, but they must not become a second implementation of scheme validation, export rules, modpack parsing, or AI tool semantics.

This stack is chosen because the product needs a consumer-friendly desktop UI, a GPU-backed 3D viewer, local-first storage, strong file and format handling, and a controlled AI integration surface. Tauri plus React/Three.js gives fast iteration on the user experience. Rust gives the application a fast, testable, reliable core for the parts where correctness matters most.

## 2. Alternatives Considered

### 2.1 Recommended: Tauri + React + Three.js + Rust

This is the selected architecture.

Benefits:

- cross-platform desktop distribution without requiring the user to install Java, Node.js, or Python;
- productive UI development through React, TypeScript, Vite, Radix UI, and Tailwind CSS;
- GPU-accelerated 3D viewer through Three.js/WebGL2;
- high-performance local core in Rust for validation, parsing, storage, rendering preparation, and exports;
- clear path to an official MCP-compatible local server;
- reasonable package size and native OS integration compared with heavier desktop web shells.

Risks:

- Tauri webview behavior can differ by OS, especially around graphics, filesystem permissions, and system webview versions;
- Minecraft modpack parsing and asset/model fidelity remain research-heavy regardless of stack;
- `.litematic` and `.schem` export fidelity must be validated with real tools;
- MCP Rust ecosystem maturity must be checked during implementation.

### 2.2 JVM/Kotlin Desktop

A JVM stack could use Java or Kotlin with Compose Desktop, JavaFX, LibGDX, LWJGL, or similar tools.

Benefits:

- closer to Minecraft Java Edition and parts of the modding ecosystem;
- potentially easier reuse of JVM-based Minecraft/NBT/modding libraries;
- familiar runtime model for some Minecraft tooling.

Reasons not selected:

- the app should not require the user to install Java;
- bundling a Java runtime increases package size and distribution complexity;
- consumer desktop UI, packaging, MCP integration, and 3D viewer work look less direct for this product than Tauri/React/Three.js;
- the product does not launch Minecraft or run mods, so JVM proximity is useful but not decisive.

### 2.3 Native Desktop: Avalonia, Qt, egui, or Flutter

A native or near-native stack could reduce the feeling of a web app in a desktop shell.

Benefits:

- potentially stronger native desktop conventions;
- direct access to system UI patterns and graphics APIs;
- good fit for some lower-level desktop applications.

Reasons not selected:

- more custom work would be needed for the 3D viewer, dense UI panels, onboarding, settings, i18n, and AI integration;
- fewer ready-made web UI primitives for the kind of tool surface this product needs;
- higher implementation risk for v1 without clearly reducing the hardest product risks.

## 3. Architecture

The application is split into a Rust core, a Tauri desktop host, and a React frontend.

```text
React / TypeScript / Three.js
  - application shell UI
  - onboarding and settings
  - sidebar, panels, dialogs, i18n
  - Three.js scene and camera interaction
  - selection gestures and transient viewport state
  - calls into Tauri commands

Tauri application host
  - desktop window lifecycle
  - native file dialogs
  - updater integration
  - secure command bridge
  - local MCP server lifecycle
  - frontend/backend event delivery

Rust workspace
  - authoritative domain model
  - CurseForge import and downloads
  - modpack asset parsing and indexing
  - SQLite storage and migrations
  - scheme operations and validation
  - render chunk and mesh preparation
  - material list generation
  - .schem and .litematic export
  - official MCP-compatible tools
```

The frontend can request data, start operations, display progress, and present errors. It should not independently decide whether an agent operation is valid, whether a block state exists, how export formats are assembled, or how a scheme is persisted.

## 4. Rust Workspace Layout

The Rust side should start as a Cargo workspace with focused crates. The goal is separation by responsibility, not excessive fragmentation.

Initial crates:

- `mpb-core`: scheme model, block references, stages, selections, operations, validation contracts, domain errors;
- `mpb-storage`: SQLite connection management, migrations, repositories, app data paths, atomic persistence helpers;
- `mpb-assets`: CurseForge import, downloaded archive handling, modpack parsing, blockstate/model/texture indexing, asset cache metadata;
- `mpb-render`: chunking, mesh preparation, visibility checks, render buffer generation, picking metadata, texture atlas metadata;
- `mpb-export`: `.schem` and `.litematic` export, NBT handling, format-specific validation;
- `mpb-agent`: official MCP server, tool schemas, request validation, active-client tracking, tool-to-core orchestration;
- `app-tauri`: Tauri setup, commands, events, updater wiring, secure storage integration, application lifecycle.

Crates can be merged early if a boundary proves artificial, but `mpb-core` should remain independent from Tauri and UI concerns.

## 5. Frontend Stack

The frontend uses:

- TypeScript;
- React;
- Vite;
- Three.js;
- Radix UI primitives;
- Tailwind CSS;
- a small local component layer for app-specific controls.

Next.js, server-side rendering, and route-heavy web frameworks are out of scope. This is a desktop application, not a website.

State guidelines:

- durable application state lives in Rust and SQLite;
- React state is for UI state, optimistic display state, panels, dialogs, filters, and current viewport interaction;
- long-running operations are represented as Rust tasks that emit progress events to the UI;
- TypeScript types for backend responses should be generated or kept close to serialized Rust types to reduce drift.

Custom styling should stay minimal. The app should use Radix primitives and Tailwind utilities for common UI behavior and reserve custom CSS for the desktop workspace layout, dark theme tokens, viewer sizing, panels, and app-specific components.

## 6. 3D Viewer And Rendering

Three.js is the selected rendering library for v1.

The viewer should use WebGL2 as the baseline renderer. WebGPU is not a v1 requirement because support can vary across desktop webviews and operating systems.

Rust prepares render data. TypeScript and Three.js upload that data to GPU buffers, manage scene objects, camera controls, selection overlays, and display state.

The renderer should not create one Three.js mesh per block. The target rendering model is:

```text
scheme blocks -> Rust chunks -> prepared mesh buffers -> Three.js BufferGeometry -> GPU
```

Expected Rust rendering responsibilities:

- split schemes into render chunks;
- skip internal faces between opaque adjacent blocks where valid;
- prepare compact vertex and index buffers;
- prepare picking and selection metadata;
- emit dirty-chunk updates after scheme edits;
- provide texture atlas or texture binding metadata;
- preserve enough metadata for block and area selection.

Expected frontend rendering responsibilities:

- maintain Three.js scene, renderer, camera, controls, overlays, and resize handling;
- request render chunks and update dirty chunks;
- map pointer interactions to picking requests or picking buffers;
- show selected blocks, selected regions, stages, and future-stage visibility modes;
- keep frame interaction responsive while Rust performs heavy work off the UI path.

Performance prototypes are not required before implementation. The implementation should still be designed so that render preparation is isolated and testable.

## 7. Local Storage

Structured data is stored in SQLite.

SQLite should contain:

- imported modpack records;
- scheme records;
- scheme dimensions and metadata;
- block data or chunked block data;
- construction stages;
- material summaries or cached derivations if useful;
- import status and asset index metadata;
- application settings that are not secrets.

Large files live in managed application data directories:

- downloaded CurseForge modpack files;
- extracted or normalized modpack assets;
- texture atlases and generated render caches;
- temporary import/export files;
- diagnostic artifacts that the user can open from settings.

CurseForge API keys and other secrets must not be stored in SQLite or plaintext config files.

SQLite migrations are managed by Rust. The app should validate migration success at startup and return a user-readable error if local data cannot be opened or migrated.

## 8. Secure Storage

CurseForge API keys are stored in the operating system's secure credential storage through a Rust keyring-style integration.

Target storage:

- macOS: Keychain;
- Windows: Credential Manager or equivalent;
- Linux: Secret Service, KWallet, libsecret, or equivalent environment-supported storage.

If secure storage is unavailable, the app should show a clear error and avoid storing the key in plaintext. A fallback plaintext secret store is not part of v1.

## 9. Network And Import Boundary

All network access goes through Rust backend code.

The frontend must not call CurseForge APIs, download modpack files, or handle the CurseForge API key directly.

Rust network responsibilities:

- read the CurseForge API key from secure storage;
- call CurseForge APIs;
- download selected modpack files;
- report progress and cancellation;
- validate responses;
- store downloaded files in app data directories;
- convert network and import errors into structured user-facing errors.

Recommended implementation direction:

- async Rust runtime suitable for Tauri integration;
- `reqwest` or equivalent mature Rust HTTP client;
- centralized retry, timeout, cancellation, and progress reporting policies.

## 10. Agent Integration

The application provides an official MCP-compatible local server inside the Tauri/Rust application.

V1 supports one active external AI client at a time.

The server starts with the application and is managed by the application lifecycle. The user does not need to start a separate companion CLI or daemon.

UI requirements:

- show whether the MCP server is running;
- show active client status if a client is connected;
- show instructions for supported external clients such as Codex, Claude Code, opencode, or similar tools;
- expose connection details needed by those clients;
- report agent operation errors in a way that is useful to both the UI and the external client.

Agent tool responsibilities:

- expose MCP tool schemas for supported operations;
- validate every request before mutation;
- reject invalid operations atomically;
- call Rust core operations rather than duplicating logic;
- emit UI events after successful mutations;
- expose current selection, material lists, validation results, and export actions.

The v1 architecture should target official MCP protocol compatibility. If the Rust MCP ecosystem has gaps during implementation, the exact transport and library choice is a technical validation item, but the product should not invent a private non-MCP protocol as its default integration.

## 11. Export And Format Handling

V1 export formats:

- Litematica `.litematic`;
- Sponge/WorldEdit `.schem`.

Low-level parsing and serialization should use mature Rust crates where available. The project should not hand-write NBT, compression, archive, or binary parsing code when reliable crates exist.

The application still owns:

- internal scheme model;
- block identifier and block state normalization;
- format-specific export validation;
- user-facing error reporting;
- golden fixture tests for exported files.

Export success means more than producing a file. Exported files should be validated against target tools during development and covered by fixtures where possible.

## 12. Distribution And Updates

The app is distributed through Tauri bundler artifacts for Windows, macOS, and Linux.

Release hosting uses public GitHub Releases.

The updater uses:

- Tauri updater;
- signed update artifacts;
- a static `latest.json` published with GitHub Releases;
- no custom update backend in v1.

Update behavior:

- the app quietly checks for updates on startup;
- network errors or unavailable GitHub endpoints do not interrupt the user;
- if an update is available, the UI shows a non-disruptive banner or menu indicator;
- installation requires explicit user confirmation;
- settings include a toggle to disable automatic update checks;
- settings include a manual `Check for updates` action.

Tauri updater signing is separate from OS code signing. macOS notarization and Windows code signing are distribution topics that should be handled before a public release, but they are not the same as the update signature mechanism.

## 13. Testing Strategy

Rust tests carry the main correctness burden.

Required Rust test coverage:

- unit tests for scheme operations and validation in `mpb-core`;
- migration and repository tests in `mpb-storage`;
- fixture-based modpack and asset parsing tests in `mpb-assets`;
- render chunk and mesh preparation tests in `mpb-render`;
- export validation and golden file tests in `mpb-export`;
- MCP tool request/response and invalid-operation tests in `mpb-agent`.

Frontend tests should be focused:

- component tests for complex UI state and panels;
- Three.js viewer tests only where behavior can be reliably asserted without overfitting to pixels;
- Playwright smoke/e2e tests for onboarding, settings, mocked import flow, scheme open, viewer load, selection display, materials, and export action.

Fixtures should include small synthetic schemes and small synthetic modpack-like asset sets. Real modpacks can be used for manual and exploratory validation, but the automated test suite should avoid depending on large external downloads.

## 14. Technical Validation Items

These items should be validated during implementation, but they do not change the selected stack:

1. Confirm the best Rust crates for NBT, `.schem`, `.litematic`, archive extraction, and compression.
2. Confirm MCP Rust library and transport choices.
3. Confirm Linux secure storage behavior across common desktop environments.
4. Confirm Tauri updater artifact generation and GitHub Releases `latest.json` publishing.
5. Confirm Three.js/WebGL2 behavior in Tauri webviews on Windows, macOS, and Linux.
6. Confirm export fidelity by opening generated `.schem` and `.litematic` files in target Minecraft ecosystem tools.
7. Confirm real CurseForge modpack parsing boundaries for blockstates, models, textures, lang files, and loader-specific differences.

## 15. Non-Goals For This Stack Decision

This technical stack decision does not include:

- embedded LLMs;
- cloud accounts or sync;
- a custom update server;
- React Native;
- Electron;
- Next.js;
- Java runtime requirements for users;
- a separate companion CLI/server as the default v1 AI integration;
- WebGPU as a v1 baseline;
- manual CAD-like block editing as a product requirement.

## 16. Decision

The selected v1 technical stack is:

```text
Desktop shell: Tauri
Core language: Rust
Frontend: TypeScript + React + Vite
UI primitives: Radix UI
Styling: Tailwind CSS with minimal custom CSS
3D viewer: Three.js on WebGL2
Storage: SQLite + app data/cache directories
Secrets: operating-system secure credential storage
Network: Rust backend only
AI integration: official MCP-compatible local server inside the app
Distribution: Tauri bundler + public GitHub Releases
Updates: Tauri updater + signed artifacts + static latest.json
```

The main reason for this choice is architectural fit. The UI can be built quickly with web technologies, the 3D viewer can use the user's GPU through WebGL2, and Rust can own the high-risk local core: modpack import, scheme validation, render preparation, storage, export, and MCP tools.
