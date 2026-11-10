# MacPurge

Disk cleanup tool for macOS developers. Scans and removes build artifacts, caches, old logs, and unused application data.

Two components: **CLI** (Rust) and **Desktop App** (Tauri 2 + Next.js). The landing page lives in [macpurge-landing](https://github.com/suenot/macpurge-landing).

## Architecture

```
macclean/
  cli/                    # Rust CLI crate (library + binary)
  desktop/
    src/                  # Next.js 14 frontend (App Router)
    src-tauri/            # Tauri 2 backend (Rust)
```

The desktop app imports the CLI crate as a library (`macpurge = { path = "../../cli" }`), so all scanning and cleaning logic is shared.

## Features

### Disk Overview
- Real-time disk space monitoring for all mounted volumes
- System tray icon with available space display
- DiskArbitration integration for hot-plug detection (USB drives, etc.)
- Automatic tray updates on mount/unmount events

### Dev Artifacts Scanner
- Finds `node_modules`, `target/`, `.venv`, `__pycache__`, `build/`, `.gradle`, `vendor/`, `deps/`, `.stack-work`, `.build/`, `.dart_tool/` across project directories
- Depth-2 scanning: detects projects nested inside grouping folders (e.g. `~/projects/client/repo`)
- Filter by minimum size and age
- Batch cleanup with dry-run support

### Application Manager
- Lists all installed applications with sizes
- Finds data, caches, and support files per app in `~/Library/`
- Detects orphaned remnants from uninstalled apps
- Lazy-loaded per-app size calculation

### Package Manager Caches
- Supports: npm, yarn, pnpm, pip, cargo, rustup, go, gem, brew, composer
- Shows cache locations and sizes for each manager
- Safe cache cleanup per manager

### System Caches & Logs
- Categories: User Cache, System Cache, Browser Cache, System Logs, iOS Backups, Trash, Old Downloads, Mail Caches
- Per-category size breakdown with paths
- Configurable minimum age filter for cleanup

### Dev Tools
- Xcode: DerivedData, Archives, device support files
- Android Studio: Gradle caches, AVD images
- Docker: images, volumes, build cache (sparse file detection with `blocks * 512`)
- Virtual Machines

### AI Models
- Scans local LLM / diffusion model artifacts and reports per-model sizes
- **Ollama**: the `~/.ollama/models` store (blobs are content-addressed, so cleaned as one unit)
- **LM Studio**: per-model directories under `~/.lmstudio/models` (GGUF and MLX)
- **Hugging Face cache**: each repo under `~/.cache/huggingface/hub` plus the Xet dedup block store — auto-categorized as `paroquant`, `vmlx`, `mlx`, `diffusers`, or generic `huggingface` (datasets and auth tokens are left untouched)
- **ComfyUI**: model subfolders (checkpoints, vae, loras, diffusion_models, …) under `~/Documents/ComfyUI/models` and other common install roots
- Per-model selection so you keep what you use and purge the rest

### Project Manager
- Lists projects from configured directories with type detection (Node, Rust, Go, Python, Java, .NET, Ruby, PHP, Elixir, Haskell, Swift, Dart)
- Background size calculation with event-based updates
- Git status per project: uncommitted changes, unpushed commits per remote
- Auto-commit and push to all remotes
- Project archiving with manifest tracking and restore capability

### Recipes
- Predefined cleanup workflows (e.g. "Quick Dev Clean", "Deep Clean")
- Each recipe is a sequence of cleanup steps
- Dry-run estimation before execution
- Step-by-step results

### Scheduled Cleanup
- macOS LaunchAgent integration for automatic periodic cleanup
- Configurable cron schedule
- Selectable task categories (dev artifacts, packages, docker, system, browsers, trash)
- Run history tracking

## CLI Usage

Install with Cargo (`cargo install --git https://github.com/suenot/macpurge macpurge`) or from the desktop app: Settings → Command line tool → Install CLI.

```bash
# Install
cargo install --git https://github.com/suenot/macpurge macpurge

# Show disk info
macpurge disks

# Scan configured project dirs for dev artifacts
macpurge scan-dev
macpurge scan-dev --min-size 100MB --older-than 30

# Scan a specific path
macpurge scan /path/to/projects
macpurge scan --dev --all

# Clean specific artifacts
macpurge clean node-modules /path --dry-run
macpurge clean rust-target /path
macpurge clean python-venv /path

# Package manager caches
macpurge scan-packages
macpurge scan-packages --manager npm

# System caches and logs
macpurge scan-system

# Dev tools (Xcode, Docker, etc.)
macpurge scan-devtools
macpurge scan-devtools --tool docker

# AI models (Ollama, LM Studio, Hugging Face, ComfyUI)
macpurge scan-ai
macpurge scan-ai --tool huggingface
macpurge scan-ai --output json

# Application management
macpurge apps list
macpurge apps remnants

# Recipes
macpurge recipe list
macpurge recipe run quick-dev-clean
macpurge recipe run quick-dev-clean --dry-run

# Schedule
macpurge schedule status
macpurge schedule enable --cron "0 3 * * 0"
macpurge schedule disable
macpurge schedule run

# Configuration
macpurge config --show
macpurge config --init
macpurge config --add-dir ~/projects
macpurge config --remove-dir ~/old-projects

# Claude AI integration
macpurge analyze
macpurge ask "what is taking the most disk space?"
```

## Desktop App

Built with Tauri 2 (Rust backend) and Next.js 14 (React frontend).

### Pages

| Page | Description |
|------|-------------|
| Dashboard | Disk usage overview with donut chart, category breakdown |
| Dev Artifacts | Scan and clean build artifacts across projects |
| Applications | Manage installed apps, find remnants |
| Packages | Package manager cache sizes and cleanup |
| System | System caches, logs, downloads, trash |
| Dev Tools | Xcode, Docker, Android Studio, VMs |
| Projects | Project overview with git status, archive/restore |
| Recipes | One-click cleanup workflows |
| Schedule | Automatic periodic cleanup configuration |
| Settings | Project directories, archive location, scan/cleanup options |

### Running

```bash
cd desktop
npm install
npm run tauri:dev    # Development
npm run tauri:build  # Production build
```

### Key Technologies

- **Tauri 2** with system tray, notifications, shell plugin
- **Next.js 14** with App Router and Turbopack
- **Tailwind CSS** for styling
- **Recharts** and **D3** for data visualization
- **Framer Motion** for animations
- **Lucide React** for icons
- **DiskArbitration** (macOS FFI) for real-time disk monitoring

## Configuration

Config file: `~/.macpurge/config.toml`

```toml
[project_dirs]
dirs = ["~/projects", "~/work"]

[scan]
max_depth = 10
include_hidden = false
artifact_types = ["node_modules", "target", ".venv", "__pycache__"]

[cleanup]
confirm_before_delete = true
use_trash = false
log_deletions = true
older_than_days = 30

# Optional: directory for archiving inactive projects
archive_dir = "~/archive"

[schedule]
enabled = false
cron = "0 3 * * 0"
```

## Building

### Prerequisites
- Rust toolchain (stable)
- Node.js 18+
- Xcode Command Line Tools (for macOS frameworks)

### CLI
```bash
cd cli
cargo build --release
# Binary at: target/release/macpurge
```

### Desktop
```bash
cd desktop
npm install
npm run tauri:build
# App bundle at: src-tauri/target/release/bundle/
```

### Landing Page
```bash
cd landing
npm install
npm run build
```

## Project Structure

```
cli/
  src/
    main.rs              # CLI entry point (clap)
    lib.rs               # Library exports
    scanner/
      disk.rs            # Disk info via df/mount
      directory.rs       # Directory scanning with rayon
      file_types.rs      # Artifact type detection
      packages.rs        # Package manager scanner
      devtools.rs        # Xcode/Docker/Android/VM scanner
      projects.rs        # Project detection, git status, archiving
    cleaners/
      mod.rs             # Clean orchestration
      apps.rs            # Application scanner & remnant finder
      system_scanner.rs  # System caches/logs scanner
      system_cache.rs    # System cache cleaner
      devtools.rs        # Dev tools cleaner
    reporters/           # Output formatting (text, JSON, markdown)
    claude.rs            # Claude AI integration
    config.rs            # TOML config management
    recipes.rs           # Cleanup recipes engine
    schedule.rs          # LaunchAgent scheduler
    utils.rs             # Path expansion, helpers

desktop/
  src-tauri/
    src/
      lib.rs             # Tauri app setup, tray, watcher
      commands/          # 40+ Tauri IPC commands
        disk.rs          # Disk info commands
        scan.rs          # Dev artifact scan
        clean.rs         # Cleanup with progress events
        config.rs        # Config CRUD
        apps.rs          # Application commands
        packages.rs      # Package manager commands
        system.rs        # System scan/clean
        devtools.rs      # Dev tools scan/clean
        recipes.rs       # Recipe execution
        schedule.rs      # Schedule management
        projects.rs      # Project list, archive, git status
      disk_watcher.rs    # DiskArbitration FFI
      tray.rs            # System tray with mpsc channels
  src/
    app/
      page.tsx           # Dashboard
      layout.tsx         # Sidebar navigation
      dev/               # Dev artifacts page
      apps/              # Applications page
      packages/          # Package managers page
      system/            # System caches page
      devtools/          # Dev tools page
      projects/          # Projects overview page
      recipes/           # Recipes page
      schedule/          # Schedule page
      settings/          # Settings page
    lib/
      types.ts           # TypeScript types (mirrors Rust structs)
      tauri.ts           # Tauri IPC wrappers
    components/          # Shared UI components
```

## License

MIT

## Author

Evgeny ([github.com/suenot](https://github.com/suenot))
