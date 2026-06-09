use std::collections::{HashSet, HashMap};
use std::process::Command;
use std::time::{Instant, Duration};

pub const FILTERS: &[&str] = &["all", "installed", "updates", "core", "extra", "multilib", "aur"];

#[derive(Clone, Debug)]
pub struct Package {
    pub name: String,
    pub version: String,
    pub repo: String,
    pub desc: String,
    pub arch: String,
    pub url: String,
    pub licenses: String,
    pub groups: String,
    pub provides: String,
    pub depends: String,
    pub optdeps: String,
    pub req_by: String,
    pub opt_for: String,
    pub conflicts: String,
    pub replaces: String,
    pub dl_size: String,
    pub inst_size: String,
    pub packager: String,
    pub build_date: String,
    pub installed: bool,
    pub upgradable: bool,
    pub search_key: String,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ConfirmAction {
    Install,
    Remove,
    Update,
}

pub struct App {
    pub pkgs: Vec<Package>,
    pub view: Vec<usize>, // indices into pkgs
    pub selected: HashSet<String>,
    pub cursor: usize,
    pub list_top: usize,
    pub detail_top: usize,
    pub query: String,
    pub filter_idx: usize,
    pub sort_idx: usize,
    pub msg: String,
    pub msg_keep: bool,
    pub msg_expires: Option<Instant>,
    
    pub search_mode: bool,
    pub show_help: bool,
    pub help_scroll: usize,
    pub confirm: Option<(ConfirmAction, Vec<String>)>,
    pub disk_free: String,
    
    pub is_loading: bool,
    pub spinner_tick: u64,
    pub needs_filter: bool,
    pub url_input_mode: bool,
    pub url_query: String,
    pub script_preview: Option<(String, String)>, // (url, content)
    pub console_mode: bool,
    pub console_lines: Vec<String>,
    pub console_scroll: usize,
    pub console_finished: Option<bool>, // Some(success)
    pub sudo_password_mode: bool,
    pub sudo_password: String,
    pub pending_action: Option<(ConfirmAction, Vec<String>)>,
    pub console_in_escape: bool,
    pub current_line: String,
    pub terminal_needs_clear: bool,
    pub last_cursor_change: std::time::Instant,
}

impl App {
    pub fn new() -> Self {
        Self {
            pkgs: Vec::new(),
            view: Vec::new(),
            selected: HashSet::new(),
            cursor: 0,
            list_top: 0,
            detail_top: 0,
            query: String::new(),
            filter_idx: 0,
            sort_idx: 0,
            msg: String::new(),
            msg_keep: false,
            msg_expires: None,
            
            search_mode: false,
            show_help: false,
            help_scroll: 0,
            confirm: None,
            disk_free: get_disk_free(),
            
            is_loading: false,
            spinner_tick: 0,
            needs_filter: false,
            url_input_mode: false,
            url_query: String::new(),
            script_preview: None,
            console_mode: false,
            console_lines: Vec::new(),
            console_scroll: 0,
            console_finished: None,
            sudo_password_mode: false,
            sudo_password: String::new(),
            pending_action: None,
            console_in_escape: false,
            current_line: String::new(),
            terminal_needs_clear: false,
            last_cursor_change: std::time::Instant::now(),
        }
    }

    pub fn set_msg(&mut self, text: &str, secs: u64, keep: bool) {
        self.msg = text.to_string();
        self.msg_keep = keep;
        if keep {
            self.msg_expires = None;
        } else {
            self.msg_expires = Some(Instant::now() + Duration::from_secs(secs));
        }
    }

    pub fn check_msg_expiry(&mut self) {
        if !self.msg_keep {
            if let Some(expiry) = self.msg_expires {
                if Instant::now() > expiry {
                    self.msg.clear();
                    self.msg_expires = None;
                }
            }
        }
    }

    pub fn apply_filter(&mut self) {
        let q = self.query.to_lowercase();
        let mode = FILTERS[self.filter_idx];
        
        self.view.clear();
        for (idx, p) in self.pkgs.iter().enumerate() {
            let matches_query = q.is_empty() || p.search_key.contains(&q);
            let matches_filter = match mode {
                "all" => true,
                "installed" => p.installed,
                "updates" => p.upgradable,
                r => p.repo == r,
            };
            if matches_query && matches_filter {
                self.view.push(idx);
            }
        }
        self.apply_sort(true);
    }

    pub fn apply_sort(&mut self, reset_cursor: bool) {
        let pkgs = &self.pkgs;
        match self.sort_idx {
            0 => { // name
                self.view.sort_by(|&a, &b| {
                    let pa = repo_priority(&pkgs[a].repo);
                    let pb = repo_priority(&pkgs[b].repo);
                    let r = pa.cmp(&pb);
                    if r == std::cmp::Ordering::Equal {
                        pkgs[a].name.to_lowercase().cmp(&pkgs[b].name.to_lowercase())
                    } else {
                        r
                    }
                });
            }
            1 => { // repo
                self.view.sort_by(|&a, &b| {
                    let pa = repo_priority(&pkgs[a].repo);
                    let pb = repo_priority(&pkgs[b].repo);
                    let r = pa.cmp(&pb);
                    if r == std::cmp::Ordering::Equal {
                        pkgs[a].name.to_lowercase().cmp(&pkgs[b].name.to_lowercase())
                    } else {
                        r
                    }
                });
            }
            2 => { // size
                self.view.sort_by(|&a, &b| {
                    let sa = parse_size(&pkgs[a].inst_size);
                    let sb = parse_size(&pkgs[b].inst_size);
                    sb.partial_cmp(&sa).unwrap_or(std::cmp::Ordering::Equal)
                });
            }
            3 => { // installed
                self.view.sort_by(|&a, &b| {
                    let ia = pkgs[a].installed;
                    let ib = pkgs[b].installed;
                    let r = (!ia).cmp(&(!ib)); // installed first
                    if r == std::cmp::Ordering::Equal {
                        pkgs[a].name.to_lowercase().cmp(&pkgs[b].name.to_lowercase())
                    } else {
                        r
                    }
                });
            }
            _ => {}
        }
        if reset_cursor {
            self.cursor = 0;
            self.list_top = 0;
            self.detail_top = 0;
        }
    }

    pub fn cycle_sort(&mut self) {
        self.sort_idx = (self.sort_idx + 1) % 4;
        self.apply_sort(true);
    }

    pub fn write_console_chunk(&mut self, chunk: &str) {
        for c in chunk.chars() {
            if self.console_in_escape {
                if c.is_ascii_alphabetic() {
                    self.console_in_escape = false;
                }
            } else if c == '\x1b' {
                self.console_in_escape = true;
            } else if c == '\n' {
                self.console_lines.push(self.current_line.clone());
                self.current_line.clear();
            } else if c == '\r' {
                self.current_line.clear();
            } else {
                self.current_line.push(c);
            }
        }
        // Limit log lines to last 1000
        if self.console_lines.len() > 1000 {
            self.console_lines.drain(0..self.console_lines.len() - 1000);
        }
        // Auto scroll to bottom
        self.console_scroll = self.console_lines.len();
    }
}

pub fn parse_size(s: &str) -> f64 {
    let parts: Vec<&str> = s.trim().split_whitespace().collect();
    if parts.len() < 2 {
        return 0.0;
    }
    let val_str = parts[0].replace(',', ".");
    let val: f64 = val_str.parse().unwrap_or(0.0);
    let unit = parts[1].to_uppercase();
    match unit.as_str() {
        "KIB" | "KB" => val,
        "MIB" | "MB" => val * 1024.0,
        "GIB" | "GB" => val * 1024.0 * 1024.0,
        _ => val,
    }
}

pub fn get_disk_free() -> String {
    let output = Command::new("df")
        .arg("/")
        .output()
        .ok();
    if let Some(o) = output {
        let out = String::from_utf8_lossy(&o.stdout);
        let lines: Vec<&str> = out.lines().collect();
        if lines.len() > 1 {
            let parts: Vec<&str> = lines[1].split_whitespace().collect();
            if parts.len() > 3 {
                return format!("Disk: {} Free", parts[3]);
            }
        }
    }
    "".to_string()
}

pub fn load_packages_sync() -> Vec<Package> {
    let helper = crate::config::aur_helper();

    let output = Command::new("pacman")
        .arg("-Si")
        .output()
        .ok();
    let raw = output.map(|o| String::from_utf8_lossy(&o.stdout).into_owned()).unwrap_or_default();
    
    let q_out = Command::new("pacman")
        .arg("-Qq")
        .output()
        .ok();
    let installed: HashSet<String> = q_out.map(|o| {
        String::from_utf8_lossy(&o.stdout)
            .lines()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty())
            .collect()
    }).unwrap_or_default();
    
    // Fetch updates via AUR helper if available (includes both pacman and AUR updates)
    let u_out = if let Some(h) = helper {
        Command::new(h).arg("-Qu").output().ok()
    } else {
        Command::new("pacman").arg("-Qu").output().ok()
    };
    let updates: HashSet<String> = u_out.map(|o| {
        String::from_utf8_lossy(&o.stdout)
            .lines()
            .filter_map(|l| {
                l.split_whitespace().next().map(|s| s.to_string())
            })
            .collect()
    }).unwrap_or_default();

    // Query installed foreign/AUR packages
    let qm_out = Command::new("pacman")
        .arg("-Qm")
        .output()
        .ok();
    let foreign: HashMap<String, String> = qm_out.map(|o| {
        String::from_utf8_lossy(&o.stdout)
            .lines()
            .filter_map(|l| {
                let mut parts = l.split_whitespace();
                let name = parts.next()?;
                let version = parts.next()?;
                Some((name.to_string(), version.to_string()))
            })
            .collect()
    }).unwrap_or_default();

    let mut pkgs = Vec::new();
    let mut cur: HashMap<String, String> = HashMap::new();
    
    for line in raw.lines() {
        if line.starts_with("Repository") && !cur.is_empty() {
            if let Some(pkg) = map_pkg(&cur, &installed, &updates) {
                pkgs.push(pkg);
            }
            cur.clear();
        }
        if let Some(pos) = line.find(" : ") {
            let k = &line[..pos];
            let v = &line[pos + 3..];
            cur.insert(k.trim().to_string(), v.trim().to_string());
        }
    }
    if !cur.is_empty() {
        if let Some(pkg) = map_pkg(&cur, &installed, &updates) {
            pkgs.push(pkg);
        }
    }
    
    // Append installed foreign/AUR packages if they are not already in sync list
    let known_names: HashSet<String> = pkgs.iter().map(|p| p.name.clone()).collect();
    for (name, version) in foreign {
        if !known_names.contains(&name) {
            let search_key = format!("{} foreign/aur package", name).to_lowercase();
            let is_upg = updates.contains(&name);
            pkgs.push(Package {
                name: name.clone(),
                version,
                repo: "aur".to_string(),
                desc: "Installed foreign/AUR package".to_string(),
                arch: "x86_64".to_string(),
                url: "None".to_string(),
                licenses: "None".to_string(),
                groups: "None".to_string(),
                provides: "None".to_string(),
                depends: "None".to_string(),
                optdeps: "None".to_string(),
                req_by: "None".to_string(),
                opt_for: "None".to_string(),
                conflicts: "None".to_string(),
                replaces: "None".to_string(),
                dl_size: "None".to_string(),
                inst_size: "None".to_string(),
                packager: "AUR / Foreign".to_string(),
                build_date: "None".to_string(),
                installed: true,
                upgradable: is_upg,
                search_key,
            });
        }
    }
    
    pkgs
}

pub fn map_pkg(
    p: &HashMap<String, String>,
    installed: &HashSet<String>,
    updates: &HashSet<String>,
) -> Option<Package> {
    let name = p.get("Name")?.clone();
    let desc = p.get("Description").cloned().unwrap_or_default();
    let repo = p.get("Repository").cloned().unwrap_or_default();
    let search_key = format!("{} {}", name, desc).to_lowercase();
    let is_inst = installed.contains(&name);
    let is_upg = updates.contains(&name);
    
    Some(Package {
        name,
        version: p.get("Version").cloned().unwrap_or_default(),
        repo,
        desc,
        arch: p.get("Architecture").cloned().unwrap_or_default(),
        url: p.get("URL").cloned().unwrap_or_default(),
        licenses: p.get("Licenses").cloned().unwrap_or_default(),
        groups: p.get("Groups").cloned().unwrap_or_default(),
        provides: p.get("Provides").cloned().unwrap_or_default(),
        depends: p.get("Depends On").cloned().unwrap_or_default(),
        optdeps: p.get("Optional Deps").cloned().unwrap_or_default(),
        req_by: p.get("Required By").cloned().unwrap_or_default(),
        opt_for: p.get("Optional For").cloned().unwrap_or_default(),
        conflicts: p.get("Conflicts With").cloned().unwrap_or_default(),
        replaces: p.get("Replaces").cloned().unwrap_or_default(),
        dl_size: p.get("Download Size").cloned().unwrap_or_default(),
        inst_size: p.get("Installed Size").cloned().unwrap_or_default(),
        packager: p.get("Packager").cloned().unwrap_or_default(),
        build_date: p.get("Build Date").cloned().unwrap_or_default(),
        installed: is_inst,
        upgradable: is_upg,
        search_key,
    })
}

pub fn load_aur_sync() -> Vec<Package> {
    use std::fs::File;
    use std::io::{BufRead, BufReader};
    use std::path::PathBuf;

    let mut pkgs = Vec::new();
    
    // Try to load from completion.cache first for high performance (typically <10ms)
    if let Some(home) = std::env::var_os("HOME") {
        let paths = vec![
            PathBuf::from(&home).join(".cache/paru/completion.cache"),
            PathBuf::from(&home).join(".cache/yay/completion.cache"),
        ];
        
        for path in paths {
            if path.exists() {
                if let Ok(file) = File::open(&path) {
                    let reader = BufReader::new(file);
                    for line in reader.lines() {
                        if let Ok(l) = line {
                            let parts: Vec<&str> = l.split_whitespace().collect();
                            if parts.len() >= 2 && parts[1].to_uppercase() == "AUR" {
                                let name = parts[0].to_string();
                                let search_key = format!("{} aur package", name).to_lowercase();
                                pkgs.push(Package {
                                    name,
                                    version: "unknown".to_string(),
                                    repo: "aur".to_string(),
                                    desc: "AUR Package".to_string(),
                                    arch: "any".to_string(),
                                    url: "None".to_string(),
                                    licenses: "None".to_string(),
                                    groups: "None".to_string(),
                                    provides: "None".to_string(),
                                    depends: "None".to_string(),
                                    optdeps: "None".to_string(),
                                    req_by: "None".to_string(),
                                    opt_for: "None".to_string(),
                                    conflicts: "None".to_string(),
                                    replaces: "None".to_string(),
                                    dl_size: "None".to_string(),
                                    inst_size: "None".to_string(),
                                    packager: "AUR".to_string(),
                                    build_date: "None".to_string(),
                                    installed: false,
                                    upgradable: false,
                                    search_key,
                                });
                            }
                        }
                    }
                    if !pkgs.is_empty() {
                        return pkgs; // Successfully loaded from cache!
                    }
                }
            }
        }
    }

    // Fallback if completion.cache does not exist
    let helper = if which("paru") {
        Some("paru")
    } else if which("yay") {
        Some("yay")
    } else {
        None
    };
    
    let helper = match helper {
        Some(h) => h,
        None => return Vec::new(),
    };
    
    let output = Command::new(helper)
        .args(&["-Sl", "aur"])
        .output()
        .ok();
    
    let out = output.map(|o| String::from_utf8_lossy(&o.stdout).into_owned()).unwrap_or_default();
    
    for line in out.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 {
            let name = parts[1].to_string();
            let version = if parts.len() > 2 { parts[2] } else { "unknown" }.to_string();
            let search_key = format!("{} aur package", name).to_lowercase();
            
            pkgs.push(Package {
                name,
                version,
                repo: "aur".to_string(),
                desc: "AUR Package".to_string(),
                arch: "any".to_string(),
                url: "None".to_string(),
                licenses: "None".to_string(),
                groups: "None".to_string(),
                provides: "None".to_string(),
                depends: "None".to_string(),
                optdeps: "None".to_string(),
                req_by: "None".to_string(),
                opt_for: "None".to_string(),
                conflicts: "None".to_string(),
                replaces: "None".to_string(),
                dl_size: "None".to_string(),
                inst_size: "None".to_string(),
                packager: "AUR".to_string(),
                build_date: "None".to_string(),
                installed: false,
                upgradable: false,
                search_key,
            });
        }
    }
    pkgs
}

fn which(cmd: &str) -> bool {
    Command::new("which")
        .arg(cmd)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

pub fn repo_priority(repo: &str) -> usize {
    match repo.to_lowercase().as_str() {
        "core" => 0,
        "extra" => 1,
        "multilib" => 2,
        "community" => 3,
        "aur" => 10,
        "local" => 11,
        _ => 5, // Custom/other repositories
    }
}
