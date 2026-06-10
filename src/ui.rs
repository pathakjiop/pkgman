use ratatui::{
    layout::{Constraint, Direction, Layout, Rect, Alignment},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
    Frame,
};

use crate::app::{App, ConfirmAction, FILTERS};

const HELP_LINES: &[(&str, &str)] = &[
    ("NAVIGATION", ""),
    ("↑ / k", "Move up"),
    ("↓ / j", "Move down"),
    ("PgUp / PgDn", "Scroll full page"),
    ("J / K", "Scroll detail pane"),
    ("Home / End", "Jump to top / bottom"),
    ("", ""),
    ("TABS & SEARCH", ""),
    ("1-7", "Direct filter tabs"),
    ("/", "Live Search"),
    ("Esc", "Exit search / Clear"),
    ("", ""),
    ("SELECTION & ACTIONS", ""),
    ("Space", "Toggle selection"),
    ("i", "Install selected"),
    ("u", "Update selected"),
    ("r", "Remove selected"),
    ("d", "Curl / Download website"),
    ("E (shift)", "Run custom curl script"),
    ("R (shift)", "Reload packages"),
    ("s", "Cycle sort mode"),
    ("t", "Theme Selector / Builder"),
    ("?", "Toggle help"),
    ("q", "Quit"),
];

const SPINNER: &[char] = &['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];

// ── colour helpers ──────────────────────────────────────────────────

fn repo_color(repo: &str, theme: &crate::theme::Theme) -> Color {
    match repo {
        "core" => theme.accent,
        "extra" => theme.selected,
        "community" => theme.success,
        "aur" => theme.accent,
        _ => theme.border,
    }
}

fn status_icon(installed: bool, upgradable: bool, theme: &crate::theme::Theme) -> Span<'static> {
    if upgradable {
        Span::styled(" ↑", Style::default().fg(theme.warning).add_modifier(Modifier::BOLD))
    } else if installed {
        Span::styled(" ✔", Style::default().fg(theme.success))
    } else {
        Span::styled(" ○", Style::default().fg(theme.border))
    }
}

// ── main render ─────────────────────────────────────────────────────

pub fn render(f: &mut Frame, app: &mut App) {
    let size = f.size();

    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),  // header
            Constraint::Length(1),  // tab bar
            Constraint::Length(1),  // divider
            Constraint::Min(6),    // main panels
            Constraint::Length(1),  // footer
        ])
        .split(size);

    render_header(f, app, outer[0]);
    render_tab_bar(f, app, outer[1]);
    render_divider(f, app, outer[2]);
    render_main(f, app, outer[3]);
    render_footer(f, app, outer[4]);

    // ── overlays ──
    if app.show_help {
        render_help_overlay(f, size, app.help_scroll);
    }
    if let Some((action, names)) = &app.confirm {
        render_confirm_overlay(f, size, action, names);
    }
    if app.url_input_mode {
        render_url_input_overlay(f, size, &app.url_query);
    } else if let Some((url, content)) = &app.script_preview {
        render_script_preview_overlay(f, size, url, content);
    }
    if app.sudo_password_mode {
        render_sudo_password_overlay(f, size, &app.sudo_password);
    }
    if app.console_mode {
        render_console_overlay(f, size, app);
    }
    if app.theme_builder_open {
        render_theme_builder_overlay(f, size, app);
    }
}

// ── header ──────────────────────────────────────────────────────────

fn render_header(f: &mut Frame, app: &App, area: Rect) {
    let spin = if app.is_loading {
        let idx = (app.spinner_tick as usize) % SPINNER.len();
        format!(" {} ", SPINNER[idx])
    } else {
        " ".into()
    };

    let sort_key = match app.sort_idx {
        0 => "Name",
        1 => "Repo",
        2 => "Size",
        3 => "Installed",
        _ => "?",
    };

    let left = Line::from(vec![
        Span::styled(spin, Style::default().fg(app.theme.accent).add_modifier(Modifier::BOLD)),
        Span::styled(
            "PKGMAN",
            Style::default().fg(app.theme.accent).add_modifier(Modifier::BOLD),
        ),
        Span::styled(" │ ", Style::default().fg(app.theme.border)),
        Span::styled("Arch Package Manager", Style::default().fg(app.theme.border)),
    ]);

    let stats = format!(
        "Total: {} │ Disk: {} │ Sort: {}",
        app.pkgs.len(), app.disk_free, sort_key
    );

    f.render_widget(Paragraph::new(left), area);

    if stats.len() < area.width as usize {
        let r = Rect {
            x: area.right().saturating_sub(stats.len() as u16 + 1),
            y: area.y,
            width: stats.len() as u16 + 1,
            height: 1,
        };
        f.render_widget(
            Paragraph::new(stats.as_str()).style(Style::default().fg(app.theme.border)),
            r,
        );
    }
}

// ── tab bar ─────────────────────────────────────────────────────────

fn render_tab_bar(f: &mut Frame, app: &App, area: Rect) {
    let upd_count = app.pkgs.iter().filter(|p| p.upgradable).count();
    let inst_count = app.pkgs.iter().filter(|p| p.installed).count();

    let mut spans: Vec<Span> = Vec::new();

    for (i, &filter) in FILTERS.iter().enumerate() {
        let count = match filter {
            "updates" if upd_count > 0 => Some(upd_count),
            "installed" => Some(inst_count),
            _ => None,
        };

        let label = if let Some(c) = count {
            format!(" {} ({}) ", filter.to_uppercase(), c)
        } else {
            format!(" {} ", filter.to_uppercase())
        };

        let style = if i == app.filter_idx {
            Style::default()
                .fg(app.theme.highlight_fg)
                .bg(app.theme.highlight_bg)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(app.theme.foreground)
        };

        spans.push(Span::styled(label, style));
        spans.push(Span::raw(" "));
    }

    f.render_widget(Paragraph::new(Line::from(spans)), area);
}

// ── divider ─────────────────────────────────────────────────────────

fn render_divider(f: &mut Frame, app: &App, area: Rect) {
    f.render_widget(
        Paragraph::new("─".repeat(area.width as usize))
            .style(Style::default().fg(app.theme.border)),
        area,
    );
}

// ── main panels ─────────────────────────────────────────────────────

fn render_main(f: &mut Frame, app: &mut App, area: Rect) {
    let wide = area.width >= 130;

    let cols = if wide {
        let list_w = (area.width as usize / 3).max(36).min(44);
        let ctx_w = 34u16;
        let _detail_w = area.width.saturating_sub(list_w as u16).saturating_sub(ctx_w);
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(list_w as u16),
                Constraint::Min(30),
                Constraint::Length(ctx_w),
            ])
            .split(area)
    } else {
        let list_w = (area.width as usize / 3).max(36).min(44);
        let _detail_w = area.width.saturating_sub(list_w as u16);
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(list_w as u16),
                Constraint::Min(20),
            ])
            .split(area)
    };

    render_package_list(f, app, cols[0]);

    if cols.len() > 2 {
        render_detail_panel(f, app, cols[1]);
        render_context_panel(f, app, cols[2]);
    } else {
        render_detail_panel(f, app, cols[1]);
    }
}

// ── package list ────────────────────────────────────────────────────

fn render_package_list(f: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // search row
            Constraint::Length(1), // divider
            Constraint::Min(3),   // list
        ])
        .split(area);

    // search prompt
    let search_style = if app.search_mode {
        Style::default().fg(app.theme.highlight_fg).bg(app.theme.highlight_bg)
    } else if !app.query.is_empty() {
        Style::default().fg(app.theme.accent)
    } else {
        Style::default().fg(app.theme.border)
    };

    let search_txt = if app.query.is_empty() && !app.search_mode {
        " / Search…".into()
    } else {
        format!(" / {}█", app.query)
    };

    f.render_widget(Paragraph::new(search_txt).style(search_style), chunks[0]);

    f.render_widget(
        Paragraph::new("─".repeat(chunks[1].width as usize))
            .style(Style::default().fg(app.theme.border)),
        chunks[1],
    );

    // build list items
    let col_w = chunks[2].width as usize;
    let inner_w = col_w.saturating_sub(2); // borders

    let mut items: Vec<ListItem> = Vec::new();
    for &idx in &app.view {
        if idx >= app.pkgs.len() {
            continue;
        }
        let pkg = &app.pkgs[idx];

        let icon = status_icon(pkg.installed, pkg.upgradable, &app.theme);

        let selected = app.selected.contains(&pkg.name);
        let name_style = if selected {
            Style::default()
                .fg(app.theme.selected)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(app.theme.foreground)
        };

        // name + version badge  e.g.  "arianna [26.04.2-1]"
        let ver_str = format!("[{}]", pkg.version);
        let name_part_len = inner_w.saturating_sub(6 + ver_str.len()); // icon(2) + gaps(2) + ver
        let name_display = if pkg.name.len() > name_part_len {
            truncate_str(&pkg.name, name_part_len)
        } else {
            pkg.name.clone()
        };

        let name_span = Span::styled(format!(" {}", name_display), name_style);
        let ver_span = Span::styled(
            format!(" {}", ver_str),
            Style::default().fg(repo_color(&pkg.repo, &app.theme)),
        );

        items.push(ListItem::new(Line::from(vec![icon, name_span, ver_span])));
    }

    // visible‐range info
    let visible_info = if !app.view.is_empty() {
        let lo = app.cursor + 1;
        let hi = app.view.len();
        format!(" {}-{} / {} ", lo, hi, hi)
    } else {
        " No packages ".into()
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Packages ")
        .title_bottom(Line::from(Span::styled(
            visible_info,
            Style::default().fg(app.theme.border),
        )))
        .border_style(Style::default().fg(app.theme.border));

    let mut state = ratatui::widgets::ListState::default();
    state.select(Some(app.cursor));

    let list = List::new(items)
        .block(block)
        .highlight_style(
            Style::default()
                .fg(app.theme.highlight_fg)
                .bg(app.theme.highlight_bg)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">>");

    f.render_stateful_widget(list, chunks[2], &mut state);
}

// ── detail panel ────────────────────────────────────────────────────

fn render_detail_panel(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Details ")
        .border_style(Style::default().fg(app.theme.border));

    let inner = block.inner(area);
    f.render_widget(block, area);

    if app.view.is_empty() || app.cursor >= app.view.len() {
        render_empty_detail(f, inner, &app.theme);
        return;
    }

    let pkg_idx = app.view[app.cursor];
    if pkg_idx >= app.pkgs.len() {
        render_empty_detail(f, inner, &app.theme);
        return;
    }
    let pkg = &app.pkgs[pkg_idx];

    // split into info + actions
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(4),       // info
            Constraint::Length(5),    // action buttons
        ])
        .split(inner);

    render_detail_info(f, pkg, chunks[0], app.detail_top as u16, &app.theme);
    render_action_buttons(f, pkg, chunks[1], &app.theme);
}

fn render_empty_detail(f: &mut Frame, area: Rect, theme: &crate::theme::Theme) {
    let txt = Paragraph::new("\n  Select a package to view details")
        .style(Style::default().fg(theme.border));
    f.render_widget(txt, area);
}

/// Render a `label  value` detail row, word-wrapping the value with a hanging
/// indent so continuation lines align under the value column instead of col 0.
fn wrap_field(label: &str, value: &str, vc: Style, width: usize, kw: usize, theme: &crate::theme::Theme) -> Vec<Line<'static>> {
    let lbl = Style::default().fg(theme.border);
    let indent = 2 + kw + 2; // leading + label + separator
    let avail = width.saturating_sub(indent).max(1);

    let mut rows: Vec<String> = Vec::new();
    let mut cur = String::new();
    for word in value.split_whitespace() {
        if cur.is_empty() {
            cur.push_str(word);
        } else if cur.chars().count() + 1 + word.chars().count() <= avail {
            cur.push(' ');
            cur.push_str(word);
        } else {
            rows.push(std::mem::take(&mut cur));
            cur.push_str(word);
        }
    }
    if !cur.is_empty() {
        rows.push(cur);
    }
    if rows.is_empty() {
        rows.push(String::new());
    }

    rows.into_iter()
        .enumerate()
        .map(|(i, row)| {
            let prefix = if i == 0 {
                format!("  {:<width$}  ", label, width = kw)
            } else {
                " ".repeat(indent)
            };
            Line::from(vec![Span::styled(prefix, lbl), Span::styled(row, vc)])
        })
        .collect()
}

fn render_detail_info(f: &mut Frame, pkg: &crate::app::Package, area: Rect, scroll: u16, theme: &crate::theme::Theme) {
    let kw = 16; // label width
    let sep = Style::default().fg(theme.border);
    let val = Style::default().fg(theme.foreground);
    let hdr = Style::default().fg(theme.accent).add_modifier(Modifier::BOLD);

    let inner_w = area.width as usize;
    macro_rules! field {
        ($lines:expr, $label:expr, $value:expr, $vc:expr) => {
            $lines.extend(wrap_field($label, $value.as_str(), Style::from($vc), inner_w, kw, theme));
        };
    }

    let mut lines: Vec<Line> = Vec::new();

    // ── Title row ──
    let status_txt = if pkg.upgradable {
        "  ↑ UPDATE AVAILABLE"
    } else if pkg.installed {
        "  ✔ INSTALLED"
    } else {
        "  NOT INSTALLED"
    };
    let status_col = if pkg.upgradable {
        theme.warning
    } else if pkg.installed {
        theme.success
    } else {
        theme.border
    };

    lines.push(Line::from(vec![
        Span::styled(
            format!(" {} ", pkg.name),
            Style::default().fg(theme.foreground).add_modifier(Modifier::BOLD),
        ),
        Span::styled(status_txt, Style::default().fg(status_col).add_modifier(Modifier::BOLD)),
    ]));
    lines.push(Line::from(""));

    // ── IDENTITY ──
    lines.push(Line::from(Span::styled("  IDENTITY", hdr)));
    lines.push(Line::from(Span::styled(
        "  ──────────────────────────────────",
        sep,
    )));
    field!(lines, "Repository", pkg.repo, repo_color(&pkg.repo, theme));
    field!(lines, "Name", pkg.name, val.clone());
    field!(lines, "Version", pkg.version, theme.accent);
    field!(lines, "Architecture", pkg.arch, val.clone());
    if pkg.groups != "None" && !pkg.groups.is_empty() {
        field!(lines, "Groups", pkg.groups, val.clone());
    }
    lines.push(Line::from(""));

    // ── DESCRIPTION ──
    lines.push(Line::from(Span::styled("  DESCRIPTION", hdr)));
    lines.push(Line::from(Span::styled(
        "  ──────────────────────────────────",
        sep,
    )));
    lines.push(Line::from(Span::styled(
        format!("  {}", pkg.desc),
        Style::default().fg(theme.foreground),
    )));
    lines.push(Line::from(""));

    // ── SIZE ──
    if pkg.dl_size != "None" || pkg.inst_size != "None" {
        lines.push(Line::from(Span::styled("  SIZE", hdr)));
        lines.push(Line::from(Span::styled(
            "  ──────────────────────────────────",
            sep,
        )));
        if pkg.dl_size != "None" && !pkg.dl_size.is_empty() {
            field!(lines, "Download Size", pkg.dl_size, theme.selected);
        }
        if pkg.inst_size != "None" && !pkg.inst_size.is_empty() {
            field!(lines, "Installed Size", pkg.inst_size, theme.selected);
        }
        lines.push(Line::from(""));
    }

    // ── DEPENDENCIES ──
    lines.push(Line::from(Span::styled("  DEPENDENCIES", hdr)));
    lines.push(Line::from(Span::styled(
        "  ──────────────────────────────────",
        sep,
    )));
    if pkg.depends != "None" && !pkg.depends.is_empty() {
        field!(lines, "Depends On", pkg.depends, val.clone());
    }
    if pkg.optdeps != "None" && !pkg.optdeps.is_empty() {
        field!(lines, "Optional Deps", pkg.optdeps, Style::default().fg(theme.border));
    }
    if pkg.req_by != "None" && !pkg.req_by.is_empty() {
        field!(lines, "Required By", pkg.req_by, val.clone());
    }
    if pkg.opt_for != "None" && !pkg.opt_for.is_empty() {
        field!(lines, "Optional For", pkg.opt_for, Style::default().fg(theme.border));
    }
    lines.push(Line::from(""));

    // ── CONFLICTS & MISC ──
    let has_conflicts = pkg.conflicts != "None" && !pkg.conflicts.is_empty();
    let has_replaces = pkg.replaces != "None" && !pkg.replaces.is_empty();
    let has_provides = pkg.provides != "None" && !pkg.provides.is_empty();
    let has_licenses = pkg.licenses != "None" && !pkg.licenses.is_empty();
    let has_url = !pkg.url.is_empty() && pkg.url != "None";
    let has_build_date = pkg.build_date != "None" && !pkg.build_date.is_empty();

    if has_conflicts || has_replaces || has_provides || has_licenses || has_url || has_build_date {
        lines.push(Line::from(Span::styled("  MORE", hdr)));
        lines.push(Line::from(Span::styled(
            "  ──────────────────────────────────",
            sep,
        )));
        if has_provides {
            field!(lines, "Provides", pkg.provides, val.clone());
        }
        if has_conflicts {
            field!(lines, "Conflicts", pkg.conflicts, theme.error);
        }
        if has_replaces {
            field!(lines, "Replaces", pkg.replaces, val.clone());
        }
        if has_licenses {
            field!(lines, "Licenses", pkg.licenses, Style::default().fg(theme.border));
        }
        if has_build_date {
            field!(lines, "Build Date", pkg.build_date, val.clone());
        }
        if has_url {
            field!(lines, "URL", pkg.url, theme.accent);
        }
    }

    let paragraph = Paragraph::new(lines)
        .wrap(Wrap { trim: false })
        .scroll((scroll, 0));

    f.render_widget(paragraph, area);
}

// ── action buttons ──────────────────────────────────────────────────

fn render_action_buttons(f: &mut Frame, pkg: &crate::app::Package, area: Rect, theme: &crate::theme::Theme) {
    let sep = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // divider line
            Constraint::Length(3), // buttons
        ])
        .split(area);

    f.render_widget(
        Paragraph::new("─".repeat(sep[0].width as usize))
            .style(Style::default().fg(theme.border)),
        sep[0],
    );

    let btn_area = sep[1];
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(14),
            Constraint::Length(14),
            Constraint::Length(14),
            Constraint::Length(14),
        ])
        .split(btn_area);

    let draw_btn = |f_: &mut Frame, r: Rect, label: &str, key: &str, col: Color, active: bool| {
        let border_col = if active { col } else { theme.border };
        let text_col = if active { col } else { theme.border };
        let txt = if active {
            format!("{}{}", key, label)
        } else {
            format!(" {} ", label)
        };
        f_.render_widget(
            Paragraph::new(txt)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(border_col)),
                )
                .style(
                    Style::default()
                        .fg(text_col)
                        .add_modifier(Modifier::BOLD),
                )
                .alignment(Alignment::Center),
            r,
        );
    };

    // Determine which actions are applicable
    let can_install = !pkg.installed;
    let can_update = pkg.upgradable;
    let can_remove = pkg.installed;

    draw_btn(f, cols[0], " Install ", "[i]", theme.accent, can_install);
    draw_btn(f, cols[1], " Update ↑ ", "[u]", theme.warning, can_update);
    draw_btn(f, cols[2], " Uninstall ", "[r]", theme.error, can_remove);
    draw_btn(f, cols[3], " Curl WWW ", "[d]", theme.accent, true);
}

// ── context panel ───────────────────────────────────────────────────

fn render_context_panel(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Context ")
        .border_style(Style::default().fg(app.theme.border));

    let inner = block.inner(area);
    f.render_widget(block, area);

    if app.view.is_empty() || app.cursor >= app.view.len() {
        return;
    }
    let pkg_idx = app.view[app.cursor];
    if pkg_idx >= app.pkgs.len() {
        return;
    }
    let pkg = &app.pkgs[pkg_idx];

    let hdr = Style::default()
        .fg(app.theme.accent)
        .add_modifier(Modifier::BOLD);
    let lbl = Style::default().fg(app.theme.border);
    let val = Style::default().fg(app.theme.foreground);

    let mut lines: Vec<Line> = Vec::new();

    // ── Quick Info ──
    lines.push(Line::from(Span::styled(" QUICK INFO", hdr)));
    lines.push(Line::from(Span::styled(
        " ──────────────────────────",
        Style::default().fg(app.theme.border),
    )));

    macro_rules! row {
        ($l:expr, $v:expr) => {
            lines.push(Line::from(vec![
                Span::styled(format!(" {:<12}", $l), lbl),
                Span::styled($v, val.clone()),
            ]));
        };
    }

    row!("Repo", &pkg.repo);
    row!("Version", &pkg.version);
    row!("Arch", &pkg.arch);
    row!("Installed", if pkg.installed { "Yes" } else { "No" });
    row!("Upgradable", if pkg.upgradable { "Yes" } else { "No" });
    lines.push(Line::from(""));

    // ── Links ──
    lines.push(Line::from(Span::styled(" LINKS", hdr)));
    lines.push(Line::from(Span::styled(
        " ──────────────────────────",
        Style::default().fg(app.theme.border),
    )));

    if !pkg.url.is_empty() && pkg.url != "None" {
        lines.push(Line::from(Span::styled(
            format!(" {}", pkg.url),
            Style::default().fg(app.theme.accent),
        )));
        lines.push(Line::from(Span::styled(
            " → Available via [d] Curl",
            Style::default().fg(app.theme.success),
        )));
    } else {
        lines.push(Line::from(Span::styled(
            " No homepage linked",
            Style::default().fg(app.theme.border),
        )));
    }
    lines.push(Line::from(""));

    // ── Maintainer ──
    lines.push(Line::from(Span::styled(" MAINTAINER", hdr)));
    lines.push(Line::from(Span::styled(
        " ──────────────────────────",
        Style::default().fg(app.theme.border),
    )));
    lines.push(Line::from(Span::styled(
        format!(" {}", pkg.packager),
        Style::default().fg(app.theme.foreground),
    )));
    lines.push(Line::from(""));

    // ── Selection ──
    lines.push(Line::from(Span::styled(" SELECTION", hdr)));
    lines.push(Line::from(Span::styled(
        " ──────────────────────────",
        Style::default().fg(app.theme.border),
    )));
    let sel_count = app.selected.len();
    if sel_count > 0 {
        lines.push(Line::from(Span::styled(
            format!(" {} package(s) selected", sel_count),
            Style::default().fg(app.theme.selected),
        )));
        lines.push(Line::from(Span::styled(
            " [i] Install  [u] Update  [r] Remove",
            Style::default().fg(app.theme.border),
        )));
    } else {
        lines.push(Line::from(Span::styled(
            " None — use [Space] to select",
            Style::default().fg(app.theme.border),
        )));
    }

    f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: true }), inner);
}

// ── footer ──────────────────────────────────────────────────────────

fn render_footer(f: &mut Frame, app: &mut App, area: Rect) {
    app.check_msg_expiry();

    let keys = " [/] Search │ [Space] Select │ [1-7] Tabs │ [E] Script │ [t] Theme │ [?] Help │ [q] Quit ";
    f.render_widget(
        Paragraph::new(keys).style(
            Style::default()
                .fg(app.theme.border)
                .bg(app.theme.background)
                .add_modifier(Modifier::REVERSED),
        ),
        area,
    );

    if !app.msg.is_empty() {
        let display = format!(" {} ", app.msg);
        if display.len() < area.width as usize {
            let r = Rect {
                x: area.right().saturating_sub(display.len() as u16 + 1),
                y: area.y,
                width: (display.len() as u16 + 1).min(area.width),
                height: 1,
            };
            f.render_widget(
                Paragraph::new(display).style(
                    Style::default()
                        .fg(app.theme.foreground)
                        .bg(app.theme.background)
                        .add_modifier(Modifier::BOLD | Modifier::REVERSED),
                ),
                r,
            );
        }
    }
}

// ── overlays ────────────────────────────────────────────────────────

fn centered_rect(px: u16, py: u16, r: Rect) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - py) / 2),
            Constraint::Percentage(py),
            Constraint::Percentage((100 - py) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - px) / 2),
            Constraint::Percentage(px),
            Constraint::Percentage((100 - px) / 2),
        ])
        .split(vertical[1])[1]
}

fn render_help_overlay(f: &mut Frame, area: Rect, scroll: usize) {
    let popup = centered_rect(60, 65, area);
    f.render_widget(Clear, popup);

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(Span::styled(
        " ⌨  KEYBOARD REFERENCE ",
        Style::default()
            .fg(Color::Black)
            .bg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(""));

    let hdr_style = Style::default()
        .fg(Color::Cyan)
        .add_modifier(Modifier::BOLD);

    for &(key, desc) in HELP_LINES.iter().skip(scroll) {
        if desc.is_empty() && !key.is_empty() {
            lines.push(Line::from(Span::styled(
                format!("  {}", key),
                hdr_style,
            )));
        } else {
            lines.push(Line::from(vec![
                Span::styled(
                    format!("  {:<20}", key),
                    Style::default().fg(Color::White),
                ),
                Span::styled(desc, Style::default().fg(Color::DarkGray)),
            ]));
        }
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  ↑/↓ Scroll   [Esc] Close",
        Style::default().fg(Color::DarkGray),
    )));

    f.render_widget(
        Paragraph::new(lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Cyan)),
            )
            .style(Style::default().bg(Color::Black))
            .wrap(Wrap { trim: false }),
        popup,
    );
}

fn render_confirm_overlay(f: &mut Frame, area: Rect, action: &ConfirmAction, names: &[String]) {
    let popup = centered_rect(50, 30, area);
    f.render_widget(Clear, popup);

    let (act_str, act_col) = match action {
        ConfirmAction::Install => ("INSTALL", Color::Cyan),
        ConfirmAction::Remove => ("REMOVE", Color::Red),
        ConfirmAction::Update => ("SYSTEM UPDATE", Color::Yellow),
    };

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(Span::styled(
        format!(" {} ", act_str),
        Style::default()
            .fg(Color::Black)
            .bg(act_col)
            .add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(""));

    match action {
        ConfirmAction::Update => {
            lines.push(Line::from("  Perform a full system upgrade (-Syu)?"));
            lines.push(Line::from("  This will sync databases and upgrade all packages."));
        }
        _ => {
            lines.push(Line::from(format!(
                "  {} {} package(s)?",
                act_str.to_lowercase(),
                names.len()
            )));

            let preview = if names.len() > 4 {
                format!("  {}, …", names[..4].join(", "))
            } else {
                format!("  {}", names.join(", "))
            };
            lines.push(Line::from(Span::styled(
                preview,
                Style::default().fg(Color::White),
            )));
        }
    }
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("  [y] Confirm    ", Style::default().fg(act_col).add_modifier(Modifier::BOLD)),
        Span::styled("[n] Cancel", Style::default().fg(Color::DarkGray)),
    ]));

    f.render_widget(
        Paragraph::new(lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(act_col)),
            )
            .style(Style::default().bg(Color::Black))
            .alignment(Alignment::Center),
        popup,
    );
}

fn render_url_input_overlay(f: &mut Frame, area: Rect, query: &str) {
    let popup = centered_rect(60, 25, area);
    f.render_widget(Clear, popup);

    let lines = vec![
        Line::from(Span::styled(
            " RUN CUSTOM SCRIPT (curl | bash) ",
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  URL: ", Style::default().fg(Color::DarkGray)),
            Span::styled(query, Style::default().fg(Color::White)),
            Span::styled("█", Style::default().fg(Color::Cyan)),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  [Enter] Fetch    [Esc] Cancel",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    f.render_widget(
        Paragraph::new(lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Cyan)),
            )
            .style(Style::default().bg(Color::Black))
            .alignment(Alignment::Center),
        popup,
    );
}

fn render_script_preview_overlay(f: &mut Frame, area: Rect, url: &str, content: &str) {
    let popup = centered_rect(80, 80, area);
    f.render_widget(Clear, popup);

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(Span::styled(
        " ⚠  SCRIPT PREVIEW ",
        Style::default()
            .fg(Color::Black)
            .bg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(vec![
        Span::styled("  Source: ", Style::default().fg(Color::DarkGray)),
        Span::styled(url, Style::default().fg(Color::Cyan)),
    ]));
    lines.push(Line::from(Span::styled(
        "  ──────────────────────────────────────",
        Style::default().fg(Color::DarkGray),
    )));

    let all_lines: Vec<&str> = content.lines().collect();
    let limit = 24;
    for &l in all_lines.iter().take(limit) {
        lines.push(Line::from(Span::styled(
            format!("  {}", l),
            Style::default().fg(Color::White),
        )));
    }
    if all_lines.len() > limit {
        lines.push(Line::from(Span::styled(
            format!("  … ({} more lines)", all_lines.len() - limit),
            Style::default().fg(Color::DarkGray),
        )));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled(
            "  [y/Enter] Execute    ",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("[n/Esc] Cancel", Style::default().fg(Color::DarkGray)),
    ]));

    f.render_widget(
        Paragraph::new(lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Yellow)),
            )
            .style(Style::default().bg(Color::Black))
            .wrap(Wrap { trim: true }),
        popup,
    );
}

// ── utilities ───────────────────────────────────────────────────────

fn truncate_str(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else if max > 2 {
        format!("{}…", &s[..max - 1])
    } else {
        s[..max].to_string()
    }
}

fn render_sudo_password_overlay(f: &mut Frame, area: Rect, password: &str) {
    let popup = centered_rect(50, 20, area);
    f.render_widget(Clear, popup);

    let user = std::env::var("USER").unwrap_or_else(|_| "user".to_string());
    let masked_password = "*".repeat(password.len());

    let lines = vec![
        Line::from(Span::styled(
            " SUDO PASSWORD REQUIRED ",
            Style::default()
                .fg(Color::Black)
                .bg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled(format!("[sudo] password for {}: ", user), Style::default().fg(Color::White)),
            Span::styled(masked_password, Style::default().fg(Color::Cyan)),
            Span::styled("█", Style::default().fg(Color::Cyan)),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  [Enter] Submit    [Esc] Cancel",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    f.render_widget(
        Paragraph::new(lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Yellow)),
            )
            .style(Style::default().bg(Color::Black))
            .alignment(Alignment::Center),
        popup,
    );
}

fn render_console_overlay(f: &mut Frame, area: Rect, app: &App) {
    let popup = centered_rect(85, 85, area);
    f.render_widget(Clear, popup);

    let (title, border_color) = match app.console_finished {
        None => (" Subprocess Output ", Color::Yellow),
        Some(true) => (" Execution Successful ", Color::Green),
        Some(false) => (" Execution Failed ", Color::Red),
    };

    let border_style = Style::default().fg(border_color);

    let mut lines = Vec::new();
    for l in &app.console_lines {
        lines.push(Line::from(Span::styled(l, Style::default().fg(Color::White))));
    }
    if !app.current_line.is_empty() {
        lines.push(Line::from(Span::styled(&app.current_line, Style::default().fg(Color::White))));
    }

    let inner_height = popup.height.saturating_sub(2) as usize;
    let max_lines = lines.len();
    let scroll_offset = if app.console_finished.is_none() {
        max_lines.saturating_sub(inner_height)
    } else {
        app.console_scroll.min(max_lines.saturating_sub(inner_height))
    };

    let footer_text = match app.console_finished {
        None => " Running... Please wait ",
        Some(true) => " Finished. Press [Esc] or [Enter] to return ",
        Some(false) => " Failed. Press [Esc] or [Enter] to return ",
    };

    let paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .title(Span::styled(title, Style::default().fg(border_color).add_modifier(Modifier::BOLD)))
                .title_bottom(Line::from(Span::styled(
                    footer_text,
                    Style::default().fg(border_color).add_modifier(Modifier::BOLD),
                )))
                .borders(Borders::ALL)
                .border_style(border_style),
        )
        .style(Style::default().bg(Color::Black))
        .scroll((scroll_offset as u16, 0));

    f.render_widget(paragraph, popup);
}

fn render_theme_builder_overlay(f: &mut Frame, area: Rect, app: &App) {
    let popup = centered_rect(80, 80, area);
    f.render_widget(Clear, popup);

    let border_color = app.theme.accent;
    let block = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(
            " 🎨  THEME BUILDER & SELECTOR ",
            Style::default().fg(app.theme.accent).add_modifier(Modifier::BOLD),
        ))
        .border_style(Style::default().fg(border_color))
        .style(Style::default().bg(app.theme.background).fg(app.theme.foreground));

    let inner = block.inner(popup);
    f.render_widget(block, popup);

    // Split inner into Left (Fields) and Right (Live Preview)
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(50),
            Constraint::Percentage(50),
        ])
        .split(inner);

    // Left pane: Attribute list
    let fields = &[
        ("Theme Preset", if app.theme_builder_selected_theme_idx < crate::theme::THEMES.len() {
            crate::theme::THEMES[app.theme_builder_selected_theme_idx].name
        } else {
            "custom"
        }),
        ("Background", crate::theme::color_name(app.theme.background)),
        ("Foreground", crate::theme::color_name(app.theme.foreground)),
        ("Border", crate::theme::color_name(app.theme.border)),
        ("Highlight FG", crate::theme::color_name(app.theme.highlight_fg)),
        ("Highlight BG", crate::theme::color_name(app.theme.highlight_bg)),
        ("Accent", crate::theme::color_name(app.theme.accent)),
        ("Selected", crate::theme::color_name(app.theme.selected)),
        ("Success", crate::theme::color_name(app.theme.success)),
        ("Warning", crate::theme::color_name(app.theme.warning)),
        ("Error", crate::theme::color_name(app.theme.error)),
    ];

    let mut list_items = Vec::new();
    for (i, &(label, val)) in fields.iter().enumerate() {
        let is_selected = i == app.theme_builder_cursor;
        
        let label_span = Span::styled(
            format!("  {:<15}", label),
            if is_selected {
                Style::default().fg(app.theme.highlight_fg).bg(app.theme.highlight_bg).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(app.theme.foreground)
            }
        );

        let value_span = Span::styled(
            format!(" < {} >", val),
            if is_selected {
                Style::default().fg(app.theme.highlight_fg).bg(app.theme.highlight_bg).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(app.theme.accent).add_modifier(Modifier::BOLD)
            }
        );

        list_items.push(ListItem::new(Line::from(vec![label_span, value_span])));
    }

    let fields_list = List::new(list_items)
        .block(Block::default().borders(Borders::RIGHT).border_style(Style::default().fg(app.theme.border)));

    f.render_widget(fields_list, chunks[0]);

    // Right pane: Live Preview!
    let preview_area = chunks[1];
    let preview_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Heading
            Constraint::Length(1), // Spacer
            Constraint::Length(3), // Mock List (Tab + package)
            Constraint::Length(1), // Spacer
            Constraint::Min(4),    // Mock Details
            Constraint::Length(1), // Spacer
            Constraint::Length(3), // Mock Buttons
        ])
        .split(preview_area);

    f.render_widget(
        Paragraph::new(" LIVE PREVIEW ").alignment(Alignment::Center).style(Style::default().fg(app.theme.accent).add_modifier(Modifier::BOLD)),
        preview_layout[0]
    );

    // Mock List Item
    let mock_icon = status_icon(true, false, &app.theme);
    let mock_name = Span::styled(" example-pkg", Style::default().fg(app.theme.foreground));
    let mock_ver = Span::styled(" [1.2.3-4]", Style::default().fg(app.theme.accent));
    let mock_list_item = Line::from(vec![mock_icon, mock_name, mock_ver]);
    let mock_list_block = Block::default()
        .borders(Borders::ALL)
        .title(" Mock List ")
        .border_style(Style::default().fg(app.theme.border));
    let mock_list_widget = Paragraph::new(mock_list_item)
        .block(mock_list_block)
        .style(Style::default().bg(app.theme.background).fg(app.theme.foreground));
    f.render_widget(mock_list_widget, preview_layout[2]);

    // Mock Details
    let mut details_lines = Vec::new();
    details_lines.push(Line::from(Span::styled(" example-pkg", Style::default().fg(app.theme.foreground).add_modifier(Modifier::BOLD))));
    details_lines.push(Line::from(vec![
        Span::styled("  Repository  ", Style::default().fg(app.theme.border)),
        Span::styled("core", Style::default().fg(app.theme.accent)),
    ]));
    details_lines.push(Line::from(vec![
        Span::styled("  Description ", Style::default().fg(app.theme.border)),
        Span::styled("This is a live preview of the theme.", Style::default().fg(app.theme.foreground)),
    ]));
    let mock_details_block = Block::default()
        .borders(Borders::ALL)
        .title(" Mock Details ")
        .border_style(Style::default().fg(app.theme.border));
    let mock_details_widget = Paragraph::new(details_lines)
        .block(mock_details_block)
        .style(Style::default().bg(app.theme.background).fg(app.theme.foreground));
    f.render_widget(mock_details_widget, preview_layout[4]);

    // Mock Buttons
    let button_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(50),
            Constraint::Percentage(50),
        ])
        .split(preview_layout[6]);

    let mock_btn1 = Paragraph::new(" [i] Install ")
        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(app.theme.accent)))
        .style(Style::default().fg(app.theme.accent).add_modifier(Modifier::BOLD))
        .alignment(Alignment::Center);
    let mock_btn2 = Paragraph::new(" [r] Remove ")
        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(app.theme.error)))
        .style(Style::default().fg(app.theme.error).add_modifier(Modifier::BOLD))
        .alignment(Alignment::Center);

    f.render_widget(mock_btn1, button_layout[0]);
    f.render_widget(mock_btn2, button_layout[1]);
}