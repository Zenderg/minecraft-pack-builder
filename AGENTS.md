# Agent Instructions

- Work only in the main branch by default.
- Do not create or switch to another branch or worktree unless the user explicitly asks for it.
- If a general workflow guide recommends branch or worktree creation, this repository instruction takes precedence.
- When you need to edit a file that is objectively large and can reasonably be decomposed, decompose the whole file into well-scoped modules, components, hooks, helpers, stylesheets, or other appropriate blocks along meaningful ownership boundaries.
- Do not split files when they are generated, vendored, lockfiles, schema snapshots, or single cohesive artifacts whose decomposition would make the code harder to understand.
- Rust is installed through `rustup` in the user's home directory. If `cargo` or `rustc` are not visible in a non-login shell, run commands through a login `zsh` shell so `$HOME/.cargo/env` from `.zprofile`/`.zshrc` is loaded.
- Node.js may be changed with the `n` CLI outside the sandbox if a project toolchain requires a different version; otherwise keep the current version.
- Do not start or rely on the web/Vite dev server for user-facing validation by default. This is a Tauri desktop app, so prefer testing the full desktop application; if desktop launch must be done manually by the user, say that clearly instead of substituting browser-only validation.
