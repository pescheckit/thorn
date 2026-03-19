# Thorn

A fast, extensible linter engine with live framework introspection. Built in Rust.

Thorn is a **library** — it provides the linting engine, plugin API, and CLI toolkit. Framework plugins build on thorn to create their own linter binaries.

## Available Plugins

| Plugin | Binary | Framework |
|--------|--------|-----------|
| [thorn-django](https://github.com/pescheckit/thorn-django) | `thorn-django` | Django / DRF |

## Architecture

```
┌─────────────────────────────────────────────┐
│                  thorn-cli                   │
│    CLI toolkit: run() builds the binary      │
├─────────────────────────────────────────────┤
│                  thorn-core                  │
│     File discovery, parallel linting (Rayon) │
├─────────────────────────────────────────────┤
│                  thorn-api                   │
│  Plugin trait, Diagnostic, Level, AppGraph   │
└─────────────────────────────────────────────┘
```

Plugins depend on these crates and ship their own binary:

```rust
// thorn-django/src/bin/thorn.rs
fn main() {
    thorn_cli::run(|| vec![Box::new(thorn_django::DjangoPlugin::new())]);
}
```

## Building a Plugin

1. Create a new Rust project
2. Depend on `thorn-api` (for the Plugin trait) and `thorn-cli` (for the CLI)
3. Implement the `Plugin` trait
4. Ship a binary that calls `thorn_cli::run()` with your plugin

See [thorn-django](https://github.com/pescheckit/thorn-django) as a reference implementation.

## Crates

| Crate | Description |
|-------|-------------|
| `thorn-api` | Plugin trait, `Diagnostic`, `AppGraph`, `Level` — the stable API |
| `thorn-core` | Linter engine — file discovery, parallel AST linting, graph checks |
| `thorn-cli` | CLI toolkit — `thorn_cli::run(plugins)` builds a complete linter binary |

## License

MIT
