<div align="center">

```
██████╗ ██╗  ██╗ ██████╗ ███╗   ███╗ █████╗ ███╗   ██╗
██╔══██╗██║ ██╔╝██╔════╝ ████╗ ████║██╔══██╗████╗  ██║
██████╔╝█████╔╝ ██║  ███╗██╔████╔██║███████║██╔██╗ ██║
██╔═══╝ ██╔═██╗ ██║   ██║██║╚██╔╝██║██╔══██║██║╚██╗██║
██║     ██║  ██╗╚██████╔╝██║ ╚═╝ ██║██║  ██║██║ ╚████║
╚═╝     ╚═╝  ╚═╝ ╚═════╝ ╚═╝     ╚═╝╚═╝  ╚═╝╚═╝  ╚═══╝
```

**A high-performance, keyboard-driven TUI package manager for Arch Linux**  
Built with Rust · Powered by Ratatui · AUR-aware

[![Rust](https://img.shields.io/badge/built_with-Rust-ce422b?style=flat-square&logo=rust)](https://www.rust-lang.org/)
[![Arch Linux](https://img.shields.io/badge/platform-Arch_Linux-1793d1?style=flat-square&logo=archlinux&logoColor=white)](https://archlinux.org/)
[![License: MIT](https://img.shields.io/badge/license-MIT-22c55e?style=flat-square)](LICENSE)
[![Stars](https://img.shields.io/github/stars/pathakjiop/pkgman?style=flat-square&color=facc15)](https://github.com/pathakjiop/pkgman/stargazers)
[![PRs Welcome](https://img.shields.io/badge/PRs-welcome-a78bfa?style=flat-square)](CONTRIBUTING.md)

<br/>

![pkgman preview](assets/preview.png)

</div>

---

## The problem with `pacman`

`pacman` and AUR helpers like `yay`/`paru` are powerful — but managing packages through them is fragmented, repetitive, and blind to context. You run one command to search, another to inspect, another to install. You lose your place. You repeat yourself.

`pkgman` collapses all of that into a single, persistent TUI: live search, full metadata at a glance, batch operations, and real-time install logs — all without leaving the terminal.

---

## Features

### ⚡ Performance-first architecture

- **< 50ms startup** — installed packages load synchronously; repos and AUR stream in the background
- **< 10ms AUR listing** — reads `yay`/`paru` shell completion caches directly, no subprocess overhead
- **~360,000 lines of pacman DB** parsed in milliseconds
- Zero-bloat binary via Rust + Ratatui — negligible memory footprint

### 🖥️ Interface

- **Split-pane layout** — package list · full metadata · maintainer/context info, all visible at once
- **Live header indicators** — `Installed: ✔  Repos: ⠋  AUR: ◌` tracks background load progress in real-time
- **7 filter tabs** — `All`, `Installed`, `Updates`, `Core`, `Extra`, `Multilib`, `AUR` (keys `1`–`7`)
- **Dynamic sort** — cycle by Name, Repository, Download Size, or Install status

### 📦 Package operations

- **Batch install/remove** — mark multiple packages with `Space`, act on all at once
- **In-TUI sudo prompts & install logs** — full console overlay with auto DB refresh on completion
- **System upgrade** — `pacman -Syu` directly from the TUI
- **Async DB reload** — refresh databases without quitting

### 🔍 Search & discovery

- **Incremental live search** — results filter as you type, no enter required
- **Lazy AUR detail fetching** — descriptions, deps, and licenses load in the background with 300ms debounce
- **Repository priority sorting** — `core > extra > multilib > others > aur > local`; AUR never floods the main view

### 🛡️ Safety

- **Script inspection** — securely fetch and preview `curl | bash` scripts before executing
- **Homepage cURL** — async-fetch the official homepage of any selected package

---

## Installation

### Prerequisites

| Requirement | Notes |
|---|---|
| Arch Linux (or Arch-based distro) | Manjaro, EndeavourOS, etc. work fine |
| Rust toolchain (`cargo`, `rustc`) | Install via [rustup.rs](https://rustup.rs) |
| `pacman` | Ships with Arch — you already have it |
| `yay` or `paru` *(optional)* | Required for AUR operations |
| `curl` *(optional)* | Required for homepage fetch and script preview |

### Build from source

```bash
# Clone the repository
git clone https://github.com/pathakjiop/pkgman.git
cd pkgman

# Build an optimized release binary
cargo build --release

# Install to your local bin
mkdir -p ~/.local/bin
cp target/release/pkgman ~/.local/bin/pkgman
chmod +x ~/.local/bin/pkgman
```

> **Ensure `~/.local/bin` is in your `$PATH`** — add this to your `~/.bashrc` or `~/.zshrc`:
> ```bash
> export PATH="$HOME/.local/bin:$PATH"
> ```

### Optimized release profile *(optional but recommended)*

Add to your `Cargo.toml` for maximum runtime performance:

```toml
[profile.release]
opt-level = 3
lto = true
codegen-units = 1
panic = "abort"
strip = true
```

---

## Configuration

`pkgman` reads its config from `$XDG_CONFIG_HOME/pkgman/config.toml` (defaults to `~/.config/pkgman/config.toml`).

The file is **created automatically on first run** — `aur` is seeded to `true` if `yay` or `paru` is found, `false` otherwise. Once it exists, it is **never overwritten**. Manual edits are always respected.

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `aur` | bool | `true` | When `false`: AUR DB skipped, searches return a disabled notice, installs/updates route through `pacman` only |

**Example — pacman-only mode:**
```toml
# ~/.config/pkgman/config.toml
aur = false
```

---

## AUR cache warm-up

`pkgman` loads 114,000+ AUR packages in under 10ms by reading your AUR helper's shell completion cache directly:

| Helper | Cache path |
|--------|-----------|
| `yay` | `~/.cache/yay/completion.cache` |
| `paru` | `~/.cache/paru/completion.cache` |

If AUR packages aren't appearing, warm up the cache first:

```bash
# yay
yay -Sl aur > /dev/null

# paru
paru -Sl aur > /dev/null
```

---

## Keyboard reference

> Press `?` inside pkgman at any time to open the interactive help overlay.

| Key | Action |
|-----|--------|
| `↑` / `k` | Move cursor up |
| `↓` / `j` | Move cursor down |
| `PgUp` / `PgDn` | Scroll full page |
| `J` / `K` | Scroll details pane |
| `1` – `7` | Switch filter tabs |
| `/` | Enter live search mode |
| `Ctrl+X` | Clear search query |
| `Esc` | Exit search / close overlay |
| `Space` | Toggle package selection |
| `i` | Install selected package(s) |
| `r` | Remove selected package(s) |
| `u` | System upgrade (`-Syu`) |
| `d` | Fetch package homepage (async) |
| `E` | Preview shell script safely |
| `R` | Reload package databases (async) |
| `s` | Cycle sort mode |
| `?` | Toggle help overlay |
| `q` | Quit |

---

## Theming

> 🚧 **Work in progress** — theming support is actively being developed.

A full theming system is planned that will allow full control over colors, pane styles, and status indicator glyphs via the config file. Stay tuned — contributions welcome (see below).

---

## Contributing

Contributions are very welcome. To keep things clean and mergeable, please follow these conventions.

### Branch naming

All work branches must follow this pattern:

```
<type>/<short-description>
```

| Type | When to use |
|------|-------------|
| `feat/` | New feature or capability |
| `fix/` | Bug fix |
| `theme/` | Theming-related work |
| `docs/` | Documentation only |
| `refactor/` | Code restructure, no behavior change |
| `perf/` | Performance improvement |
| `chore/` | Tooling, CI, dependencies |
| `test/` | Tests only |

**Examples:**
```
feat/theme-engine
fix/aur-cache-fallback
docs/contributing-guide
refactor/event-loop-cleanup
theme/catppuccin-mocha
```

> ⚠️ PRs from branches that don't match this pattern will be asked to rename before review.

### Workflow

```bash
# 1. Fork the repo and clone your fork
git clone https://github.com/pathakjiop/pkgman.git
cd pkgman

# 2. Create your branch from main
git checkout -b feat/your-feature-name

# 3. Make your changes, commit with a clear message
git commit -m "feat: add catppuccin theme support"

# 4. Push and open a PR against main
git push origin feat/your-feature-name
```

### Commit message format

Follow the conventional commit style:

```
<type>: <short imperative description>

Optional longer body explaining the why, not the what.
```

### PR checklist

Before opening a PR, make sure:

- [ ] `cargo build --release` succeeds with no warnings
- [ ] Your branch name follows the naming convention above
- [ ] Changes are scoped — one concern per PR
- [ ] You've updated documentation if behavior changed
- [ ] For new features, a brief description is in the PR body

### Good first issues

Look for issues tagged [`good first issue`](https://github.com/pathakjiop/pkgman/issues?q=label%3A"good+first+issue") — these are intentionally scoped for newcomers.

---

## Roadmap

- [ ] **Theming engine** — full color/style customization via `config.toml`
- [ ] **Built-in theme presets** — Catppuccin, Nord, Gruvbox, Tokyo Night
- [ ] **Mouse support** — optional click-to-select
- [ ] **AUR comment viewer** — inline AUR comments and flag status
- [ ] **Dependency tree visualizer** — graphical dep tree in the details pane
- [ ] **AUR PKGBUILD diff viewer** — diff updates before installing

---

## License

MIT — see [LICENSE](LICENSE).

---

<div align="center">

Made with ♥ on Arch Linux  
[⭐ Star this repo](https://github.com/pathakjiop/pkgman) · [🐛 Report a bug](https://github.com/pathakjiop/pkgman/issues/new?template=bug_report.md) · [💡 Request a feature](https://github.com/pathakjiop/pkgman/issues/new?template=feature_request.md)

</div>
