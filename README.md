# Klink

**Deterministic, offline static dependency graph and blast-radius impact analyzer for Linux, Windows, and macOS.**

No AI. No LLM. No cloud. Pure parsing, graph theory, indexing, and raw WebGL2 (`#version 300 es`) GPU instancing.

Klink scans an entire repository tree, extracts file-level and symbol-level relationships across all 8 Tier-1 languages plus PE/ELF/Mach-O binaries, and answers one core engineering question interactively: *if I modify this file or function, what else is affected, and how?*

## Core Highlights

- **All 8 Tier-1 Languages Out of the Box:** C, C++, Rust, Go, Java, JavaScript, TypeScript, and Python, plus build manifests (`Cargo.toml`, `package.json`, `Makefile`, `CMakeLists.txt`), config text references (Aho-Corasick), and pure-Rust PE/ELF/Mach-O binary import/export inspection (`object`).
- **Transparency Over False Confidence:** Every edge carries an explicit confidence tag—**`Static`** (solid blue), **`Heuristic`** (dashed amber), or **`Unresolved`** (dotted red). Dynamic imports, reflection (`Class.forName`, `importlib`, `eval`), and macros are surfaced honestly instead of guessed.
- **Folder-Layered WebGL2 Visualizer:** Renders 20,000–50,000+ nodes at 60fps in a thin custom WebGL2 engine with non-overlapping folder islands, zoom-adaptive sub-linear node sizing, screen-space breathing drift, O(1) framebuffer mouse picking, and `R32UI` integer-texture blast-radius highlighting.
- **Native Desktop UX:** OS-native folder picker (`rfd`), `Ctrl/Cmd-K` file and symbol fly-to search, language and confidence filters, live **Rescan / Diff** drawer, keyboard navigation (`R`, `Space`, `+`, `-`, `Esc`), and `aria-live` accessibility announcements.
- **Zero-Copy Rust Core:** Forward and reverse Compressed Sparse Row (CSR) graphs with `mmap` loading, iterative Tarjan SCC cycle detection, condensation DAG, Brandes betweenness centrality, bounded LRU Roaring hub cache, and `redb` ACID symbol/cross-reference storage.

## Measured Performance (20,000-Node Benchmark Suite)

Verified on local Linux x86_64 hardware via `cargo run --release -p klink-cli --bin klink-bench`:

| Benchmark Gate | Target Budget | Measured Result | Status |
| :--- | :--- | :--- | :--- |
| CSR + Iterative Tarjan SCC + Condensation (20k nodes) | < 100 ms | **5.93 ms** | PASS |
| Zero-copy `mmap` CSR project open | < 150 ms | **0.040 ms** | PASS |
| P99 single-source impact query (20k nodes) | < 1,000 µs | **1 µs** | PASS |
| Unified symbol search (Exact → Prefix → Fuzzy, 20k symbols) | < 5.0 ms | **0.861 ms** | PASS |

## Workspace Architecture

| Crate / App | Responsibility |
| :--- | :--- |
| `crates/klink-common` | Typed IDs (`FileId`, `SymbolId`, `ClusterId`), `StringInterner`, `ScratchArena`, `NodeBitmap` (Roaring), `Confidence`/`EdgeMeta`, `ScanConfig`, typed errors |
| `crates/klink-parse` | Fact-emitting parsers for all 8 Tier-1 languages, `object` PE/ELF/Mach-O binary extractor, Shannon-entropy minification skipper, Aho-Corasick text scanner, `PathTrie`, single-owner `ImportResolver` |
| `crates/klink-graph` | Forward & reverse CSR, iterative Tarjan SCC, condensation DAG, Kahn topo sort, reverse-CSR `ImpactEngine` + LRU Roaring hub cache, Brandes centrality, snapshot diff, zero-copy `mmap` CSR serialization |
| `crates/klink-index` | Struct-of-arrays `SymbolTable`, CSR `XrefIndex` posting lists, `ExactIndex`, `PrefixIndex`, rarest-trigram `FuzzyIndex`, `UnifiedSearch`, `redb` `.klink/symbols.redb` store |
| `crates/klink-layout` | Non-overlapping folder-island circle packing, Louvain clustering, Barnes-Hut quadtree force simulation, stress majorization, Sugiyama layered DAG, LOD summaries, 32-byte `bytemuck::Pod` GPU buffers |
| `crates/klink-core` | Lock-free `ArcSwap` snapshot orchestrator, scan diff engine, raw little-endian binary IPC buffers, `u32` GPU impact bitmask export |
| `crates/klink-cli` | Standalone CLI scanner (`klink-cli`) and 20k-node latency benchmark binary (`klink-bench`) |
| `apps/desktop/src-tauri` | Thin offline Tauri v2 native host with `rfd` OS folder picker and binary IPC commands |
| `apps/desktop/ui` | SolidJS + raw WebGL2 (`#version 300 es`) desktop frontend |

## Install on Another Machine (Prebuilt Release)

### Linux (Debian / Ubuntu / Linux Mint)

Download `klink-desktop_1.0.0_amd64.deb` from the **Releases** page and install:

```bash
sudo dpkg -i klink-desktop_1.0.0_amd64.deb
klink-desktop
```

## Build & Run From Source

### Prerequisites

- **Rust (stable):** Install via [rustup.rs](https://rustup.rs/)
- **Node.js 18+:** Install via [nodejs.org](https://nodejs.org/)
- **Linux (Debian / Ubuntu / Mint) WebKitGTK system libraries:**
  ```bash
  sudo apt update && sudo apt install -y \
    libwebkit2gtk-4.1-dev libsoup-3.0-dev libgtk-3-dev \
    libayatana-appindicator3-dev librsvg2-dev libxdo-dev libssl-dev
  ```
- **Windows:** Install Visual Studio Build Tools (*Desktop development with C++* workload). WebView2 is preinstalled on Windows 11.
- **macOS:** Install Xcode Command Line Tools (`xcode-select --install`).

### 1. Clone, Test & Benchmark

```bash
git clone https://github.com/ri-tiksaini/klink.git
cd klink
cargo test --workspace --release
cargo run --release -p klink-cli --bin klink-bench
```

### 2. Run the CLI Scanner

```bash
# Full repository scan & summary
cargo run --release -p klink-cli -- /path/to/repo

# Blast-radius impact query for a specific file
cargo run --release -p klink-cli -- /path/to/repo --impact src/main.rs

# Unified symbol search (Exact -> Prefix -> Fuzzy) with cross-reference counts
cargo run --release -p klink-cli -- /path/to/repo --search tarjan

# Machine-readable JSON output
cargo run --release -p klink-cli -- /path/to/repo --json
```

### 3. Run the Desktop App (Development Mode)

```bash
cd apps/desktop/ui
npm install
node node_modules/esbuild/install.js   # only needed if npm blocks install scripts
cd ..
./ui/node_modules/.bin/tauri dev       # Linux / macOS
# .\ui\node_modules\.bin\tauri.cmd dev # Windows PowerShell
```

### 4. Build Production Installers

- **Linux (`.deb` + `klink-cli`):**
  ```bash
  cd apps/desktop && ./ui/node_modules/.bin/tauri build --bundles deb
  ```
  Output: `apps/desktop/src-tauri/target/release/bundle/deb/*.deb`
- **Linux (AppImage + `.deb` + `klink-cli`):**
  ```bash
  bash scripts/build-linux.sh
  ```
- **Windows (`.msi` + NSIS `.exe` + `klink-cli.exe` in PowerShell):**
  ```powershell
  .\scripts\build-windows.ps1
  ```

## Controls & Shortcuts

| Action | Shortcut / Input |
| :--- | :--- |
| Pan canvas | Click & drag |
| Zoom (cursor-centered) | Mouse wheel or `+` / `-` |
| Inspect blast radius (`R32UI` glow) | Click any node |
| Jump to folder cluster | Click folder in left rail |
| Search files & symbols | `Ctrl+K` / `Cmd+K` |
| Fit all clusters to screen | `R` or `Fit` button |
| Toggle breathing motion | `Space` or `Breathing` button |
| Clear selection / close search | `Esc` |

## Honest Limitations

Static analysis is fundamentally incomplete—Klink surfaces uncertainty instead of hiding it. Dynamic imports, reflection, macro-expanded symbols, and stripped binaries without DWARF/PDB metadata cannot be resolved statically and are flagged explicitly. See `docs/ACCURACY_MATRIX.md` for per-language details.

## License

Dual-licensed under **MIT** or **Apache-2.0**, at your option.
