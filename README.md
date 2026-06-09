# 📦 pkgman

> A High-Performance, Keyboard-Driven Package Manager Terminal User Interface (TUI) for Arch Linux.

`pkgman` is a terminal wrapper that makes searching, selecting, installing, removing, and managing packages from official Arch Linux repositories and the AUR (via `yay`/`paru`) visually intuitive, lightning-fast, and distraction-free.

---

## 📸 Preview

![pkgman-ultra Interface](assets/preview.png)

---

## ❓ Why Is It Needed?

While Arch Linux's `pacman` and AUR helpers like `yay` or `paru` are extremely powerful CLI tools, they come with certain friction points when managing packages:

1. **Information Fragmentation:** Checking dependencies, licenses, download sizes, and descriptions of multiple packages requires running multiple commands. `pkgman` puts all details in a clean, split-pane layout at a single glance.
2. **Interactive Live Search:** Typing CLI search terms repeatedly can be slow. `pkgman` features dynamic, real-time filtering as you type, letting you narrow down results instantly.
3. **Batch Actions:** Instead of installing packages one by one, you can scroll through the list, mark multiple packages with `[Space]`, and batch-install or remove them.
4. **Safety & Script Inspection:** When installing packages or viewing source code, you sometimes want to verify script files or curl homepage contents. `pkgman` includes built-in asynchronous cURL retrieval and safe custom script previews with validation overlays.
5. **In-TUI Package Operations:** All password prompt entries (`sudo`) and installation logs are handled directly inside the TUI in a real-time console overlay with automatic database refreshes upon transaction completions.
6. **Blazing Fast & Lightweight:** Written in **Rust** using the high-performance **Ratatui** library, providing instantaneous startup times, extremely fast database parsing (~360,000 lines of pacman DB parsed in milliseconds), and a negligible memory footprint.

---

## 🚀 Key Features

- **Split-Pane Dashboard:** Package listing on the left, full metadata details in the middle, and extra context/maintainer info on the right.
- **Incremental Multi-stage Loading:** Boots up in under 50ms by loading installed packages synchronously, streaming official repositories in the background, and finally loading the AUR completion cache.
- **Side-by-side Load Status Dashboard:** Shows live status indicators in the header (`Installed: ✔  Repos: ⠋  AUR: ◌`) so you can visually track load progress in real-time.
- **Repository Priority Sorting:** Orders packages by repository priority (`core` > `extra` > `multilib` > others > `aur` > `local`) first, then alphabetically by name. Official packages take priority, and AUR packages do not flood the main view.
- **AUR Support:** Integrates with `yay`/`paru` to search and install AUR packages asynchronously.
- **Instant AUR Loading (<10ms):** Automatically reads and parses local shell completion caches for the AUR to instantly populate all 114,000+ packages on startup.
- **Lazy Details Fetching:** AUR package details (dependencies, licenses, descriptions) are lazily fetched in the background with a 300ms scrolling debounce.
- **Multi-Tab Filtering:** Effortlessly switch tabs (All, Installed, Updates, Core, Extra, Multilib, AUR) with keys `1`-`7`.
- **Dynamic Sorting:** Cycle sort options by Name, Repository, Download Size, and Installed status (using the `s` key).

---

## 📋 Prerequisites

To run `pkgman`, make sure you have:
- **Arch Linux** (or any Arch-based distribution)
- **Rust Toolchain** (`cargo`, `rustc` - to compile from source)
- **pacman** (standard package manager)
- **yay** or **paru** (optional, for AUR query & install capability)
- **curl** (optional, for website downloads and custom scripts)

---

## ⚙️ Compilation & Installation

### ⚡ Optimization Flags (Fastest Runtime)

To get the absolute best performance out of `pkgman`, you can configure Cargo to compile with link-time optimization (LTO) and aggressive optimizations. Add the following to your `Cargo.toml` if you wish to tune the binary size and speed:

```toml
[profile.release]
opt-level = 3
lto = true
codegen-units = 1
panic = "abort"
strip = true
```

### 🛠️ Compilation Commands

```bash
# 1. Clone or navigate to the workspace directory
cd /path/to/PackageTUI

# 2. Build the optimized release binary
cargo build --release

# 3. Create the local binary folder if it doesn't exist
mkdir -p ~/.local/bin

# 4. Copy the compiled binary to your path
cp target/release/pkgman ~/.local/bin/pkgman

# 5. Make sure the binary is executable
chmod +x ~/.local/bin/pkgman
```

> [!TIP]
> Ensure `~/.local/bin` is in your environment's `$PATH` variable by adding this line to your shell configuration (`~/.bashrc` or `~/.zshrc`):
> `export PATH="$HOME/.local/bin:$PATH"`

---

## 🎛️ Hyprland Shortcut Setup

When running Hyprland, environment variables like `$PATH` might not be loaded in the same way they are in your interactive shell session. Therefore, using the **absolute path** to the `pkgman` executable is highly recommended.

Add one of the following keybindings to your Hyprland configuration (typically at `~/.config/hypr/hyprland.conf`):

### Using Kitty (Recommended)
```ini
# Open pkgman in kitty using Super + P
bind = SUPER, P, exec, kitty sh -c "$HOME/.local/bin/pkgman"
```

### Using Alacritty
```ini
# Open pkgman in alacritty using Super + P
bind = SUPER, P, exec, alacritty -e sh -c "$HOME/.local/bin/pkgman"
```

### Using Foot
```ini
# Open pkgman in foot using Super + P
bind = SUPER, P, exec, foot sh -c "$HOME/.local/bin/pkgman"
```

After modifying the configuration file, reload Hyprland to apply the changes immediately:
```bash
hyprctl reload
```

---

## ⚡ AUR Completion Cache Synchronization

To achieve sub-10ms listing of the 114,000+ packages in the AUR on startup, `pkgman` reads completion cache files generated by your AUR helper. 

- **Yay Cache Location:** `~/.cache/yay/completion.cache`
- **Paru Cache Location:** `~/.cache/paru/completion.cache`

If the TUI does not list AUR packages or shows a message indicating a fallback load, warm up your AUR helper's cache by running a simple operation or triggering a search:
```bash
# For yay users
yay -Sl aur > /dev/null

# For paru users
paru -Sl aur > /dev/null
```
Once the file is generated, `pkgman` will load it instantly on subsequent launches.

---

## ⌨️ Keyboard Reference

Press `?` inside `pkgman` at any time to pull up the interactive keyboard shortcut reference overlay:

| Key | Action |
|---|---|
| **`↑` / `k`** | Move cursor up in package list |
| **`↓` / `j`** | Move cursor down in package list |
| **`PgUp` / `PgDn`** | Scroll full page in package list |
| **`J` / `K`** | Scroll details pane (for packages with long descriptions/dependencies) |
| **`1` - `7`** | Switch direct filter tabs (All, Installed, Updates, etc.) |
| **`/`** | Activate Live Search mode (Ctrl+X clears query) |
| **`Esc`** | Exit search mode / Cancel overlay / Close logs |
| **`Space`** | Select/deselect multiple packages for batch actions |
| **`i`** | Install selected package(s) inside TUI |
| **`r`** | Uninstall selected package(s) inside TUI |
| **`u`** | Run system upgrade (`-Syu`) inside TUI |
| **`d`** | Asynchronously download/cURL the official homepage of the selected package |
| **`E` (Shift+E)** | Securely fetch and preview custom shell scripts (`curl | bash`) |
| **`R` (Shift+R)** | Asynchronously reload package databases |
| **`s`** | Cycle sorting mode (Name, Repo, Size, Installed status) |
| **`?`** | Toggle Help Overlay |
| **`q`** | Quit application |

