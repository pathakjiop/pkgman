use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

pub struct Config {
    /// Enable AUR features: helper search/install, AUR DB load, foreign-pkg detail fetch.
    pub aur: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self { aur: true }
    }
}

static CONFIG: OnceLock<Config> = OnceLock::new();

pub fn cfg() -> &'static Config {
    CONFIG.get_or_init(load)
}

fn load() -> Config {
    let path = config_path();

    // Existing file is the source of truth: manual on/off is always respected.
    if let Some(p) = &path {
        if let Ok(text) = std::fs::read_to_string(p) {
            return parse(&text);
        }
    }

    // First run: seed a config. AUR on only if a helper is actually installed.
    let aur = installed_helper().is_some();
    if let Some(p) = &path {
        let _ = write_default(p, aur);
    }
    Config { aur }
}

fn parse(text: &str) -> Config {
    let mut c = Config::default();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((k, v)) = line.split_once('=') else { continue };
        match k.trim() {
            "aur" => c.aur = parse_bool(v.trim()).unwrap_or(c.aur),
            _ => {}
        }
    }
    c
}

fn write_default(path: &Path, aur: bool) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let body = format!(
        "# pkgman configuration\n\
         # aur: enable AUR helper (yay/paru) features. Set false for pacman-only.\n\
         aur = {}\n",
        aur
    );
    std::fs::write(path, body)
}

fn config_path() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))?;
    Some(base.join("pkgman").join("config.toml"))
}

fn parse_bool(v: &str) -> Option<bool> {
    match v {
        "true" | "1" | "yes" | "on" => Some(true),
        "false" | "0" | "no" | "off" => Some(false),
        _ => None,
    }
}

fn which(cmd: &str) -> bool {
    Command::new("which")
        .arg(cmd)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn installed_helper() -> Option<&'static str> {
    ["paru", "yay"].into_iter().find(|h| which(h))
}

/// AUR helper to use, or None when AUR is disabled or no helper is installed.
pub fn aur_helper() -> Option<&'static str> {
    if !cfg().aur {
        return None;
    }
    installed_helper()
}
