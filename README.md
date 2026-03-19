# Thorn

A fast, extensible linter engine with live framework introspection. Built in Rust.

Thorn provides the **CLI** and **plugin API** — framework-specific intelligence lives in plugins like [thorn-django](https://github.com/pescheckit/thorn-django).

## Architecture

```
┌─────────────────────────────────────────────┐
│                  thorn-cli                   │
│       CLI, output formatting, config         │
├─────────────────────────────────────────────┤
│                  thorn-core                  │
│     File discovery, parallel linting (Rayon) │
├─────────────────────────────────────────────┤
│                  thorn-api                   │
│  Plugin trait, Diagnostic, Level, AppGraph   │
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

Thorn itself has **no framework knowledge**. It provides:
- Parallel file discovery and AST parsing (via Ruff's Python parser)
- A plugin registration system with dynamic CLI parameters
- Check levels (`fix`, `improve`, `all`)
- Text and JSON output
- Inline suppression (`# noqa: XX001`, `# thorn: ignore[XX001]`)

Plugins own everything framework-specific: model graphs, runtime bridges, settings checks, and their own config sections.

## Quick Start

```sh
# Lint (plugins add their own flags)
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
```

Plugins define their own config sections (e.g. `[tool.thorn-django]`).

## Workspace

| Crate | Description |
|-------|-------------|
| `thorn-api` | Plugin trait, `Diagnostic`, `AppGraph`, `Level` — the stable API plugins depend on |
| `thorn-core` | Linter engine — file discovery, parallel AST linting, graph checks |
| `thorn-cli` | CLI binary — argument parsing, config loading, output formatting |

## Plugin System

Plugins declare their own CLI parameters:
```
--{plugin-name}-{param}
```

For example, [thorn-django](https://github.com/pescheckit/thorn-django) adds:
```
--django-settings    Django settings module
--django-graph-file  Pre-generated model graph
```

## Installation

```sh
cargo install --git https://github.com/pescheckit/thorn
```

## License

MIT
