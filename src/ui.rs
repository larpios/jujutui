use crate::app::{App, Focus, Mode};
use crate::theme::Theme;
use ansi_to_tui::IntoText;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, Paragraph, Wrap},
    Frame,
};

pub fn ui(f: &mut Frame, app: &mut App) {
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // header
            Constraint::Min(0),    // log + diff
            Constraint::Length(1), // status
            Constraint::Length(1), // help bar
        ])
        .split(f.area());

    render_header(f, app, outer[0]);

    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(outer[1]);

    render_log(f, app, main_chunks[0]);
    render_diff(f, app, main_chunks[1]);
    render_status(f, app, outer[2]);
    render_help_bar(f, app, outer[3]);

    // Overlays — drawn last so they appear on top
    match app.mode {
        Mode::CommandPalette => render_input_overlay(
            f, &app.theme, " Command Palette ",
            &app.command_input.clone(), app.theme.help_key,
            "commands: abandon · squash · new · edit · describe · duplicate · undo · refresh · quit",
        ),
        Mode::Describe => render_input_overlay(
            f, &app.theme, " Describe Revision ",
            &app.describe_input.clone(), app.theme.border_focused,
            "Enter to confirm · Esc to cancel",
        ),
        _ => {}
    }

    if app.show_help {
        render_help_overlay(f, app);
    }
}

fn render_header(f: &mut Frame, app: &App, area: Rect) {
    let t = &app.theme;
    let wc = app.revisions.iter().find(|r| r.is_working_copy);
    let mut spans = vec![
        Span::styled(
            " jjtui ",
            Style::default().fg(Color::Black).bg(t.working_copy).add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
    ];
    if let Some(wc) = wc {
        let desc = if wc.description.is_empty() { "(no description)" } else { &wc.description };
        spans.push(Span::styled("@ ", Style::default().fg(t.working_copy).add_modifier(Modifier::BOLD)));
        spans.push(Span::styled(wc.change_id.clone(), Style::default().fg(t.change_id)));
        spans.push(Span::raw("  "));
        spans.push(Span::styled(desc, Style::default().fg(t.desc).add_modifier(Modifier::ITALIC)));
    } else {
        spans.push(Span::styled("no working copy", Style::default().fg(t.immutable)));
    }
    f.render_widget(
        Paragraph::new(Line::from(spans)).style(Style::default().bg(t.header_bg)),
        area,
    );
}

fn render_log(f: &mut Frame, app: &mut App, area: Rect) {
    let t = &app.theme;
    let in_rebase = app.mode == Mode::RebaseTarget;
    let focused = app.focus == Focus::Log || in_rebase;

    let title = if in_rebase { " Rebase — choose destination " } else { " Log " };
    let border_color = if in_rebase {
        t.rebase_src
    } else if focused {
        t.border_focused
    } else {
        t.border_unfocused
    };

    let items: Vec<ListItem> = app.revisions.iter().map(|rev| {
        let is_sel = app.selected_revisions.contains(&rev.change_id);
        let is_src = app.rebase_source.as_deref() == Some(rev.change_id.as_str());
        revision_item(rev, is_sel, is_src, t)
    }).collect();

    let list = List::new(items)
        .block(
            Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(border_color)),
        )
        .highlight_style(Style::default().bg(t.border_unfocused).add_modifier(Modifier::BOLD))
        .highlight_symbol("▶ ");

    f.render_stateful_widget(list, area, &mut app.list_state);
}

fn render_diff(f: &mut Frame, app: &App, area: Rect) {
    let t = &app.theme;
    let border_color = if app.focus == Focus::Diff { t.border_focused } else { t.border_unfocused };

    let block = Block::default()
        .title(" Diff ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color));

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
            .scroll((app.diff_scroll, 0)),
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
        Paragraph::new(format!(" {}", app.status_message)).style(style),
        area,
    );
}

fn render_help_bar(f: &mut Frame, app: &App, area: Rect) {
    let t = &app.theme;
    let text = match app.mode {
        Mode::Normal if app.focus == Focus::Diff => {
            " [?] help  [j/k] scroll  [Ctrl+D/U] page  [h] back to log"
        }
        Mode::Normal => " [?] help  [:] commands  [l] diff  [q] quit",
        Mode::CommandPalette => " [Enter] execute  [Esc] cancel",
        Mode::Describe => " [Enter] confirm  [Esc] cancel",
        Mode::RebaseTarget => " [j/k] choose destination  [Enter] rebase here  [Esc] cancel",
    };
    f.render_widget(
        Paragraph::new(text).style(Style::default().fg(t.immutable)),
        area,
    );
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
        .border_style(Style::default().fg(color));
    let inner = block.inner(area);
    f.render_widget(block, area);
    f.render_widget(
        Paragraph::new(format!("{input}█")).style(Style::default().fg(t.desc)),
        inner,
    );
}

fn render_help_overlay(f: &mut Frame, app: &App) {
    let t = &app.theme;
    let area = centered_rect(68, 24, f.area());
    f.render_widget(Clear, area);

    let block = Block::default()
        .title(" Keybindings ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(t.border_focused));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let key  = |k: &'static str| Span::styled(format!("{k:<12}"), Style::default().fg(t.help_key).add_modifier(Modifier::BOLD));
    let desc = |d: &'static str| Span::styled(format!("{d:<28}"), Style::default().fg(t.desc));
    let sec  = |s: &'static str| Span::styled(s, Style::default().fg(t.help_section).add_modifier(Modifier::BOLD));
    let blank = Line::raw("");
    let row = |k1: &'static str, d1: &'static str, k2: &'static str, d2: &'static str| {
        Line::from(vec![Span::raw("  "), key(k1), desc(d1), Span::raw("  "), key(k2), desc(d2)])
    };

    let lines: Vec<Line> = vec![
        blank.clone(),
        Line::from(vec![Span::raw("  "), sec("Navigation              "), Span::raw("  "), sec("Log Operations")]),
        row("j / k",     "move up / down",       "n",  "new revision"),
        row("g / G",     "top / bottom",          "e",  "edit (set working copy)"),
        row("l",         "focus diff pane",        "d",  "describe commit"),
        row("Space / v", "toggle select",          "a",  "abandon"),
        row("Esc",       "clear selection",        "s",  "squash selected"),
        row("",          "",                       "D",  "duplicate"),
        row("",          "",                       "r",  "rebase (pick target)"),
        blank.clone(),
        Line::from(vec![Span::raw("  "), sec("Diff Pane (when focused)  "), Span::raw("  "), sec("General")]),
        row("j / k",     "scroll 3 lines",         "u",  "undo last operation"),
        row("Ctrl+D",    "page down",              "R",  "refresh log"),
        row("Ctrl+U",    "page up",                ":",  "command palette"),
        row("h",         "back to log pane",        "?",  "toggle this help"),
        row("",          "",                        "q",  "quit"),
        blank.clone(),
        Line::from(vec![
            Span::raw("  "),
            Span::styled("Theme: ", Style::default().fg(t.immutable)),
            Span::styled(app.theme.name, Style::default().fg(t.help_key).add_modifier(Modifier::BOLD)),
            Span::styled("  (set via ~/.config/jutui/config.toml)", Style::default().fg(t.immutable)),
        ]),
        blank.clone(),
        Line::from(vec![
            Span::raw("  "),
            Span::styled("Press any key to close", Style::default().fg(t.immutable).add_modifier(Modifier::ITALIC)),
        ]),
    ];

    f.render_widget(Paragraph::new(lines), inner);
}

// ── List item ─────────────────────────────────────────────────────────────────

fn revision_item<'a>(
    rev: &crate::jj::Revision,
    is_selected: bool,
    is_rebase_src: bool,
    t: &Theme,
) -> ListItem<'a> {
    let (icon, icon_style) = if is_rebase_src {
        ("↕", Style::default().fg(t.rebase_src).add_modifier(Modifier::BOLD))
    } else if rev.has_conflict {
        ("✗", Style::default().fg(t.conflict).add_modifier(Modifier::BOLD))
    } else if rev.is_working_copy {
        ("◉", Style::default().fg(t.working_copy).add_modifier(Modifier::BOLD))
    } else if is_selected {
        ("●", Style::default().fg(t.selected).add_modifier(Modifier::BOLD))
    } else if rev.is_immutable {
        ("◆", Style::default().fg(t.immutable))
    } else if rev.is_empty {
        ("◦", Style::default().fg(t.empty))
    } else {
        ("○", Style::default().fg(Color::White))
    };

    let id_fg = if is_rebase_src    { t.rebase_src }
                else if is_selected  { t.selected }
                else if rev.is_immutable { t.immutable }
                else { t.change_id };

    let author_fg = if rev.is_immutable { t.immutable } else { t.author };

    let desc_fg = if rev.is_immutable          { t.immutable }
                  else if is_selected || is_rebase_src { t.selected }
                  else if rev.is_empty          { t.empty }
                  else                          { t.desc };

    let raw = if rev.description.is_empty() { "(no description)" } else { &rev.description };
    let desc: String = if raw.chars().count() > 55 {
        format!("{}…", raw.chars().take(55).collect::<String>())
    } else {
        raw.to_string()
    };
    let author_short: String = rev.author.chars().take(14).collect();

    let line = Line::from(vec![
        Span::styled(format!(" {icon} "), icon_style),
        Span::styled(format!("{:<12} ", rev.change_id), Style::default().fg(id_fg)),
        Span::styled(format!("{:<14} ", author_short), Style::default().fg(author_fg)),
        Span::styled(desc, Style::default().fg(desc_fg)),
    ]);

    let bg = if is_rebase_src {
        Style::default().bg(Color::Rgb(50, 30, 0))
    } else if is_selected {
        Style::default().bg(Color::Rgb(40, 30, 0))
    } else {
        Style::default()
    };

    ListItem::new(line).style(bg)
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
