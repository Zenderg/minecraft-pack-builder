# Minecraft Pack Builder

Local-first desktop application for building and reviewing Minecraft modpack schemes with external AI tools.

## Development

```bash
pnpm install
pnpm test
pnpm build
cargo test --workspace
pnpm tauri dev
```

The desktop host is a Tauri app in `src-tauri`. Rust domain crates live under `crates/`, and the React/Vite frontend lives under `src/`.
