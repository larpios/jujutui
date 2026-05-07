use crate::app::{App, Focus, Mode};
use crate::theme::Theme;
use ansi_to_tui::IntoText;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, Paragraph, Wrap},
};

pub fn ui(f: &mut Frame, app: &mut App) {
    if !app.config.transparent_background {
        f.render_widget(
            Block::default().style(Style::default().bg(app.theme.bg)),
            f.area(),
        );
    }

    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // header
            Constraint::Min(0),    // log + detail + diff
            Constraint::Length(1), // status
            Constraint::Length(1), // help bar
        ])
        .split(f.area());

    render_header(f, app, outer[0]);

    match app.active_tab {
        crate::app::ActiveTab::Log => {
            let main_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
                .split(outer[1]);

            let right_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(10), // detail
                    Constraint::Min(0),     // diff
                ])
                .split(main_chunks[1]);

            render_log(f, app, main_chunks[0]);
            render_detail(f, app, right_chunks[0]);
            render_diff(f, app, right_chunks[1]);
        }
        crate::app::ActiveTab::Status => {
            render_status_tab(f, app, outer[1]);
        }
    }

    render_status(f, app, outer[2]);
    render_help_bar(f, app, outer[3]);

    // Overlays — drawn last so they appear on top
    match &app.mode {
        Mode::CommandPalette => render_input_overlay(
            f,
            &app.theme,
            " Command Palette ",
            &app.command_input.clone(),
            app.theme.help_key,
            "commands: a(abandon) · s(squash) · n(new) · e(edit) · d(describe) · undo · fetch · push · absorb · discard · split",
        ),
        Mode::Describe => render_input_overlay(
            f,
            &app.theme,
            " Describe Revision ",
            &app.describe_input.clone(),
            app.theme.border_focused,
            "Enter to confirm · Esc to cancel",
        ),
        Mode::Confirm(action) => render_confirm_overlay(f, &app.theme, action),
        _ => {}
    }

    if let Some(output) = &app.command_output {
        render_output_popup(f, &app.theme, output);
    }

    if app.show_help {
        render_help_overlay(f, app);
    }
}

fn render_confirm_overlay(f: &mut Frame, t: &Theme, action: &crate::app::PendingAction) {
    let area = centered_rect(60, 10, f.area());
    f.render_widget(Clear, area);
    let block = Block::default()
        .title(" Confirm Action ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(t.conflict))
        .style(Style::default().bg(t.bg));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let text = vec![
        Line::from(vec![
            Span::styled(
                "Warning: ",
                Style::default().fg(t.conflict).add_modifier(Modifier::BOLD),
            ),
            Span::raw(format!("You are about to {} an ", action.name())),
            Span::styled(
                "immutable",
                Style::default()
                    .fg(t.immutable)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" revision."),
        ]),
        Line::raw(""),
        Line::from(vec![
            Span::raw("Proceed? "),
            Span::styled(
                "[y] Yes",
                Style::default()
                    .fg(t.status_ok)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" / "),
            Span::styled(
                "[n] No",
                Style::default()
                    .fg(t.status_err)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
    ];

    f.render_widget(
        Paragraph::new(text)
            .alignment(Alignment::Center)
            .style(Style::default().fg(t.desc)),
        inner,
    );
}

fn render_output_popup(f: &mut Frame, t: &Theme, output: &str) {
    let area = centered_rect(80, 80, f.area());
    f.render_widget(Clear, area);
    let block = Block::default()
        .title(" Command Output ")
        .title_bottom(Line::from(Span::styled(
            " Press any key to close ",
            Style::default()
                .fg(t.immutable)
                .add_modifier(Modifier::ITALIC),
        )))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(t.border_focused))
        .style(Style::default().bg(t.bg));
    let inner = block.inner(area);
    f.render_widget(block, area);
    f.render_widget(
        Paragraph::new(output)
            .style(Style::default().fg(t.desc))
            .wrap(Wrap { trim: false }),
        inner,
    );
}

fn render_header(f: &mut Frame, app: &App, area: Rect) {
    let t = &app.theme;
    let wc = app.revisions.iter().find(|r| r.is_working_copy);
    let mut spans = vec![
        Span::styled(
            " jjtui ",
            Style::default()
                .fg(Color::Black)
                .bg(t.working_copy)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
    ];
    if let Some(wc) = wc {
        let desc = wc.description.lines().next().unwrap_or("(no description)");
        spans.push(Span::styled(
            "@ ",
            Style::default()
                .fg(t.working_copy)
                .add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::styled(
            wc.change_id.clone(),
            Style::default().fg(t.change_id),
        ));
        spans.push(Span::raw("  "));
        spans.push(Span::styled(
            desc,
            Style::default().fg(t.desc).add_modifier(Modifier::ITALIC),
        ));
    } else {
        spans.push(Span::styled(
            "no working copy",
            Style::default().fg(t.immutable),
        ));
    }
    f.render_widget(
        Paragraph::new(Line::from(spans)).style(Style::default().bg(t.header_bg)),
        area,
    );
}

fn render_log(f: &mut Frame, app: &mut App, area: Rect) {
    let t = &app.theme;
    let mode = &app.mode;
    let in_selection_mode = matches!(mode, Mode::RebaseTarget | Mode::SquashTarget);
    let focused = app.focus == Focus::Log || in_selection_mode;

    let title = if matches!(mode, Mode::RebaseTarget) {
        " Rebase — choose destination "
    } else if matches!(mode, Mode::SquashTarget) {
        " Squash — choose target "
    } else {
        " Log "
    };

    let border_color = if in_selection_mode {
        t.rebase_src
    } else if focused {
        t.border_focused
    } else {
        t.border_unfocused
    };

    let items: Vec<ListItem> = app
        .revisions
        .iter()
        .map(|rev| {
            let is_sel = app.selected_revisions.contains(&rev.change_id);
            let is_src = app.rebase_source.as_deref() == Some(rev.change_id.as_str());
            revision_item(rev, is_sel, is_src, t, area.width)
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(border_color))
                .style(Style::default().bg(t.bg)),
        )
        .highlight_style(
            Style::default()
                .bg(t.border_unfocused)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ")
        .style(Style::default().bg(t.bg));

    f.render_stateful_widget(list, area, &mut app.list_state);
}

fn render_detail(f: &mut Frame, app: &App, area: Rect) {
    let t = &app.theme;
    let rev = app.current_rev();

    let block = Block::default()
        .title(" Revision Details ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(t.border_unfocused))
        .style(Style::default().bg(t.bg));

    let content = if let Some(rev) = rev {
        let mut lines = vec![
            Line::from(vec![
                Span::styled("Change ID: ", Style::default().fg(t.immutable)),
                Span::styled(&rev.change_id, Style::default().fg(t.change_id)),
                Span::raw("  "),
                Span::styled("Commit ID: ", Style::default().fg(t.immutable)),
                Span::styled(&rev.commit_id, Style::default().fg(t.change_id)),
            ]),
            Line::from(vec![
                Span::styled("Author:    ", Style::default().fg(t.immutable)),
                Span::styled(&rev.author, Style::default().fg(t.author)),
            ]),
            Line::raw(""),
        ];

        for line in rev.description.lines() {
            lines.push(Line::from(Span::styled(line, Style::default().fg(t.desc))));
        }

        lines
    } else {
        vec![Line::raw("  (no revision selected)")]
    };

    f.render_widget(
        Paragraph::new(content)
            .block(block)
            .wrap(Wrap { trim: false })
            .style(Style::default().bg(t.bg)),
        area,
    );
}

fn render_diff(f: &mut Frame, app: &App, area: Rect) {
    let t = &app.theme;
    let border_color = if app.focus == Focus::Diff {
        t.border_focused
    } else {
        t.border_unfocused
    };

    let block = Block::default()
        .title(" Diff ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color))
        .style(Style::default().bg(t.bg));

    let content = if app.current_diff.is_empty() {
        ratatui::text::Text::raw("  (no changes)")
    } else {
        app.current_diff
            .as_bytes()
            .into_text()
            .unwrap_or_else(|_| ratatui::text::Text::raw(app.current_diff.clone()))
    };

    f.render_widget(
        Paragraph::new(content)
            .block(block)
            .wrap(Wrap { trim: false })
            .scroll((app.diff_scroll, 0))
            .style(Style::default().bg(t.bg)),
        area,
    );
}

fn render_status(f: &mut Frame, app: &App, area: Rect) {
    let t = &app.theme;
    let style = if app.status_is_error {
        Style::default().fg(t.status_err)
    } else {
        Style::default().fg(t.status_ok)
    };
    f.render_widget(
        Paragraph::new(format!(" {}", app.status_message)).style(style.bg(t.bg)),
        area,
    );
}

fn render_help_bar(f: &mut Frame, app: &App, area: Rect) {
    let t = &app.theme;
    let text = match &app.mode {
        Mode::Normal if app.active_tab == crate::app::ActiveTab::Status => {
            " [Tab] log  [Space] select  [s] squash into parent  [S/c] squash into target  [x] discard  [q] quit"
        }
        Mode::Normal if app.focus == Focus::Diff => {
            " [?] help  [j/k] scroll  [Ctrl+D/U] page  [h] back to log"
        }
        Mode::Normal => {
            " [?] help  [Tab] status  [:] commands  [l] diff  [f/p] fetch/push  [q] quit"
        }
        Mode::CommandPalette => " [Enter] execute  [Esc] cancel",
        Mode::Describe => " [Enter] confirm  [Esc] cancel",
        Mode::RebaseTarget => " [j/k] choose destination  [Enter] rebase here  [Esc] cancel",
        Mode::SquashTarget => " [j/k] choose target  [Enter] squash here  [Esc] cancel",
        Mode::Confirm(_) => " [y] confirm  [n/Esc] cancel",
    };
    f.render_widget(
        Paragraph::new(text).style(Style::default().fg(t.immutable).bg(t.bg)),
        area,
    );
}

fn render_status_tab(f: &mut Frame, app: &mut App, area: Rect) {
    let t = &app.theme;
    let rev = app.current_rev();
    let title = if let Some(rev) = rev {
        format!(
            " Files in {} ",
            &rev.change_id[..8.min(rev.change_id.len())]
        )
    } else {
        " Files ".to_string()
    };

    let items: Vec<ListItem> = app
        .status_files
        .iter()
        .map(|file| {
            let is_sel = app.selected_files.contains(&file.path);
            let style = if is_sel {
                Style::default().fg(t.selected).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(t.desc)
            };

            let status_style = match file.status.as_str() {
                "A" => Style::default().fg(t.status_ok),
                "M" => Style::default().fg(t.change_id),
                "D" => Style::default().fg(t.status_err),
                _ => Style::default().fg(t.immutable),
            };

            ListItem::new(Line::from(vec![
                Span::styled(if is_sel { " ● " } else { " ○ " }, style),
                Span::styled(format!("{:<2} ", file.status), status_style),
                Span::styled(&file.path, Style::default().fg(t.desc)),
            ]))
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(t.border_focused))
                .style(Style::default().bg(t.bg)),
        )
        .highlight_style(
            Style::default()
                .bg(t.border_unfocused)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ")
        .style(Style::default().bg(t.bg));

    f.render_stateful_widget(list, area, &mut app.status_list_state);
}

fn render_input_overlay(
    f: &mut Frame,
    t: &Theme,
    title: &str,
    input: &str,
    color: Color,
    hint: &str,
) {
    let area = centered_rect(64, 5, f.area());
    f.render_widget(Clear, area);
    let block = Block::default()
        .title(title)
        .title_bottom(Line::from(Span::styled(
            format!(" {hint} "),
            Style::default().fg(t.immutable),
        )))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(color))
        .style(Style::default().bg(t.bg));
    let inner = block.inner(area);
    f.render_widget(block, area);
    f.render_widget(
        Paragraph::new(format!("{input}█")).style(Style::default().fg(t.desc)),
        inner,
    );
}

fn render_help_overlay(f: &mut Frame, app: &App) {
    let t = &app.theme;
    let area = centered_rect(72, 28, f.area());
    f.render_widget(Clear, area);

    let block = Block::default()
        .title(" Keybindings ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(t.border_focused))
        .style(Style::default().bg(t.bg));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let key = |k: &'static str| {
        Span::styled(
            format!("{k:<12}"),
            Style::default().fg(t.help_key).add_modifier(Modifier::BOLD),
        )
    };
    let desc = |d: &'static str| Span::styled(format!("{d:<26}"), Style::default().fg(t.desc));
    let sec = |s: &'static str| {
        Span::styled(
            s,
            Style::default()
                .fg(t.help_section)
                .add_modifier(Modifier::BOLD),
        )
    };
    let blank = Line::raw("");
    let row = |k1: &'static str, d1: &'static str, k2: &'static str, d2: &'static str| {
        Line::from(vec![
            Span::raw("  "),
            key(k1),
            desc(d1),
            Span::raw("  "),
            key(k2),
            desc(d2),
        ])
    };

    let lines: Vec<Line> = vec![
        blank.clone(),
        Line::from(vec![
            Span::raw("  "),
            sec("Navigation              "),
            Span::raw("  "),
            sec("Log Operations"),
        ]),
        row("j / k", "move up / down", "n", "new revision"),
        row("g / G", "top / bottom", "e / Ent", "edit (working copy)"),
        row("l / h", "focus diff / log", "d", "describe commit"),
        row("Tab", "switch Log/Status", "a", "abandon"),
        row("Space / v", "toggle select", "s", "squash selected"),
        row("Esc", "clear selection", "S", "squash cursor"),
        row("", "", "D", "duplicate"),
        row("f / p", "git fetch / push", "r", "rebase (pick target)"),
        blank.clone(),
        Line::from(vec![
            Span::raw("  "),
            sec("Status Tab             "),
            Span::raw("  "),
            sec("General"),
        ]),
        row("Space", "select file", "u", "undo last operation"),
        row("s", "squash into parent", "R", "refresh log"),
        row("S / c", "squash into target", ":", "command palette"),
        row("x", "discard (restore)", "?", "toggle this help"),
        row("", "", "T", "cycle theme"),
        row("", "", "q", "quit"),
        blank.clone(),
        Line::from(vec![
            Span::raw("  "),
            Span::styled("Theme: ", Style::default().fg(t.immutable)),
            Span::styled(
                app.theme.name,
                Style::default().fg(t.help_key).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "  (config: ~/.config/jujutui/config.toml)",
                Style::default().fg(t.immutable),
            ),
        ]),
        blank.clone(),
        Line::from(vec![
            Span::raw("  "),
            Span::styled(
                "Press any key to close",
                Style::default()
                    .fg(t.immutable)
                    .add_modifier(Modifier::ITALIC),
            ),
        ]),
    ];

    f.render_widget(
        Paragraph::new(lines).style(Style::default().bg(t.bg)),
        inner,
    );
}

// ── List item ─────────────────────────────────────────────────────────────────

fn revision_item<'a>(
    rev: &crate::jj::Revision,
    is_selected: bool,
    is_rebase_src: bool,
    t: &Theme,
    width: u16,
) -> ListItem<'a> {
    let (icon, icon_style) = if is_rebase_src {
        (
            "↕",
            Style::default()
                .fg(t.rebase_src)
                .add_modifier(Modifier::BOLD),
        )
    } else if rev.has_conflict {
        (
            "✗",
            Style::default().fg(t.conflict).add_modifier(Modifier::BOLD),
        )
    } else if rev.is_working_copy {
        (
            "◉",
            Style::default()
                .fg(t.working_copy)
                .add_modifier(Modifier::BOLD),
        )
    } else if is_selected {
        (
            "●",
            Style::default().fg(t.selected).add_modifier(Modifier::BOLD),
        )
    } else if rev.is_immutable {
        ("◆", Style::default().fg(t.immutable))
    } else if rev.is_empty {
        ("◦", Style::default().fg(t.empty))
    } else {
        ("○", Style::default().fg(Color::White))
    };

    let id_fg = if is_rebase_src {
        t.rebase_src
    } else if is_selected {
        t.selected
    } else if rev.is_immutable {
        t.immutable
    } else {
        t.change_id
    };

    let author_fg = if rev.is_immutable {
        t.immutable
    } else {
        t.author
    };

    let desc_fg = if rev.is_immutable {
        t.immutable
    } else if is_selected || is_rebase_src {
        t.selected
    } else if rev.is_empty {
        t.empty
    } else {
        t.desc
    };

    let first_line = rev.description.lines().next().unwrap_or("(no description)");

    // Calculate available width for description
    // icon (3) + id (13) + author (15) + spacing
    let reserved = 3 + 13 + 15 + 4;
    let available_width = width.saturating_sub(reserved as u16) as usize;

    let desc: String = if first_line.chars().count() > available_width {
        format!(
            "{}…",
            first_line
                .chars()
                .take(available_width.saturating_sub(1))
                .collect::<String>()
        )
    } else {
        first_line.to_string()
    };

    let author_short: String = rev.author.chars().take(14).collect();

    let line = Line::from(vec![
        Span::styled(format!(" {icon} "), icon_style),
        Span::styled(
            format!("{:<12} ", rev.change_id),
            Style::default().fg(id_fg),
        ),
        Span::styled(
            format!("{:<14} ", author_short),
            Style::default().fg(author_fg),
        ),
        Span::styled(desc, Style::default().fg(desc_fg)),
    ]);

    ListItem::new(line)
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn centered_rect(percent_x: u16, height: u16, r: Rect) -> Rect {
    let vert = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length((r.height.saturating_sub(height)) / 2),
            Constraint::Length(height),
            Constraint::Min(0),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vert[1])[1]
}
