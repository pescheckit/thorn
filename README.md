# Thorn

A fast, extensible linter engine with live framework introspection. Built in Rust.

Thorn provides the **CLI** and **plugin API** — framework-specific intelligence lives in plugins like [thorn-django](https://github.com/anthropics/thorn-django).

## Architecture

```
┌─────────────────────────────────────────────┐
│                  thorn-cli                   │
│         CLI, config, output formatting       │
├─────────────────────────────────────────────┤
│                  thorn-core                  │
│     File discovery, parallel linting (Rayon) │
├─────────────────────────────────────────────┤
│                  thorn-api                   │
│   Plugin trait, Diagnostic, Level, AppGraph  │
└──────────────────────┬──────────────────────┘
                       │ implements Plugin
           ┌───────────┴───────────┐
           │                       │
    ┌──────▼──────┐        ┌───────▼──────┐
    │thorn-django │        │ your-plugin  │
    │ DJ* checks  │        │  XX* checks  │
    │ model graph │        │              │
    │ PyO3 bridge │        │              │
    └─────────────┘        └──────────────┘
```

Thorn itself has **no Django knowledge**. It provides:
- Parallel file discovery and AST parsing (via Ruff's parser)
- A plugin registration system
- Check levels (`fix`, `improve`, `all`)
- Config from `pyproject.toml` (`[tool.thorn]`)
- Text and JSON output
- Inline suppression (`# noqa: XX001`, `# thorn: ignore[XX001]`)

## Quick Start

```sh
# Install a plugin (e.g. thorn-django) and lint
thorn .

# Only show bugs and security issues
thorn . --check=fix

# JSON output for CI/CD
thorn . --format=json

# Exclude patterns
thorn . --exclude "*/migrations/*" --exclude "*/tests/*"

# Ignore specific rules
thorn . --ignore DJ015 --ignore DJ034
```

## Configuration

```toml
# pyproject.toml
[tool.thorn]
exclude = ["*/migrations/*"]
ignore = ["DJ015"]
graph_file = ".thorn/graph.json"
```

## Workspace

| Crate | Description |
|-------|-------------|
| `thorn-api` | Plugin trait, `Diagnostic`, `AppGraph`, `Level` — the stable API plugins depend on |
| `thorn-core` | Linter engine — file discovery, parallel AST linting, graph checks |
| `thorn-cli` | CLI binary — argument parsing, config loading, output formatting |
| `thorn-bridge` | PyO3 bridge utilities for plugins that need Python interop |

## Building

```sh
cargo build --release
./target/release/thorn --help
```

## License

MIT
