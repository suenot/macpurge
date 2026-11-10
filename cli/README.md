# macpurge

CLI tool for cleaning macOS disk space, with special focus on development artifacts and package manager caches.

## Features

- **Scan directories** for cleanable development artifacts
- **Clean dev artifacts**: node_modules, venv, target/, etc.
- **Clean package manager caches**: npm, Cargo, Homebrew, pip
- **Manage applications**: list installed apps, find remnants, uninstall completely
- **Claude integration**: ask questions and get recommendations
- **Multiple output formats**: interactive TUI, JSON, Markdown

## Installation

```bash
cd cli
cargo build --release
# Binary will be at target/release/macpurge
```

Or install directly:

```bash
cargo install --git https://github.com/suenot/macpurge macpurge
```

## Usage

### Scanning

```bash
# Scan current directory
macpurge scan

# Scan specific directory
macpurge scan /path/to/dir

# Scan all disks
macpurge scan --all

# Developer mode (all dev artifacts)
macpurge scan ~/projects --dev

# Output as JSON
macpurge scan ~/projects -o json
```

### Cleaning Dev Artifacts

```bash
# Clean node_modules
macpurge clean node-modules ~/projects

# Clean Python virtual environments
macpurge clean venv ~/projects

# Clean Rust target directories
macpurge clean rust ~/projects

# Clean all dev artifacts
macpurge clean all ~/projects

# Dry run (see what would be deleted)
macpurge clean node-modules ~/projects --dry-run
```

### Cleaning Package Manager Caches

```bash
# Clean npm cache
macpurge clean npm

# Clean Cargo cache
macpurge clean cargo

# Clean Homebrew cache
macpurge clean brew

# Clean pip cache
macpurge clean pip
```

### Application Management

```bash
# List installed applications
macpurge apps list

# Find remnants of uninstalled apps
macpurge apps remnants

# Completely uninstall an application
macpurge apps uninstall "App Name"
```

### Disk Information

```bash
# Show disk usage
macpurge disks
```

### Claude Integration

```bash
# Generate analysis report
macpurge analyze

# Save report to file
macpurge analyze -o report.md

# Ask Claude a question
macpurge ask "what can I clean?"
macpurge ask "which node_modules are safe to delete?"
```

## Dev Artifacts Supported

| Language | Patterns |
|----------|----------|
| Node.js | `node_modules/` |
| Python | `.venv/`, `venv/`, `__pycache__/` |
| Rust | `target/` |
| Go | `vendor/` |
| Java | `.gradle/`, `build/`, `target/` |
| Ruby | `vendor/bundle/` |
| PHP | `vendor/` |
| .NET | `bin/`, `obj/` |
| Elixir | `_build/`, `deps/` |
| Haskell | `.stack-work/` |

## Global Caches

The tool can clean these global caches:

- `~/.npm/` - npm cache
- `~/.cargo/registry/cache/` - Cargo cache
- `~/Library/Caches/Homebrew/` - Homebrew cache
- `~/.cache/pip/` - pip cache
- `~/.gradle/caches/` - Gradle cache
- `~/.m2/repository/` - Maven cache

## Flags

| Flag | Short | Description |
|------|-------|-------------|
| `--dry-run` | `-n` | Show what would be deleted without deleting |
| `--yes` | `-y` | Skip confirmation prompts |
| `--verbose` | `-v` | Show detailed output |
| `--output` | `-o` | Output format (text, json) |
| `--dev` | `-d` | Enable developer mode (scan all artifacts) |
| `--all` | | Scan all user disks |

## Examples

### Find the largest node_modules directories

```bash
macpurge scan ~/projects --dev -o json | jq '.entries | sort_by(.size_bytes) | reverse | .[0:10]'
```

### Clean all caches at once

```bash
macpurge clean npm -y
macpurge clean cargo -y
macpurge clean brew -y
macpurge clean pip -y
```

### Generate report and ask Claude

```bash
macpurge analyze -o /tmp/report.md
macpurge ask "Based on my disk usage, what should I prioritize cleaning?"
```

## Requirements

- macOS 10.15+
- Rust 1.70+ (for building)
- Claude CLI (optional, for Claude integration)

## License

MIT

## Author

Evgeny (github.com/suenot)
