mod app;
mod config;
mod event;
mod handlers;
mod theme;
mod ui;

use app::App;
use event::AppEvent;
use handlers::{handle_key, trigger_db_reload};

use crossterm::{
	event::{self as ct_event, Event as CrosstermEvent},
	execute,
	terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::{error::Error, io, time::Duration};
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
	// Check if pacman command is available on user's system
	if std::process::Command::new("which")
		.arg("pacman")
		.output()
		.map(|o| !o.status.success())
		.unwrap_or(true)
	{
		eprintln!("Error: pacman not found on this system.");
		std::process::exit(1);
	}

	// Initialize raw terminal mode
	enable_raw_mode()?;
	let mut stdout = io::stdout();
	execute!(stdout, EnterAlternateScreen)?;
	let backend = CrosstermBackend::new(stdout);
	let mut terminal = Terminal::new(backend)?;

	// Channel for application event passing
	let (tx, mut rx) = mpsc::unbounded_channel::<AppEvent>();

	// Spawn input key event listener task
	let tx_key = tx.clone();
	tokio::spawn(async move {
		loop {
			if ct_event::poll(Duration::from_millis(10)).unwrap_or(false)
				&& let Ok(ev) = ct_event::read()
			{
				match ev {
					CrosstermEvent::Key(key) => {
						let _ = tx_key.send(AppEvent::Key(key));
					}
					CrosstermEvent::Resize(_, _) => {
						let _ = tx_key.send(AppEvent::Resize);
					}
					_ => {}
				}
			}
			tokio::time::sleep(Duration::from_millis(10)).await;
		}
	});

	// Spawn tick generator task
	let tx_tick = tx.clone();
	tokio::spawn(async move {
		loop {
			tokio::time::sleep(Duration::from_millis(100)).await;
			let _ = tx_tick.send(AppEvent::Tick);
		}
	});

	// Initialize App State
	let mut app = App::new();
	app.is_loading = true;

	// Initial package DB loading trigger
	trigger_db_reload(tx.clone());

	// Clear terminal screen on startup
	let _ = terminal.clear();

	// Main event loop
	loop {
		if app.terminal_needs_clear {
			let _ = terminal.clear();
			app.terminal_needs_clear = false;
		}

		if app.needs_filter {
			app.apply_filter();
			app.needs_filter = false;
		}

		terminal.draw(|f| ui::render(f, &mut app))?;

		if let Some(event) = rx.recv().await {
			match event {
				AppEvent::Tick => {
					if app.is_loading {
						app.spinner_tick += 1;
					}
					app.check_msg_expiry();

					// Check if we need to fetch details for selected AUR package
					if !app.view.is_empty() && app.cursor < app.view.len() {
						let idx = app.view[app.cursor];
						if idx < app.pkgs.len()
							&& config::cfg().aur && app.pkgs[idx].repo
							== "aur" && app.pkgs[idx].desc == "AUR Package"
							&& app.last_cursor_change.elapsed()
								> Duration::from_millis(300)
						{
							let name = app.pkgs[idx].name.clone();
							// Mark as fetching so we don't spawn multiple tasks
							app.pkgs[idx].desc =
								"Fetching details...".to_string();
							handlers::trigger_aur_details_fetch(
								name,
								tx.clone(),
							);
						}
					}

					// Check if we need to fetch the dependency tree
					if app.show_dep_tree && !app.view.is_empty() && app.cursor < app.view.len() {
						let idx = app.view[app.cursor];
						if idx < app.pkgs.len() {
							let name = &app.pkgs[idx].name;
							if app.dep_tree_pkg_name.as_ref() != Some(name)
								&& !app.dep_tree_loading
								&& app.last_cursor_change.elapsed() > Duration::from_millis(250)
							{
								app.dep_tree_loading = true;
								app.dep_tree_content.clear();
								handlers::trigger_dep_tree_fetch(
									name.clone(),
									app.pkgs[idx].installed,
									tx.clone(),
								);
							}
						}
					}
				}
				AppEvent::Key(key) => {
					if handle_key(key, &mut app, &tx) {
						break; // Exit loop
					}
				}
				AppEvent::DbLoaded(pkgs) => {
					app.pkgs = pkgs;
					app.update_installed_cache();
					app.needs_filter = true;
				}
				AppEvent::AurLoaded(aur) => {
					let known: std::collections::HashSet<String> =
						app.pkgs.iter().map(|p| p.name.clone()).collect();
					for a in aur {
						if !known.contains(&a.name) {
							app.pkgs.push(a);
						}
					}
					app.update_installed_cache();
					app.is_loading = false;
					let count = app.pkgs.len();
					app.set_msg(
						&format!("Loaded {} packages.", count),
						4,
						false,
					);
					app.needs_filter = true;
				}
				AppEvent::Message(msg, secs, keep) => {
					app.set_msg(&msg, secs, keep);
				}
				AppEvent::LoadingDone => {
					app.is_loading = false;
				}
				AppEvent::ConsoleChunk(chunk) => {
					app.write_console_chunk(&chunk);
				}
				AppEvent::ConsoleFinished(success) => {
					app.console_finished = Some(success);
					app.is_loading = false;
					if success {
						trigger_db_reload(tx.clone());
					}
				}
				AppEvent::Resize => {
					let _ = terminal.clear();
				}
				AppEvent::AurDetailsLoaded(fetched) => {
					if let Some(idx) =
						app.pkgs.iter().position(|p| p.name == fetched.name)
					{
						let installed = app.pkgs[idx].installed;
						let upgradable = app.pkgs[idx].upgradable;
						app.pkgs[idx] = *fetched;
						app.pkgs[idx].installed = installed;
						app.pkgs[idx].upgradable = upgradable;
						app.pkgs[idx].repo = "aur".to_string();
					}
					app.update_installed_cache();
				}
				AppEvent::DepTreeLoaded(pkg_name, res) => {
					if !app.view.is_empty() && app.cursor < app.view.len() {
						let current_pkg_name = &app.pkgs[app.view[app.cursor]].name;
						if current_pkg_name == &pkg_name {
							app.dep_tree_loading = false;
							match res {
								Ok(tree) => {
									app.dep_tree_content = tree;
									app.dep_tree_pkg_name = Some(pkg_name);
								}
								Err(err) => {
									app.dep_tree_content = vec![
										"Error loading dependency tree:".to_string(),
										err,
									];
									app.dep_tree_pkg_name = Some(pkg_name);
								}
							}
						}
					}
				}
			}
		}
	}

	// Gracefully restore terminal settings
	disable_raw_mode()?;
	execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
	terminal.show_cursor()?;

	Ok(())
}
