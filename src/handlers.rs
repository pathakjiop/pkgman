use crate::app::{App, ConfirmAction, FILTERS};
use crate::event::AppEvent;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::process::Command;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::mpsc;

pub fn is_sudo_cached() -> bool {
	Command::new("sudo")
		.args(["-n", "true"])
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
				.args(["-S", "-v"])
				.stdin(std::process::Stdio::piped())
				.stdout(std::process::Stdio::null())
				.stderr(std::process::Stdio::piped())
				.spawn()
			{
				Ok(c) => c,
				Err(e) => {
					let _ = tx.send(AppEvent::Message(
						format!("Sudo initialization failed: {}", e),
						4,
						false,
					));
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
					let _ = tx.send(AppEvent::Message(
						"Sudo authentication failed".to_string(),
						4,
						false,
					));
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
					c.args(["pacman", "--noconfirm", "-S"]);
					c.args(name_refs);
					c
				} else {
					let mut c = tokio::process::Command::new(helper);
					c.args(["--noconfirm", "-S"]);
					c.args(name_refs);
					c
				}
			}
			ConfirmAction::Remove => {
				let mut c = tokio::process::Command::new("sudo");
				c.args(["pacman", "--noconfirm", "-Rns"]);
				c.args(name_refs);
				c
			}
			ConfirmAction::Update => {
				if helper == "pacman" {
					let mut c = tokio::process::Command::new("sudo");
					c.args(["pacman", "--noconfirm", "-Syu"]);
					c
				} else {
					let mut c = tokio::process::Command::new(helper);
					c.args(["--noconfirm", "-Syu"]);
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
				let _ = tx.send(AppEvent::Message(
					format!("Failed to spawn command: {}", e),
					4,
					false,
				));
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
					if n == 0 {
						break;
					}
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
					if n == 0 {
						break;
					}
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
						format!(
							"Successfully installed {} packages",
							names.len()
						)
					}
				}
				ConfirmAction::Remove => {
					if names.len() == 1 {
						format!("Successfully removed {}", names[0])
					} else {
						format!(
							"Successfully removed {} packages",
							names.len()
						)
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
		let Some(helper) = crate::config::aur_helper() else {
			return;
		};

		let output = tokio::process::Command::new(helper)
			.args(["-Si", &name])
			.output()
			.await;

		if let Ok(out) = output
			&& out.status.success()
		{
			let stdout_str = String::from_utf8_lossy(&out.stdout);
			let mut cur = std::collections::HashMap::new();
			for line in stdout_str.lines() {
				if let Some(pos) = line.find(" : ") {
					let k = &line[..pos];
					let v = &line[pos + 3..];
					cur.insert(k.trim().to_string(), v.trim().to_string());
				}
			}
			if !cur.is_empty()
				&& let Some(pkg) = crate::app::map_pkg(
					&cur,
					&std::collections::HashSet::new(),
					&std::collections::HashSet::new(),
				) {
				let _ = tx.send(AppEvent::AurDetailsLoaded(Box::new(pkg)));
			}
		}
	});
}

pub fn trigger_db_reload(tx: mpsc::UnboundedSender<AppEvent>) {
	tokio::spawn(async move {
		let _ = tx.send(AppEvent::Message("Loading Core DB...".to_string(), 0, true));
		let pkgs = tokio::task::spawn_blocking(crate::app::load_packages_sync)
			.await
			.unwrap_or_default();
		let _ = tx.send(AppEvent::DbLoaded(pkgs));

		// Always send AurLoaded (empty when AUR is off) so the spinner clears.
		let aur = if crate::config::cfg().aur {
			let _ = tx.send(AppEvent::Message(
				"Loading AUR DB...".to_string(),
				0,
				true,
			));
			tokio::task::spawn_blocking(crate::app::load_aur_sync)
				.await
				.unwrap_or_default()
		} else {
			Vec::new()
		};
		let _ = tx.send(AppEvent::AurLoaded(aur));
	});
}

pub fn trigger_open_homepage(url: String, tx: mpsc::UnboundedSender<AppEvent>) {
	tokio::spawn(async move {
		let res = tokio::process::Command::new("xdg-open")
			.arg(&url)
			.stdout(std::process::Stdio::null())
			.stderr(std::process::Stdio::null())
			.status()
			.await;

		match res {
			Ok(status) if status.success() => {
				let _ = tx.send(AppEvent::Message(
					format!("Opened {}", url),
					5,
					false,
				));
			}
			_ => {
				let _ = tx.send(AppEvent::Message(
					format!("xdg-open failed for {}", url),
					4,
					false,
				));
			}
		}
	});
}

pub fn trigger_dep_tree_fetch(name: String, installed: bool, tx: mpsc::UnboundedSender<AppEvent>) {
	tokio::spawn(async move {
		let mut cmd = tokio::process::Command::new("pactree");
		cmd.arg("-a");
		if !installed {
			cmd.arg("-s");
		}
		cmd.arg(&name);
		let output = cmd.output().await;
		match output {
			Ok(out) if out.status.success() => {
				let tree = String::from_utf8_lossy(&out.stdout)
					.lines()
					.map(|s| s.to_string())
					.collect::<Vec<String>>();
				let _ = tx.send(AppEvent::DepTreeLoaded(name, Ok(tree)));
			}
			Ok(out) => {
				let err_msg =
					String::from_utf8_lossy(&out.stderr).trim().to_string();
				let _ = tx.send(AppEvent::DepTreeLoaded(name, Err(err_msg)));
			}
			Err(e) => {
				let _ = tx.send(AppEvent::DepTreeLoaded(name, Err(e.to_string())));
			}
		}
	});
}

pub fn trigger_aur_search(query: String, tx: mpsc::UnboundedSender<AppEvent>) {
	tokio::spawn(async move {
		let _ = tx.send(AppEvent::Message(
			format!("Searching AUR for '{}'...", query),
			0,
			true,
		));
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
			.args(["-Ss", "--aur", &query])
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
					let desc = if i + 1 < lines.len() {
						lines[i + 1].trim()
					} else {
						""
					};
					i += 2;

					if hdr.contains('/') {
						let parts: Vec<&str> =
							hdr.split_whitespace().collect();
						if !parts.is_empty() {
							let rn = parts[0];
							let repo = "aur".to_string();
							let name =
								if let Some(pos) = rn.find('/') {
									&rn[pos + 1..]
								} else {
									rn
								}
								.to_string();

							let version = if parts.len() > 1 {
								parts[1]
							} else {
								""
							}
							.to_string();
							let search_key =
								format!("{} {}", name, desc)
									.to_lowercase();

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
				let _ = tx.send(AppEvent::Message(
					format!("Found {} AUR packages.", count),
					4,
					false,
				));
			}
			_ => {
				let _ = tx.send(AppEvent::Message(
					"AUR search failed.".to_string(),
					3,
					false,
				));
				let _ = tx.send(AppEvent::LoadingDone);
			}
		}
	});
}

pub fn handle_key(key: KeyEvent, app: &mut App, tx: &mpsc::UnboundedSender<AppEvent>) -> bool {
	let prev_cursor = app.cursor;

	if app.show_wiki {
		match key.code {
			KeyCode::Esc | KeyCode::Enter | KeyCode::Char('q') | KeyCode::Char('Q') => {
				app.show_wiki = false;
				app.wiki_content.clear();
				app.wiki_err_msg = None;
			}
			KeyCode::Up | KeyCode::Char('k') => {
				app.wiki_scroll = app.wiki_scroll.saturating_sub(1);
			}
			KeyCode::Down | KeyCode::Char('j') => {
				app.wiki_scroll = app.wiki_scroll.saturating_add(1);
			}
			KeyCode::PageUp => {
				app.wiki_scroll = app.wiki_scroll.saturating_sub(20);
			}
			KeyCode::PageDown => {
				app.wiki_scroll = app.wiki_scroll.saturating_add(20);
			}
			KeyCode::Char('w') => {
				let url = format!(
					"https://wiki.archlinux.org/index.php?search={}",
					app.wiki_pkg_name
				);
				trigger_open_homepage(url, tx.clone());
			}
			_ => {}
		}
		return false;
	}

	if app.theme_builder_open {
		match key.code {
			KeyCode::Esc | KeyCode::Char('t') | KeyCode::Char('T') | KeyCode::Enter => {
				app.theme_builder_open = false;
				let _ = crate::config::save_config(
					crate::config::cfg().aur,
					&app.theme,
				);
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
			KeyCode::Left
			| KeyCode::Char('h')
			| KeyCode::Right
			| KeyCode::Char('l') => {
				let is_left =
					matches!(key.code, KeyCode::Left | KeyCode::Char('h'));
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
					let pos = colors
						.iter()
						.position(|&c| c == current_color)
						.unwrap_or(0);
					let new_pos = if is_left {
						if pos == 0 { colors.len() - 1 } else { pos - 1 }
					} else {
						(pos + 1) % colors.len()
					};
					let new_color = colors[new_pos];
					app.theme.name = "custom";
					app.theme_builder_selected_theme_idx =
						crate::theme::THEMES.len();
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
				if key.modifiers.contains(KeyModifiers::CONTROL) {
					delete_last_word(&mut app.sudo_password);
				} else {
					app.sudo_password.pop();
				}
			}
			KeyCode::Char('w') if key.modifiers.contains(KeyModifiers::CONTROL) => {
				delete_last_word(&mut app.sudo_password);
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
				if key.modifiers.contains(KeyModifiers::CONTROL) {
					delete_last_word(&mut app.query);
				} else {
					app.query.pop();
				}
				app.needs_filter = true;
			}
			KeyCode::Char('w') if key.modifiers.contains(KeyModifiers::CONTROL) => {
				delete_last_word(&mut app.query);
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
		// Scroll details / dependency tree when Ctrl is held
		KeyCode::Char('k') if key.modifiers.contains(KeyModifiers::CONTROL) => {
			app.detail_top = app.detail_top.saturating_sub(1);
		}
		KeyCode::Char('j') if key.modifiers.contains(KeyModifiers::CONTROL) => {
			app.detail_top = app.detail_top.saturating_add(1);
		}
		KeyCode::Up if key.modifiers.contains(KeyModifiers::CONTROL) => {
			app.detail_top = app.detail_top.saturating_sub(1);
		}
		KeyCode::Down if key.modifiers.contains(KeyModifiers::CONTROL) => {
			app.detail_top = app.detail_top.saturating_add(1);
		}
		KeyCode::PageUp if key.modifiers.contains(KeyModifiers::CONTROL) => {
			app.detail_top = app.detail_top.saturating_sub(20);
		}
		KeyCode::PageDown if key.modifiers.contains(KeyModifiers::CONTROL) => {
			app.detail_top = app.detail_top.saturating_add(20);
		}

		KeyCode::Up | KeyCode::Char('k') if app.cursor > 0 => {
			app.cursor -= 1;
			app.detail_top = 0;
			if app.cursor < app.list_top {
				app.list_top = app.cursor;
			}
		}
		KeyCode::Down | KeyCode::Char('j')
			if !app.view.is_empty() && app.cursor < app.view.len() - 1 =>
		{
			app.cursor += 1;
			app.detail_top = 0;
			if app.cursor >= app.list_top + list_height {
				app.list_top = app.cursor - list_height + 1;
			}
		}
		KeyCode::PageUp => {
			app.cursor = app.cursor.saturating_sub(list_height);
			app.detail_top = 0;
			if app.cursor < app.list_top {
				app.list_top = app.cursor;
			}
		}
		KeyCode::PageDown if !app.view.is_empty() => {
			app.cursor = (app.cursor + list_height).min(app.view.len() - 1);
			app.detail_top = 0;
			if app.cursor >= app.list_top + list_height {
				app.list_top = app.cursor - list_height + 1;
			}
		}
		KeyCode::Home => {
			app.cursor = 0;
			app.detail_top = 0;
			app.list_top = 0;
		}
		KeyCode::End if !app.view.is_empty() => {
			app.cursor = app.view.len() - 1;
			app.detail_top = 0;
			if app.cursor >= list_height {
				app.list_top = app.cursor - list_height + 1;
			}
		}
		KeyCode::Char('K') => {
			app.detail_top = app.detail_top.saturating_sub(1);
		}
		KeyCode::Char('J') => {
			app.detail_top = app.detail_top.saturating_add(1);
		}
		KeyCode::Char(' ') if !app.view.is_empty() && app.cursor < app.view.len() => {
			let pkg_name = app.pkgs[app.view[app.cursor]].name.clone();
			if app.selected.contains(&pkg_name) {
				app.selected.remove(&pkg_name);
			} else {
				app.selected.insert(pkg_name);
			}
		}
		KeyCode::Char('d') if !app.view.is_empty() && app.cursor < app.view.len() => {
			let pkg = &app.pkgs[app.view[app.cursor]];
			if !pkg.url.is_empty() && pkg.url != "None" {
				trigger_open_homepage(pkg.url.clone(), tx.clone());
			} else {
				app.set_msg("Error: No website URL available.", 3, false);
			}
		}
		KeyCode::Char('w') if !app.view.is_empty() && app.cursor < app.view.len() => {
			let pkg_name = app.pkgs[app.view[app.cursor]].name.clone();
			let url =
				format!("https://wiki.archlinux.org/index.php?search={}", pkg_name);
			trigger_open_homepage(url, tx.clone());
		}
		KeyCode::Char('W') if !app.view.is_empty() && app.cursor < app.view.len() => {
			let pkg_name = app.pkgs[app.view[app.cursor]].name.clone();
			app.show_wiki = true;
			app.wiki_loading = true;
			app.wiki_pkg_name = pkg_name.clone();
			app.wiki_content.clear();
			app.wiki_err_msg = None;
			app.wiki_scroll = 0;
			trigger_wiki_fetch(pkg_name, tx.clone());
		}
		KeyCode::Char('i') if !app.view.is_empty() && app.cursor < app.view.len() => {
			let name = app.pkgs[app.view[app.cursor]].name.clone();
			let names = if app.selected.is_empty() {
				vec![name]
			} else {
				app.selected.iter().cloned().collect()
			};
			app.confirm = Some((ConfirmAction::Install, names));
		}
		KeyCode::Char('r') if !app.view.is_empty() && app.cursor < app.view.len() => {
			let name = app.pkgs[app.view[app.cursor]].name.clone();
			let names = if app.selected.is_empty() {
				vec![name]
			} else {
				app.selected.iter().cloned().collect()
			};
			app.confirm = Some((ConfirmAction::Remove, names));
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
		KeyCode::Char(c) if c.is_ascii_digit() => {
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
		KeyCode::Char('v') | KeyCode::Char('V') => {
			app.show_dep_tree = !app.show_dep_tree;
			app.detail_top = 0; // reset scroll
			if app.show_dep_tree && !app.view.is_empty() && app.cursor < app.view.len()
			{
				let pkg = &app.pkgs[app.view[app.cursor]];
				if app.dep_tree_pkg_name.as_ref() != Some(&pkg.name) {
					app.dep_tree_loading = true;
					app.dep_tree_content.clear();
					trigger_dep_tree_fetch(
						pkg.name.clone(),
						pkg.installed,
						tx.clone(),
					);
				}
			}
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

pub fn trigger_wiki_fetch(pkg_name: String, tx: mpsc::UnboundedSender<AppEvent>) {
	tokio::spawn(async move {
		let res = match fetch_wiki_text(&pkg_name).await {
			Ok(text) => {
				let cleaned = clean_wikitext(&text);
				let lines = cleaned
					.lines()
					.map(|s| s.to_string())
					.collect::<Vec<String>>();
				Ok(lines)
			}
			Err(e) => Err(e),
		};
		let _ = tx.send(AppEvent::WikiLoaded(pkg_name, res));
	});
}

async fn fetch_wiki_text(pkg_name: &str) -> Result<String, String> {
	// 1. Try exact name query
	match fetch_revisions_api(pkg_name).await {
		Ok(content) => return Ok(content),
		Err(e) if e == "Missing" => {}
		Err(e) => return Err(e),
	}

	// 2. Try base name query (e.g. python-pip -> pip)
	let base = base_package_name(pkg_name);
	if base != pkg_name {
		match fetch_revisions_api(base).await {
			Ok(content) => return Ok(content),
			Err(e) if e == "Missing" => {}
			Err(e) => return Err(e),
		}
	}

	// 3. Try search API for exact name
	if let Some(title) = search_wiki_title(pkg_name).await? {
		match fetch_revisions_api(&title).await {
			Ok(content) => return Ok(content),
			Err(e) if e == "Missing" => {}
			Err(e) => return Err(e),
		}
	}

	// 4. Try search API for base name
	if base != pkg_name
		&& let Some(title) = search_wiki_title(base).await?
		&& let Ok(content) = fetch_revisions_api(&title).await
	{
		return Ok(content);
	}

	Err(format!("No wiki page found for '{}'", pkg_name))
}

async fn fetch_revisions_api(title: &str) -> Result<String, String> {
	let url = format!(
		"https://wiki.archlinux.org/api.php?action=query&prop=revisions&titles={}&rvslots=main&rvprop=content&format=json&redirects=1",
		url_encode(title)
	);

	let output = tokio::process::Command::new("curl")
		.args(["-sL", &url])
		.output()
		.await
		.map_err(|e| format!("Failed to execute curl: {}", e))?;

	if !output.status.success() {
		return Err("Curl request failed".to_string());
	}

	let json_str = String::from_utf8_lossy(&output.stdout);
	let val: serde_json::Value =
		serde_json::from_str(&json_str).map_err(|e| format!("JSON parse error: {}", e))?;

	if let Some(pages) = val["query"]["pages"].as_object() {
		for (_id, page) in pages {
			if page["missing"].is_string() {
				return Err("Missing".to_string());
			}
			if let Some(revisions) = page["revisions"].as_array()
				&& let Some(rev) = revisions.first()
				&& let Some(content) = rev["slots"]["main"]["*"].as_str()
			{
				return Ok(content.to_string());
			}
		}
	}

	Err("Invalid API response structure".to_string())
}

async fn search_wiki_title(query: &str) -> Result<Option<String>, String> {
	let url = format!(
		"https://wiki.archlinux.org/api.php?action=query&list=search&srsearch={}&format=json",
		url_encode(query)
	);

	let output = tokio::process::Command::new("curl")
		.args(["-sL", &url])
		.output()
		.await
		.map_err(|e| format!("Failed to execute curl: {}", e))?;

	if !output.status.success() {
		return Err("Curl request failed".to_string());
	}

	let json_str = String::from_utf8_lossy(&output.stdout);
	let val: serde_json::Value =
		serde_json::from_str(&json_str).map_err(|e| format!("JSON parse error: {}", e))?;

	if let Some(search_results) = val["query"]["search"].as_array()
		&& let Some(first_result) = search_results.first()
		&& let Some(title) = first_result["title"].as_str()
	{
		return Ok(Some(title.to_string()));
	}

	Ok(None)
}

fn base_package_name(name: &str) -> &str {
	if let Some(rest) = name.strip_prefix("python-") {
		rest
	} else if let Some(rest) = name.strip_prefix("rust-") {
		rest
	} else if let Some(rest) = name.strip_prefix("ruby-") {
		rest
	} else if let Some(rest) = name.strip_prefix("perl-") {
		rest
	} else if let Some(rest) = name.strip_prefix("php-") {
		rest
	} else if let Some(rest) = name.strip_prefix("go-") {
		rest
	} else if let Some(rest) = name.strip_prefix("nodejs-") {
		rest
	} else if let Some(rest) = name.strip_prefix("lib") {
		rest
	} else {
		name
	}
}

fn url_encode(s: &str) -> String {
	let mut encoded = String::new();
	for c in s.chars() {
		if c.is_ascii_alphanumeric()
			|| c == '.' || c == '_'
			|| c == '+' || c == '-'
			|| c == '@'
		{
			encoded.push(c);
		} else if c == ' ' {
			encoded.push_str("%20");
		} else {
			encoded.push_str(&format!("%{:02X}", c as u32));
		}
	}
	encoded
}

fn clean_wikitext(text: &str) -> String {
	let mut result = String::new();
	for line in text.lines() {
		let trimmed = line.trim();
		// Skip categories and language links
		if trimmed.starts_with("[[Category:") {
			continue;
		}
		// Language links: e.g. [[de:Python]]
		if trimmed.starts_with("[[")
			&& trimmed.ends_with("]]")
			&& trimmed.contains(':')
			&& !trimmed.contains("Wikipedia:")
		{
			let inner = &trimmed[2..trimmed.len() - 2];
			if let Some(colon_idx) = inner.find(':') {
				let prefix = &inner[..colon_idx];
				if prefix.len() <= 10
					&& prefix
						.chars()
						.all(|c| c.is_ascii_lowercase() || c == '-')
				{
					continue;
				}
			}
		}
		// Skip template starts/ends/contents
		if trimmed.starts_with("{{")
			|| trimmed.starts_with("}}")
			|| trimmed.starts_with("|")
		{
			continue;
		}

		// Process the line to resolve internal/external links and formatting
		let mut cleaned_line = String::new();
		let chars = line.chars().collect::<Vec<char>>();
		let mut i = 0;
		while i < chars.len() {
			// Internal links: [[PageName|Display]] or [[PageName]]
			if i + 1 < chars.len() && chars[i] == '[' && chars[i + 1] == '[' {
				let mut found_close = false;
				let mut close_idx = 0;
				for j in i + 2..chars.len() - 1 {
					if chars[j] == ']' && chars[j + 1] == ']' {
						found_close = true;
						close_idx = j;
						break;
					}
				}
				if found_close {
					let inner =
						chars[i + 2..close_idx].iter().collect::<String>();
					if let Some(pipe_idx) = inner.find('|') {
						let display = &inner[pipe_idx + 1..];
						cleaned_line.push_str(display);
					} else {
						cleaned_line.push_str(&inner);
					}
					i = close_idx + 2;
					continue;
				}
			}
			// External links: [url Display] or [url]
			if chars[i] == '['
				&& let Some(offset) = chars[i + 1..].iter().position(|&c| c == ']')
			{
				let close_idx = i + 1 + offset;
				let inner =
					chars[i + 1..close_idx].iter().collect::<String>();
				if let Some(space_idx) = inner.find(' ') {
					let url = &inner[..space_idx];
					let display = &inner[space_idx + 1..];
					cleaned_line.push_str(&format!(
						"{} ({})",
						display, url
					));
				} else {
					cleaned_line.push_str(&inner);
				}
				i = close_idx + 1;
				continue;
			}
			// Bold/Italics: '' or '''
			if chars[i] == '\'' {
				while i < chars.len() && chars[i] == '\'' {
					i += 1;
				}
				continue;
			}

			cleaned_line.push(chars[i]);
			i += 1;
		}

		result.push_str(&cleaned_line);
		result.push('\n');
	}
	result
}

fn delete_last_word(s: &mut String) {
	// 1. Pop all trailing whitespace
	while let Some(c) = s.chars().next_back() {
		if c.is_whitespace() {
			s.pop();
		} else {
			break;
		}
	}

	// 2. If empty, return
	if s.is_empty() {
		return;
	}

	// 3. Check if the last character is a boundary symbol
	if let Some(c) = s.chars().next_back()
		&& (c == '-' || c == '_' || c == '/' || c == '.' || c == '@' || c == ':')
	{
		s.pop();
		return;
	}

	// 4. Pop consecutive word characters (alphanumeric)
	while let Some(c) = s.chars().next_back() {
		if c.is_alphanumeric() {
			s.pop();
		} else {
			break;
		}
	}
}
