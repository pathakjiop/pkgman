use crate::app::{App, ConfirmAction, FILTERS};
use crate::event::AppEvent;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::io::{self, Write};
use std::process::Command;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::ExecutableCommand;
use tokio::sync::mpsc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub fn is_sudo_cached() -> bool {
    Command::new("sudo")
        .args(&["-n", "true"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

pub fn trigger_action_in_tui(
    password: Option<String>,
    tx: mpsc::UnboundedSender<AppEvent>,
    action: ConfirmAction,
    names: Vec<String>,
) {
    tokio::spawn(async move {
        let _ = tx.send(AppEvent::Message("Initializing...".to_string(), 0, true));

        if let Some(pwd) = password {
            let mut child = match tokio::process::Command::new("sudo")
                .args(&["-S", "-v"])
                .stdin(std::process::Stdio::piped())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::piped())
                .spawn()
            {
                Ok(c) => c,
                Err(e) => {
                    let _ = tx.send(AppEvent::Message(format!("Sudo initialization failed: {}", e), 4, false));
                    let _ = tx.send(AppEvent::ConsoleFinished(false));
                    return;
                }
            };

            if let Some(mut stdin) = child.stdin.take() {
                let _ = stdin.write_all(pwd.as_bytes()).await;
                let _ = stdin.write_all(b"\n").await;
            }

            match child.wait().await {
                Ok(s) if s.success() => {}
                _ => {
                    let _ = tx.send(AppEvent::Message("Sudo authentication failed".to_string(), 4, false));
                    let _ = tx.send(AppEvent::ConsoleFinished(false));
                    return;
                }
            }
        }

        let helper = crate::config::aur_helper().unwrap_or("pacman");

        let name_refs: Vec<&str> = names.iter().map(|s| s.as_str()).collect();
        let mut cmd = match action {
            ConfirmAction::Install => {
                if helper == "pacman" {
                    let mut c = tokio::process::Command::new("sudo");
                    c.args(&["pacman", "--noconfirm", "-S"]);
                    c.args(name_refs);
                    c
                } else {
                    let mut c = tokio::process::Command::new(helper);
                    c.args(&["--noconfirm", "-S"]);
                    c.args(name_refs);
                    c
                }
            }
            ConfirmAction::Remove => {
                let mut c = tokio::process::Command::new("sudo");
                c.args(&["pacman", "--noconfirm", "-Rns"]);
                c.args(name_refs);
                c
            }
            ConfirmAction::Update => {
                if helper == "pacman" {
                    let mut c = tokio::process::Command::new("sudo");
                    c.args(&["pacman", "--noconfirm", "-Syu"]);
                    c
                } else {
                    let mut c = tokio::process::Command::new(helper);
                    c.args(&["--noconfirm", "-Syu"]);
                    c
                }
            }
        };

        cmd.stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        let mut child = match cmd.spawn() {
            Ok(c) => c,
            Err(e) => {
                let _ = tx.send(AppEvent::Message(format!("Failed to spawn command: {}", e), 4, false));
                let _ = tx.send(AppEvent::ConsoleFinished(false));
                return;
            }
        };

        let stdout = child.stdout.take();
        let stderr = child.stderr.take();

        if let Some(mut out) = stdout {
            let tx = tx.clone();
            tokio::spawn(async move {
                let mut buf = [0; 1024];
                while let Ok(n) = out.read(&mut buf).await {
                    if n == 0 { break; }
                    let s = String::from_utf8_lossy(&buf[..n]).into_owned();
                    let _ = tx.send(AppEvent::ConsoleChunk(s));
                }
            });
        }

        if let Some(mut err) = stderr {
            let tx = tx.clone();
            tokio::spawn(async move {
                let mut buf = [0; 1024];
                while let Ok(n) = err.read(&mut buf).await {
                    if n == 0 { break; }
                    let s = String::from_utf8_lossy(&buf[..n]).into_owned();
                    let _ = tx.send(AppEvent::ConsoleChunk(s));
                }
            });
        }

        let status = child.wait().await;
        let success = status.map(|s| s.success()).unwrap_or(false);
        
        let _ = tx.send(AppEvent::ConsoleFinished(success));
        
        if success {
            let msg = match action {
                ConfirmAction::Install => {
                    if names.len() == 1 {
                        format!("Successfully installed {}", names[0])
                    } else {
                        format!("Successfully installed {} packages", names.len())
                    }
                }
                ConfirmAction::Remove => {
                    if names.len() == 1 {
                        format!("Successfully removed {}", names[0])
                    } else {
                        format!("Successfully removed {} packages", names.len())
                    }
                }
                ConfirmAction::Update => "System successfully updated".to_string(),
            };
            let _ = tx.send(AppEvent::Message(msg, 6, false));
        } else {
            let msg = match action {
                ConfirmAction::Install => "Installation failed".to_string(),
                ConfirmAction::Remove => "Removal failed".to_string(),
                ConfirmAction::Update => "System update failed".to_string(),
            };
            let _ = tx.send(AppEvent::Message(msg, 6, false));
        }
    });
}

pub fn trigger_aur_details_fetch(name: String, tx: mpsc::UnboundedSender<AppEvent>) {
    tokio::spawn(async move {
        let Some(helper) = crate::config::aur_helper() else { return };

        let output = tokio::process::Command::new(helper)
            .args(&["-Si", &name])
            .output()
            .await;
            
        if let Ok(out) = output {
            if out.status.success() {
                let stdout_str = String::from_utf8_lossy(&out.stdout);
                let mut cur = std::collections::HashMap::new();
                for line in stdout_str.lines() {
                    if let Some(pos) = line.find(" : ") {
                        let k = &line[..pos];
                        let v = &line[pos + 3..];
                        cur.insert(k.trim().to_string(), v.trim().to_string());
                    }
                }
                if !cur.is_empty() {
                    if let Some(pkg) = crate::app::map_pkg(&cur, &std::collections::HashSet::new(), &std::collections::HashSet::new()) {
                        let _ = tx.send(AppEvent::AurDetailsLoaded(pkg));
                    }
                }
            }
        }
    });
}

pub fn execute_external(cmd_name: &str, args: &[&str]) -> io::Result<()> {
    disable_raw_mode()?;
    io::stdout().execute(LeaveAlternateScreen)?;
    
    println!("\n\x1b[1;36m→ Running: {} {}\x1b[0m\n", cmd_name, args.join(" "));
    
    let mut child = Command::new(cmd_name)
        .args(args)
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .spawn()?;
        
    let _ = child.wait()?;
    
    print!("\n\x1b[1;33mPress Enter to return to pkgtui…\x1b[0m");
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    
    io::stdout().execute(EnterAlternateScreen)?;
    enable_raw_mode()?;
    
    Ok(())
}

pub fn trigger_db_reload(tx: mpsc::UnboundedSender<AppEvent>) {
    tokio::spawn(async move {
        let _ = tx.send(AppEvent::Message("Loading Core DB...".to_string(), 0, true));
        let pkgs = tokio::task::spawn_blocking(crate::app::load_packages_sync).await.unwrap_or_default();
        let _ = tx.send(AppEvent::DbLoaded(pkgs));
        
        // Always send AurLoaded (empty when AUR is off) so the spinner clears.
        let aur = if crate::config::cfg().aur {
            let _ = tx.send(AppEvent::Message("Loading AUR DB...".to_string(), 0, true));
            tokio::task::spawn_blocking(crate::app::load_aur_sync).await.unwrap_or_default()
        } else {
            Vec::new()
        };
        let _ = tx.send(AppEvent::AurLoaded(aur));
    });
}

pub fn trigger_curl_homepage(name: String, url: String, tx: mpsc::UnboundedSender<AppEvent>) {
    tokio::spawn(async move {
        let _ = tx.send(AppEvent::Message(format!("Curling {}...", url), 0, true));
        let output_file = format!("{}_homepage.html", name);
        let res = tokio::process::Command::new("curl")
            .args(&["-sL", "-o", &output_file, &url])
            .status()
            .await;
            
        match res {
            Ok(status) if status.success() => {
                let _ = tx.send(AppEvent::Message(format!("Success: Saved to {}", output_file), 5, false));
            }
            _ => {
                let _ = tx.send(AppEvent::Message(format!("Curl failed for {}", url), 4, false));
            }
        }
    });
}

pub fn trigger_fetch_script(url: String, tx: mpsc::UnboundedSender<AppEvent>) {
    tokio::spawn(async move {
        let _ = tx.send(AppEvent::Message(format!("Downloading {}...", url), 0, true));
        let output = tokio::process::Command::new("curl")
            .args(&["-fsSL", &url])
            .output()
            .await;
            
        match output {
            Ok(out) if out.status.success() => {
                let content = String::from_utf8_lossy(&out.stdout).into_owned();
                let _ = tx.send(AppEvent::ScriptFetched(url, content));
            }
            _ => {
                let _ = tx.send(AppEvent::Message("Error: Failed to fetch script.".to_string(), 4, false));
            }
        }
    });
}

pub fn trigger_aur_search(query: String, tx: mpsc::UnboundedSender<AppEvent>) {
    tokio::spawn(async move {
        let _ = tx.send(AppEvent::Message(format!("Searching AUR for '{}'...", query), 0, true));
        let helper = match crate::config::aur_helper() {
            Some(h) => h,
            None => {
                let msg = if crate::config::cfg().aur {
                    "No AUR helper (yay/paru) found."
                } else {
                    "AUR features disabled in config."
                };
                let _ = tx.send(AppEvent::Message(msg.to_string(), 3, false));
                let _ = tx.send(AppEvent::LoadingDone);
                return;
            }
        };

        let output = tokio::process::Command::new(helper)
            .args(&["-Ss", "--aur", &query])
            .output()
            .await;
            
        match output {
            Ok(out) if out.status.success() => {
                let stdout_str = String::from_utf8_lossy(&out.stdout).into_owned();
                let lines: Vec<&str> = stdout_str.lines().collect();
                let mut pkgs = Vec::new();
                let mut i = 0;
                
                while i < lines.len() {
                    let hdr = lines[i].trim();
                    let desc = if i + 1 < lines.len() { lines[i + 1].trim() } else { "" };
                    i += 2;
                    
                    if hdr.contains('/') {
                        let parts: Vec<&str> = hdr.split_whitespace().collect();
                        if !parts.is_empty() {
                            let rn = parts[0];
                            let repo = "aur".to_string();
                            let name = if let Some(pos) = rn.find('/') {
                                &rn[pos + 1..]
                            } else {
                                rn
                            }.to_string();
                            
                            let version = if parts.len() > 1 { parts[1] } else { "" }.to_string();
                            let search_key = format!("{} {}", name, desc).to_lowercase();
                            
                            pkgs.push(crate::app::Package {
                                name,
                                version,
                                repo,
                                desc: desc.to_string(),
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
                                packager: "AUR".to_string(),
                                build_date: "None".to_string(),
                                installed: false,
                                upgradable: false,
                                search_key,
                            });
                        }
                    }
                }
                let count = pkgs.len();
                let _ = tx.send(AppEvent::AurLoaded(pkgs));
                let _ = tx.send(AppEvent::Message(format!("Found {} AUR packages.", count), 4, false));
            }
            _ => {
                let _ = tx.send(AppEvent::Message("AUR search failed.".to_string(), 3, false));
                let _ = tx.send(AppEvent::LoadingDone);
            }
        }
    });
}

pub fn handle_key(key: KeyEvent, app: &mut App, tx: &mpsc::UnboundedSender<AppEvent>) -> bool {
    let prev_cursor = app.cursor;
    if app.theme_builder_open {
        match key.code {
            KeyCode::Esc | KeyCode::Char('t') | KeyCode::Char('T') | KeyCode::Enter => {
                app.theme_builder_open = false;
                let _ = crate::config::save_config(crate::config::cfg().aur, &app.theme);
            }
            KeyCode::Up | KeyCode::Char('k') => {
                app.theme_builder_cursor = if app.theme_builder_cursor == 0 {
                    10
                } else {
                    app.theme_builder_cursor - 1
                };
            }
            KeyCode::Down | KeyCode::Char('j') => {
                app.theme_builder_cursor = (app.theme_builder_cursor + 1) % 11;
            }
            KeyCode::Left | KeyCode::Char('h') | KeyCode::Right | KeyCode::Char('l') => {
                let is_left = matches!(key.code, KeyCode::Left | KeyCode::Char('h'));
                if app.theme_builder_cursor == 0 {
                    let total_themes = crate::theme::THEMES.len() + 1; // predefined + custom
                    let idx = app.theme_builder_selected_theme_idx;
                    let new_idx = if is_left {
                        if idx == 0 { total_themes - 1 } else { idx - 1 }
                    } else {
                        (idx + 1) % total_themes
                    };
                    app.theme_builder_selected_theme_idx = new_idx;
                    if new_idx < crate::theme::THEMES.len() {
                        app.theme = crate::theme::THEMES[new_idx];
                    } else {
                        app.theme.name = "custom";
                    }
                } else {
                    let current_color = match app.theme_builder_cursor {
                        1 => app.theme.background,
                        2 => app.theme.foreground,
                        3 => app.theme.border,
                        4 => app.theme.highlight_fg,
                        5 => app.theme.highlight_bg,
                        6 => app.theme.accent,
                        7 => app.theme.selected,
                        8 => app.theme.success,
                        9 => app.theme.warning,
                        10 => app.theme.error,
                        _ => unreachable!(),
                    };
                    let colors = crate::theme::COLORS;
                    let pos = colors.iter().position(|&c| c == current_color).unwrap_or(0);
                    let new_pos = if is_left {
                        if pos == 0 { colors.len() - 1 } else { pos - 1 }
                    } else {
                        (pos + 1) % colors.len()
                    };
                    let new_color = colors[new_pos];
                    app.theme.name = "custom";
                    app.theme_builder_selected_theme_idx = crate::theme::THEMES.len();
                    match app.theme_builder_cursor {
                        1 => app.theme.background = new_color,
                        2 => app.theme.foreground = new_color,
                        3 => app.theme.border = new_color,
                        4 => app.theme.highlight_fg = new_color,
                        5 => app.theme.highlight_bg = new_color,
                        6 => app.theme.accent = new_color,
                        7 => app.theme.selected = new_color,
                        8 => app.theme.success = new_color,
                        9 => app.theme.warning = new_color,
                        10 => app.theme.error = new_color,
                        _ => {}
                    }
                }
            }
            _ => {}
        }
        return false;
    }

    if app.sudo_password_mode {
        match key.code {
            KeyCode::Enter => {
                app.sudo_password_mode = false;
                app.console_mode = true;
                app.console_lines.clear();
                app.current_line.clear();
                app.console_scroll = 0;
                app.console_finished = None;
                app.is_loading = true;
                let pwd = Some(app.sudo_password.clone());
                app.sudo_password.clear();
                if let Some((action, names)) = app.pending_action.take() {
                    trigger_action_in_tui(pwd, tx.clone(), action, names);
                }
            }
            KeyCode::Esc => {
                app.sudo_password_mode = false;
                app.sudo_password.clear();
                app.pending_action = None;
            }
            KeyCode::Backspace => {
                app.sudo_password.pop();
            }
            KeyCode::Char(c) => {
                app.sudo_password.push(c);
            }
            _ => {}
        }
        return false;
    }

    if app.console_mode {
        match key.code {
            KeyCode::Esc | KeyCode::Enter if app.console_finished.is_some() => {
                app.console_mode = false;
                app.console_finished = None;
                app.console_lines.clear();
                app.current_line.clear();
            }
            KeyCode::Up | KeyCode::Char('k') => {
                app.console_scroll = app.console_scroll.saturating_sub(1);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                app.console_scroll = app.console_scroll.saturating_add(1);
            }
            KeyCode::PageUp => {
                app.console_scroll = app.console_scroll.saturating_sub(20);
            }
            KeyCode::PageDown => {
                app.console_scroll = app.console_scroll.saturating_add(20);
            }
            _ => {}
        }
        return false;
    }

    if app.url_input_mode {
        match key.code {
            KeyCode::Enter => {
                app.url_input_mode = false;
                let url = app.url_query.trim().to_string();
                if !url.is_empty() {
                    trigger_fetch_script(url, tx.clone());
                }
            }
            KeyCode::Esc => {
                app.url_input_mode = false;
            }
            KeyCode::Backspace => {
                app.url_query.pop();
            }
            KeyCode::Char(c) => {
                app.url_query.push(c);
            }
            _ => {}
        }
        return false;
    }

    if app.script_preview.is_some() {
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
                if let Some((_url, content)) = app.script_preview.take() {
                    let _ = execute_external("bash", &["-c", &content]);
                    app.terminal_needs_clear = true;
                    trigger_db_reload(tx.clone());
                }
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                app.script_preview = None;
            }
            _ => {}
        }
        return false;
    }

    if let Some((action, names)) = app.confirm.take() {
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
                if is_sudo_cached() {
                    app.console_mode = true;
                    app.console_lines.clear();
                    app.current_line.clear();
                    app.console_scroll = 0;
                    app.console_finished = None;
                    app.is_loading = true;
                    trigger_action_in_tui(None, tx.clone(), action, names);
                } else {
                    app.sudo_password_mode = true;
                    app.sudo_password.clear();
                    app.pending_action = Some((action, names));
                }
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                // Action canceled
            }
            _ => {
                app.confirm = Some((action, names));
            }
        }
        return false;
    }

    if app.show_help {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                app.help_scroll = app.help_scroll.saturating_sub(1);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                app.help_scroll = app.help_scroll.saturating_add(1);
            }
            KeyCode::Char('?') | KeyCode::Char('q') | KeyCode::Esc => {
                app.show_help = false;
            }
            _ => {}
        }
        return false;
    }

    if app.search_mode {
        match key.code {
            KeyCode::Enter => {
                app.search_mode = false;
                let q = app.query.trim().to_string();
                if !q.is_empty() && crate::config::cfg().aur {
                    app.is_loading = true;
                    trigger_aur_search(q, tx.clone());
                }
            }
            KeyCode::Esc => {
                app.search_mode = false;
            }
            KeyCode::Backspace => {
                app.query.pop();
                app.needs_filter = true;
            }
            KeyCode::Char('x') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                app.query.clear();
                app.needs_filter = true;
            }
            KeyCode::Char(c) => {
                app.query.push(c);
                app.needs_filter = true;
            }
            _ => {}
        }
        return false;
    }

    let list_height = 20;
    match key.code {
        KeyCode::Up | KeyCode::Char('k') => {
            if app.cursor > 0 {
                app.cursor -= 1;
                app.detail_top = 0;
                if app.cursor < app.list_top {
                    app.list_top = app.cursor;
                }
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if !app.view.is_empty() && app.cursor < app.view.len() - 1 {
                app.cursor += 1;
                app.detail_top = 0;
                if app.cursor >= app.list_top + list_height {
                    app.list_top = app.cursor - list_height + 1;
                }
            }
        }
        KeyCode::PageUp => {
            app.cursor = app.cursor.saturating_sub(list_height);
            app.detail_top = 0;
            if app.cursor < app.list_top {
                app.list_top = app.cursor;
            }
        }
        KeyCode::PageDown => {
            if !app.view.is_empty() {
                app.cursor = (app.cursor + list_height).min(app.view.len() - 1);
                app.detail_top = 0;
                if app.cursor >= app.list_top + list_height {
                    app.list_top = app.cursor - list_height + 1;
                }
            }
        }
        KeyCode::Char('K') => {
            app.detail_top = app.detail_top.saturating_sub(1);
        }
        KeyCode::Char('J') => {
            app.detail_top = app.detail_top.saturating_add(1);
        }
        KeyCode::Char(' ') => {
            if !app.view.is_empty() && app.cursor < app.view.len() {
                let pkg_name = app.pkgs[app.view[app.cursor]].name.clone();
                if app.selected.contains(&pkg_name) {
                    app.selected.remove(&pkg_name);
                } else {
                    app.selected.insert(pkg_name);
                }
            }
        }
        KeyCode::Char('d') => {
            if !app.view.is_empty() && app.cursor < app.view.len() {
                let pkg = &app.pkgs[app.view[app.cursor]];
                if !pkg.url.is_empty() && pkg.url != "None" {
                    trigger_curl_homepage(pkg.name.clone(), pkg.url.clone(), tx.clone());
                } else {
                    app.set_msg("Error: No website URL available.", 3, false);
                }
            }
        }
        KeyCode::Char('E') => {
            app.url_input_mode = true;
            app.url_query.clear();
        }
        KeyCode::Char('i') => {
            if !app.view.is_empty() && app.cursor < app.view.len() {
                let name = app.pkgs[app.view[app.cursor]].name.clone();
                let names = if app.selected.is_empty() {
                    vec![name]
                } else {
                    app.selected.iter().cloned().collect()
                };
                app.confirm = Some((ConfirmAction::Install, names));
            }
        }
        KeyCode::Char('r') => {
            if !app.view.is_empty() && app.cursor < app.view.len() {
                let name = app.pkgs[app.view[app.cursor]].name.clone();
                let names = if app.selected.is_empty() {
                    vec![name]
                } else {
                    app.selected.iter().cloned().collect()
                };
                app.confirm = Some((ConfirmAction::Remove, names));
            }
        }
        KeyCode::Char('u') => {
            app.confirm = Some((ConfirmAction::Update, vec![]));
        }
        KeyCode::Char('R') => {
            trigger_db_reload(tx.clone());
        }
        KeyCode::Char('/') => {
            app.search_mode = true;
        }
        KeyCode::Char(c) if c.is_digit(10) => {
            let idx = (c.to_digit(10).unwrap() as usize).saturating_sub(1);
            if idx < FILTERS.len() {
                app.filter_idx = idx;
                app.needs_filter = true;
            }
        }
        KeyCode::Char('s') => {
            app.cycle_sort();
        }
        KeyCode::Char('t') | KeyCode::Char('T') => {
            app.theme_builder_open = true;
            app.theme_builder_cursor = 0;
        }
        KeyCode::Char('?') => {
            app.show_help = true;
            app.help_scroll = 0;
        }
        KeyCode::Char('q') | KeyCode::Char('Q') => {
            return true;
        }
        _ => {}
    }
    
    if app.cursor != prev_cursor {
        app.last_cursor_change = std::time::Instant::now();
    }
    
    false
}
