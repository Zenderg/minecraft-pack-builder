# Agent Instructions

This file is the source of truth for repository-wide agent workflow and maintenance rules. Product,
architecture, subsystem, and validation details belong in the focused documents linked from
`docs/README.md`, not in this file.

- Work only in the main branch by default.
- Do not create or switch to another branch or worktree unless the user explicitly asks for it.
- If a general workflow guide recommends branch or worktree creation, this repository instruction takes precedence.
- Subagents may be launched when they are useful for parallel investigation, review, or implementation support.
- Do not propose MVP, v1, "start simple", or temporary starter solutions by default. Prefer complete, robust, prod-like solutions immediately.
- Before committing, record any new durable information that surfaced in the current dialogue in an appropriate project file, if such new information exists. This includes product decisions, architecture constraints, technology stack details, OS/tooling quirks, performance findings, debugging lessons, validation requirements, and any other project-specific knowledge that would help future agents avoid repeating the same investigation or mistake.
- When you need to edit a file that is objectively large and can reasonably be decomposed, decompose the whole file into well-scoped modules, components, hooks, helpers, stylesheets, or other appropriate blocks along meaningful ownership boundaries.
- Do not split files when they are generated, vendored, lockfiles, schema snapshots, or single cohesive artifacts whose decomposition would make the code harder to understand.
- Rust is installed through `rustup` in the user's home directory. If `cargo` or `rustc` are not visible in a non-login shell, run commands through a login `zsh` shell so `$HOME/.cargo/env` from `.zprofile`/`.zshrc` is loaded.
- Node.js may be changed with the `n` CLI outside the sandbox if a project toolchain requires a different version; otherwise keep the current version.
- Do not start or rely on the web/Vite dev server for user-facing validation by default. This is a Tauri desktop app, so prefer testing the full desktop application; if desktop launch must be done manually by the user, say that clearly instead of substituting browser-only validation.
- Do not create tests just to satisfy a process. Tests must verify meaningful product or code behavior. Do not test documentation wording or specs with automated tests unless the user explicitly asks for that exact kind of guard.
- Before adding or changing user-facing UI behavior that is not explicitly specified in the product specs, stop and ask the user to confirm the intended flow. Examples include where an action lives, whether it opens a menu or modal, which fields are shown, and the order of user decisions.

## Documentation Maintenance

- Keep documentation organized by ownership and purpose, not by convenience.
- User-facing documents, such as `README.md` and `SECURITY.md`, should stay reader-facing. Do not add agent-only purpose boilerplate there unless it also helps the intended human reader.
- Internal and agent-facing documentation should start with a short purpose statement explaining what the document is the source of truth for, what belongs in it, and what should be documented somewhere else.
- Before adding information to an existing document, read its opening purpose statement and make sure the new content belongs there.
- Create a new focused document when the information introduces a distinct long-lived topic, workflow, subsystem, contract, or troubleshooting area that does not clearly fit an existing document.
- Do not append unrelated discoveries to a nearby or familiar document just because it already exists.
- Prefer updating an existing document when the new information clarifies, corrects, or extends that document's stated purpose.
- Avoid turning docs into chronological work logs. Temporary plans, investigation notes, and shipped implementation details should be removed, condensed, or moved into durable product, architecture, development, or decision documentation as appropriate.
- When adding or changing docs, preserve links between related documents so future agents can find the right source of truth.
