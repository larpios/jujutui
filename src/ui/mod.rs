mod render;
use self::render::*;

use crate::app::{App, Mode};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::Style,
    widgets::Block,
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

    match &mut app.mode {
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
        Mode::BookmarkMenu(sel) => render_bookmark_menu(f, &app.theme, sel),
        Mode::BookmarkList {
            action,
            state,
            bookmarks,
        } => render_bookmark_list(f, &app.theme, *action, state, bookmarks),
        Mode::BookmarkPrompt {
            action,
            input,
            target_bookmark,
        } => {
            let title = match action {
                crate::app::BookmarkAction::Create => " Create Bookmark ".to_string(),
                crate::app::BookmarkAction::Rename => format!(
                    " Rename Bookmark: {} ",
                    target_bookmark.as_deref().unwrap_or("")
                ),
                _ => " Bookmark Action ".to_string(),
            };
            render_input_overlay(
                f,
                &app.theme,
                &title,
                input,
                app.theme.border_focused,
                "Enter to confirm · Esc to cancel",
            )
        }
        Mode::PushContextMenu { selected, entries } => {
            render_push_context_menu(f, &app.theme, *selected, entries)
        }
        Mode::PushBookmarkList {
            state, bookmarks, ..
        } => render_push_bookmark_list(f, &app.theme, state, bookmarks),
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
