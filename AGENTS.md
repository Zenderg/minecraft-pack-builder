# Agent Instructions

- Work only in the main branch by default.
- Do not create or switch to another branch or worktree unless the user explicitly asks for it.
- If a general workflow guide recommends branch or worktree creation, this repository instruction takes precedence.
- Rust is installed through `rustup` in the user's home directory. If `cargo` or `rustc` are not visible in a non-login shell, run commands through a login `zsh` shell so `$HOME/.cargo/env` from `.zprofile`/`.zshrc` is loaded.
- Node.js may be changed with the `n` CLI outside the sandbox if a project toolchain requires a different version; otherwise keep the current version.
