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

## Installation

### Prerequisites

| Requirement | Notes |
|---|---|
| Arch Linux (or Arch-based distro) | Manjaro, EndeavourOS, etc. work fine |
| Rust toolchain (`cargo`, `rustc`) | Install via [rustup.rs](https://rustup.rs) |
| `pacman` | Ships with Arch — you already have it |
| `yay` or `paru` *(optional)* | Required for AUR operations |

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
| `d` | Open package homepage (xdg-open) |
| `R` | Reload package databases (async) |
| `s` | Cycle sort mode |
| `?` | Toggle help overlay |
| `q` | Quit |

---

## Theming

> 🚧 **Work in progress** — theming support is actively being developed.

A full theming system is planned that will allow full control over colors, pane styles, and status indicator glyphs via the config file. Stay tuned — contributions welcome (see below).

---

## Roadmap

- [ ] **Theming engine** — full color/style customization via `config.toml`
- [ ] **Built-in theme presets** — Catppuccin, Nord, Gruvbox, Tokyo Night
- [ ] **Mouse support** — optional click-to-select
- [ ] **AUR comment viewer** — inline AUR comments and flag status
- [ ] **Dependency tree visualizer** — graphical dep tree in the details pane
- [ ] **AUR PKGBUILD diff viewer** — diff updates before installing

---

<div align="center">

Made with ♥ on Arch Linux  
[⭐ Star this repo](https://github.com/pathakjiop/pkgman) · [🐛 Report a bug](https://github.com/pathakjiop/pkgman/issues/new?template=bug_report.md) · [💡 Request a feature](https://github.com/pathakjiop/pkgman/issues/new?template=feature_request.md)

</div>
