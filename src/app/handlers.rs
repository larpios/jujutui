use super::App;
use crate::app::{ActiveTab, BookmarkAction, Focus, GitSync, Mode, PendingAction, PushTarget};
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::widgets::ListState;

impl App {
    pub fn on_key(&mut self, key: KeyCode, mods: KeyModifiers) {
        if self.show_help {
            self.show_help = false;
            return;
        }
        if self.command_output.is_some() {
            self.command_output = None;
            return;
        }
        if key == KeyCode::Tab && self.mode == Mode::Normal {
            self.active_tab = match self.active_tab {
                ActiveTab::Log => ActiveTab::Status,
                ActiveTab::Status => ActiveTab::Log,
            };
            if self.active_tab == ActiveTab::Status {
                self.refresh_status();
            }
            return;
        }
        match &self.mode {
            Mode::Normal => match self.active_tab {
                ActiveTab::Log => self.handle_normal(key, mods),
                ActiveTab::Status => self.handle_status(key, mods),
            },
            Mode::CommandPalette => self.handle_command_palette(key),
            Mode::Describe => self.handle_describe(key),
            Mode::RebaseTarget => self.handle_rebase_target(key),
            Mode::SquashTarget => self.handle_squash_target(key),
            Mode::BookmarkMenu(_) => self.handle_bookmark_menu(key),
            Mode::BookmarkList { .. } => self.handle_bookmark_list(key),
            Mode::BookmarkPrompt { .. } => self.handle_bookmark_prompt(key),
            Mode::PushContextMenu(_, _) => self.handle_push_context_menu(key),
            Mode::PushBookmarkList { .. } => self.handle_push_bookmark_list(key),
            Mode::Confirm(_) => self.handle_confirm(key),
        }
    }

    pub(super) fn handle_normal(&mut self, key: KeyCode, mods: KeyModifiers) {
        match key {
            KeyCode::Char('q') => {
                self.should_quit = true;
                return;
            }
            KeyCode::Char('c') if mods.contains(KeyModifiers::CONTROL) => {
                self.should_quit = true;
                return;
            }
            KeyCode::Char('?') => {
                self.show_help = true;
                return;
            }
            KeyCode::Char(':') => {
                self.mode = Mode::CommandPalette;
                self.command_input.clear();
                return;
            }
            KeyCode::Esc => {
                self.selected_revisions.clear();
                self.ok("Selection cleared");
                return;
            }
            KeyCode::Char('d') if mods.contains(KeyModifiers::CONTROL) => {
                self.diff_scroll = self.diff_scroll.saturating_add(20);
                return;
            }
            KeyCode::Char('u') if mods.contains(KeyModifiers::CONTROL) => {
                self.diff_scroll = self.diff_scroll.saturating_sub(20);
                return;
            }
            _ => {}
        }

        if self.focus == Focus::Diff {
            match key {
                KeyCode::Char('h') => self.focus = Focus::Log,
                KeyCode::Char('j') | KeyCode::Down => {
                    self.diff_scroll = self.diff_scroll.saturating_add(3)
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    self.diff_scroll = self.diff_scroll.saturating_sub(3)
                }
                _ => {}
            }
            return;
        }

        match key {
            KeyCode::Char('j') | KeyCode::Down => self.next(),
            KeyCode::Char('k') | KeyCode::Up => self.previous(),
            KeyCode::Char('l') => self.focus = Focus::Diff,
            KeyCode::Enter => self.edit_revision(),
            KeyCode::Char('g') => self.go_to(0),
            KeyCode::Char('G') => {
                let last = self.revisions.len().saturating_sub(1);
                self.go_to(last);
            }
            KeyCode::Char(' ') | KeyCode::Char('v') => self.toggle_selection(),
            KeyCode::Char('a') => self.abandon_selected(),
            KeyCode::Char('s') if mods.contains(KeyModifiers::CONTROL) => {
                let id = self
                    .current_rev()
                    .map(|r| r.change_id.clone())
                    .unwrap_or_else(|| "@".to_string());
                let args = crate::jj::split_args(&id);
                self.run_interactive(&args.iter().map(|s| s.as_str()).collect::<Vec<_>>());
            }
            KeyCode::Char('s') => self.start_squash_selected_to_target(),
            KeyCode::Char('S') => self.squash_cursor(),
            KeyCode::Char('n') => self.new_revision(),
            KeyCode::Char('e') => self.edit_revision(),
            KeyCode::Char('d') => self.start_describe(),
            KeyCode::Char('D') => self.duplicate_revision(),
            KeyCode::Char('r') => self.start_rebase(),
            KeyCode::Char('u') => self.undo(),
            KeyCode::Char('U') => self.redo(),
            KeyCode::Char('R') => {
                self.refresh_log();
                self.ok("Log refreshed");
            }
            KeyCode::Char('b') => {
                self.mode = Mode::BookmarkMenu(0);
            }
            KeyCode::Char('T') => self.cycle_theme(),
            KeyCode::Char('f') => {
                self.ok("Fetching from remote...");
                self.pending_git_sync = Some(GitSync::Fetch);
            }
            KeyCode::Char('p') => self.start_push_cursor(),
            KeyCode::Char('P') => self.start_push_bookmark_global(),
            _ => {}
        }
    }

    pub(super) fn handle_status(&mut self, key: KeyCode, _mods: KeyModifiers) {
        match key {
            KeyCode::Char('j') | KeyCode::Down => {
                let len = self.status_files.len();
                if len == 0 {
                    return;
                }
                let i = self
                    .status_list_state
                    .selected()
                    .map_or(0, |i| if i + 1 >= len { 0 } else { i + 1 });
                self.status_list_state.select(Some(i));
            }
            KeyCode::Char('k') | KeyCode::Up => {
                let len = self.status_files.len();
                if len == 0 {
                    return;
                }
                let i = self
                    .status_list_state
                    .selected()
                    .map_or(0, |i| if i == 0 { len - 1 } else { i - 1 });
                self.status_list_state.select(Some(i));
            }
            KeyCode::Char(' ') | KeyCode::Char('v') => {
                if let Some(i) = self.status_list_state.selected() {
                    let path = self.status_files[i].path.clone();
                    if self.selected_files.contains(&path) {
                        self.selected_files.remove(&path);
                    } else {
                        self.selected_files.insert(path);
                    }
                }
            }
            KeyCode::Char('s') => self.squash_files(),
            KeyCode::Char('S') | KeyCode::Char('c') => self.start_squash_to_target(),
            KeyCode::Char('x') => self.discard_files(),
            KeyCode::Char('R') => self.refresh_status(),
            KeyCode::Esc => {
                self.selected_files.clear();
            }
            KeyCode::Char(':') => {
                self.mode = Mode::CommandPalette;
                self.command_input.clear();
            }
            KeyCode::Char('q') => self.should_quit = true,
            _ => {}
        }
    }

    pub(super) fn refresh_status(&mut self) {
        if let Some(rev) = self.current_rev() {
            match crate::jj::get_status_files(&rev.change_id) {
                Ok(files) => {
                    self.status_files = files;
                    if self.status_list_state.selected().is_none() && !self.status_files.is_empty()
                    {
                        self.status_list_state.select(Some(0));
                    }
                }
                Err(e) => self.err(e.to_string()),
            }
        }
    }

    pub(super) fn discard_files(&mut self) {
        let files: Vec<String> = if self.selected_files.is_empty() {
            if let Some(i) = self.status_list_state.selected() {
                vec![self.status_files[i].path.clone()]
            } else {
                return;
            }
        } else {
            self.selected_files.iter().cloned().collect()
        };
        self.perform_or_confirm(PendingAction::DiscardFiles(files));
    }

    pub(super) fn start_squash_selected_to_target(&mut self) {
        let ids: Vec<String> = if self.selected_revisions.is_empty() {
            match self.current_rev() {
                Some(r) => vec![r.change_id.clone()],
                None => return,
            }
        } else {
            self.selected_revisions.iter().cloned().collect()
        };
        self.squash_source = Some((ids, Vec::new()));
        self.mode = Mode::SquashTarget;
        self.active_tab = ActiveTab::Log;
        self.ok("Pick target revision for squash, Enter to confirm, Esc to cancel");
    }

    pub(super) fn start_squash_to_target(&mut self) {
        if self.selected_files.is_empty() {
            if let Some(i) = self.status_list_state.selected() {
                self.selected_files
                    .insert(self.status_files[i].path.clone());
            } else {
                self.err("Select files with Space first");
                return;
            }
        }
        let files: Vec<String> = self.selected_files.iter().cloned().collect();
        let src = self
            .revisions
            .iter()
            .find(|r| r.is_working_copy)
            .map(|r| r.change_id.clone())
            .unwrap_or_else(|| "@".to_string());

        self.squash_source = Some((vec![src], files));
        self.mode = Mode::SquashTarget;
        self.active_tab = ActiveTab::Log;
        self.ok("Pick target revision for squash, Enter to confirm, Esc to cancel");
    }

    pub(super) fn squash_files(&mut self) {
        let files: Vec<String> = self.selected_files.iter().cloned().collect();
        if files.is_empty() {
            self.err("Select files with Space first");
            return;
        }

        let Some(rev) = self.current_rev() else {
            return;
        };
        self.perform_or_confirm(PendingAction::Squash {
            revisions: vec![rev.change_id.clone()],
            target: None,
            files,
        });
    }

    pub(super) fn handle_command_palette(&mut self, key: KeyCode) {
        match key {
            KeyCode::Enter => {
                let cmd = self.command_input.trim().to_string();
                self.mode = Mode::Normal;
                self.command_input.clear();
                self.execute_command(&cmd);
            }
            KeyCode::Esc => {
                self.mode = Mode::Normal;
                self.command_input.clear();
            }
            KeyCode::Backspace => {
                self.command_input.pop();
            }
            KeyCode::Char(c) => {
                self.command_input.push(c);
            }
            _ => {}
        }
    }

    pub(super) fn handle_describe(&mut self, key: KeyCode) {
        match key {
            KeyCode::Enter => {
                if let Some(rev) = self.current_rev().cloned() {
                    let msg = self.describe_input.clone();
                    self.perform_or_confirm(PendingAction::Describe {
                        revision: rev.change_id,
                        message: msg,
                    });
                }
                self.mode = Mode::Normal;
                self.describe_input.clear();
            }
            KeyCode::Esc => {
                self.mode = Mode::Normal;
                self.describe_input.clear();
                self.ok("Describe cancelled");
            }
            KeyCode::Backspace => {
                self.describe_input.pop();
            }
            KeyCode::Char(c) => {
                self.describe_input.push(c);
            }
            _ => {}
        }
    }

    pub(super) fn handle_rebase_target(&mut self, key: KeyCode) {
        match key {
            KeyCode::Enter => {
                if let Some(source) = self.rebase_source.take()
                    && let Some(dest) = self.current_rev().map(|r| r.change_id.clone())
                {
                    self.perform_or_confirm(PendingAction::Rebase {
                        source,
                        destination: dest,
                    });
                }
                self.mode = Mode::Normal;
            }
            KeyCode::Esc => {
                self.rebase_source = None;
                self.mode = Mode::Normal;
                self.ok("Rebase cancelled");
            }
            KeyCode::Char('j') | KeyCode::Down => self.next(),
            KeyCode::Char('k') | KeyCode::Up => self.previous(),
            _ => {}
        }
    }

    pub(super) fn handle_squash_target(&mut self, key: KeyCode) {
        match key {
            KeyCode::Enter => {
                if let Some(dest) = self.current_rev().map(|r| r.change_id.clone())
                    && let Some((revisions, files)) = self.squash_source.take()
                {
                    self.perform_or_confirm(PendingAction::Squash {
                        revisions,
                        target: Some(dest),
                        files,
                    });
                }
                self.mode = Mode::Normal;
                self.selected_files.clear();
            }
            KeyCode::Esc => {
                self.squash_source = None;
                self.mode = Mode::Normal;
                self.active_tab = ActiveTab::Status;
                self.ok("Squash cancelled");
            }
            KeyCode::Char('j') | KeyCode::Down => self.next(),
            KeyCode::Char('k') | KeyCode::Up => self.previous(),
            _ => {}
        }
    }

    pub(super) fn handle_bookmark_menu(&mut self, key: KeyCode) {
        let current_sel = if let Mode::BookmarkMenu(s) = self.mode {
            s
        } else {
            0
        };
        match key {
            KeyCode::Char('j') | KeyCode::Down => {
                self.mode = Mode::BookmarkMenu((current_sel + 1) % 4);
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.mode = Mode::BookmarkMenu(if current_sel == 0 { 3 } else { current_sel - 1 });
            }
            KeyCode::Enter => {
                let action = match current_sel {
                    0 => BookmarkAction::Move,
                    1 => BookmarkAction::Create,
                    2 => BookmarkAction::Rename,
                    3 => BookmarkAction::Delete,
                    _ => return,
                };
                self.start_bookmark_action(action);
            }
            KeyCode::Esc => self.mode = Mode::Normal,
            _ => {}
        }
    }

    pub(super) fn start_bookmark_action(&mut self, action: BookmarkAction) {
        match action {
            BookmarkAction::Create => {
                self.mode = Mode::BookmarkPrompt {
                    action,
                    input: String::new(),
                    target_bookmark: None,
                };
            }
            BookmarkAction::Move | BookmarkAction::Rename | BookmarkAction::Delete => {
                let mut bookmarks: Vec<String> = self
                    .revisions
                    .iter()
                    .flat_map(|r| r.bookmarks.clone())
                    .filter(|b| !b.contains('@'))
                    .collect();
                bookmarks.sort();
                bookmarks.dedup();

                if bookmarks.is_empty() {
                    self.err("No bookmarks found");
                    self.mode = Mode::Normal;
                    return;
                }

                let mut state = ListState::default();
                state.select(Some(0));
                self.mode = Mode::BookmarkList {
                    action,
                    state,
                    bookmarks,
                };
            }
        }
    }

    pub(super) fn handle_bookmark_list(&mut self, key: KeyCode) {
        if let Mode::BookmarkList {
            action,
            ref mut state,
            ref bookmarks,
        } = self.mode
        {
            match key {
                KeyCode::Char('j') | KeyCode::Down => {
                    let i = state.selected().unwrap_or(0);
                    state.select(Some((i + 1) % bookmarks.len()));
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    let i = state.selected().unwrap_or(0);
                    state.select(Some(if i == 0 { bookmarks.len() - 1 } else { i - 1 }));
                }
                KeyCode::Enter => {
                    if let Some(i) = state.selected() {
                        let selected_bookmark = bookmarks[i].clone();
                        match action {
                            BookmarkAction::Delete => {
                                match crate::jj::bookmark_delete(&selected_bookmark) {
                                    Ok(_) => {
                                        self.ok(format!("Deleted bookmark {}", selected_bookmark));
                                        self.refresh_log();
                                        self.mode = Mode::Normal;
                                    }
                                    Err(e) => self.err(e.to_string()),
                                }
                            }
                            BookmarkAction::Move => {
                                if let Some(rev) = self.current_rev() {
                                    match crate::jj::bookmark_move(
                                        &selected_bookmark,
                                        &rev.change_id,
                                    ) {
                                        Ok(_) => {
                                            self.ok(format!(
                                                "Moved bookmark {} to {}",
                                                selected_bookmark,
                                                &rev.change_id[..8.min(rev.change_id.len())]
                                            ));
                                            self.refresh_log();
                                            self.mode = Mode::Normal;
                                        }
                                        Err(e) => self.err(e.to_string()),
                                    }
                                }
                            }
                            BookmarkAction::Rename => {
                                self.mode = Mode::BookmarkPrompt {
                                    action,
                                    input: selected_bookmark.clone(),
                                    target_bookmark: Some(selected_bookmark),
                                };
                            }
                            _ => {}
                        }
                    }
                }
                KeyCode::Esc => self.mode = Mode::Normal,
                _ => {}
            }
        }
    }

    pub(super) fn handle_bookmark_prompt(&mut self, key: KeyCode) {
        if let Mode::BookmarkPrompt {
            action,
            ref mut input,
            ref target_bookmark,
        } = self.mode
        {
            match key {
                KeyCode::Enter => {
                    let name = input.trim().to_string();
                    if name.is_empty() {
                        self.err("Bookmark name cannot be empty");
                        return;
                    }
                    let res = match action {
                        BookmarkAction::Create => {
                            if let Some(rev) = self.current_rev() {
                                crate::jj::bookmark_create(&name, &rev.change_id)
                                    .map(|_| format!("Created bookmark {}", name))
                            } else {
                                Err(anyhow::anyhow!("No revision selected"))
                            }
                        }
                        BookmarkAction::Rename => {
                            if let Some(old) = target_bookmark {
                                crate::jj::bookmark_rename(old, &name)
                                    .map(|_| format!("Renamed {} to {}", old, name))
                            } else {
                                Err(anyhow::anyhow!("No bookmark to rename"))
                            }
                        }
                        _ => Ok("".to_string()),
                    };
                    match res {
                        Ok(msg) => {
                            self.ok(msg);
                            self.refresh_log();
                            self.mode = Mode::Normal;
                        }
                        Err(e) => self.err(e.to_string()),
                    }
                }
                KeyCode::Esc => self.mode = Mode::Normal,
                KeyCode::Backspace => {
                    input.pop();
                }
                KeyCode::Char(c) => {
                    input.push(c);
                }
                _ => {}
            }
        }
    }

    pub(super) fn start_push_cursor(&mut self) {
        if !self.selected_revisions.is_empty() {
            let ids: Vec<String> = self.selected_revisions.iter().cloned().collect();
            for id in &ids {
                self.ok(format!("Pushing change {}...", &id[..8.min(id.len())]));
                self.pending_git_sync = Some(GitSync::Push(PushTarget::Change(id.clone())));
            }
            return;
        }

        let Some(rev) = self.current_rev() else {
            return;
        };
        let change_id = rev.change_id.clone();
        let bookmarks: Vec<String> = rev
            .bookmarks
            .iter()
            .filter(|b| !b.contains('@'))
            .cloned()
            .collect();

        if bookmarks.is_empty() {
            self.ok(format!("Pushing change {}...", &change_id[..8]));
            self.pending_git_sync = Some(GitSync::Push(PushTarget::Change(change_id)));
            return;
        }

        self.mode = Mode::PushContextMenu(0, bookmarks);
    }

    pub(super) fn start_push_bookmark_global(&mut self) {
        let mut bookmarks: Vec<String> = self
            .revisions
            .iter()
            .flat_map(|r| r.bookmarks.clone())
            .filter(|b| !b.contains('@'))
            .collect();
        bookmarks.sort();
        bookmarks.dedup();

        if bookmarks.is_empty() {
            self.err("No bookmarks found");
            return;
        }

        let mut state = ListState::default();
        state.select(Some(0));
        self.mode = Mode::PushBookmarkList {
            global: true,
            state,
            bookmarks,
        };
    }

    pub(super) fn handle_push_context_menu(&mut self, key: KeyCode) {
        let (current_sel, bookmarks): (usize, Vec<String>) =
            match std::mem::replace(&mut self.mode, Mode::Normal) {
                Mode::PushContextMenu(s, b) => (s, b),
                other => {
                    self.mode = other;
                    return;
                }
            };

        let num_bookmarks = bookmarks.len();
        let menu_len = if num_bookmarks == 1 {
            2
        } else {
            num_bookmarks + 1
        };

        match key {
            KeyCode::Char('j') | KeyCode::Down => {
                self.mode = Mode::PushContextMenu((current_sel + 1) % menu_len, bookmarks);
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.mode = Mode::PushContextMenu(
                    if current_sel == 0 {
                        menu_len - 1
                    } else {
                        current_sel - 1
                    },
                    bookmarks,
                );
            }
            KeyCode::Enter => {
                if num_bookmarks == 1 {
                    match current_sel {
                        0 => {
                            let b = bookmarks[0].clone();
                            self.ok(format!("Pushing bookmark {}...", b));
                            self.pending_git_sync = Some(GitSync::Push(PushTarget::Bookmark(b)));
                            self.mode = Mode::Normal;
                        }
                        1 => {
                            let change_id = self
                                .current_rev()
                                .map(|r| r.change_id.clone())
                                .unwrap_or_default();
                            self.ok(format!("Pushing change {}...", &change_id[..8]));
                            self.pending_git_sync =
                                Some(GitSync::Push(PushTarget::Change(change_id)));
                            self.mode = Mode::Normal;
                        }
                        _ => {}
                    }
                } else {
                    if current_sel < num_bookmarks {
                        let b = bookmarks[current_sel].clone();
                        self.ok(format!("Pushing bookmark {}...", b));
                        self.pending_git_sync = Some(GitSync::Push(PushTarget::Bookmark(b)));
                        self.mode = Mode::Normal;
                    } else if current_sel == num_bookmarks {
                        let change_id = self
                            .current_rev()
                            .map(|r| r.change_id.clone())
                            .unwrap_or_default();
                        self.ok(format!("Pushing change {}...", &change_id[..8]));
                        self.pending_git_sync =
                            Some(GitSync::Push(PushTarget::Change(change_id)));
                        self.mode = Mode::Normal;
                    }
                }
            }
            KeyCode::Esc => self.mode = Mode::Normal,
            _ => {}
        }
    }

    pub(super) fn handle_push_bookmark_list(&mut self, key: KeyCode) {
        if let Mode::PushBookmarkList {
            ref mut state,
            ref bookmarks,
            ..
        } = self.mode
        {
            match key {
                KeyCode::Char('j') | KeyCode::Down => {
                    let i = state.selected().unwrap_or(0);
                    state.select(Some((i + 1) % bookmarks.len()));
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    let i = state.selected().unwrap_or(0);
                    state.select(Some(if i == 0 { bookmarks.len() - 1 } else { i - 1 }));
                }
                KeyCode::Enter => {
                    if let Some(i) = state.selected() {
                        let selected_bookmark = bookmarks[i].clone();
                        self.ok(format!("Pushing bookmark {}...", selected_bookmark));
                        self.pending_git_sync =
                            Some(GitSync::Push(PushTarget::Bookmark(selected_bookmark)));
                        self.mode = Mode::Normal;
                    }
                }
                KeyCode::Esc => self.mode = Mode::Normal,
                _ => {}
            }
        }
    }

    pub(super) fn handle_confirm(&mut self, key: KeyCode) {
        let action = if let Mode::Confirm(a) = &self.mode {
            Some(a.clone())
        } else {
            None
        };
        match key {
            KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
                if let Some(a) = action {
                    self.execute_pending_action(a, true);
                }
                self.mode = Mode::Normal;
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                self.mode = Mode::Normal;
                self.ok("Action cancelled");
            }
            _ => {}
        }
    }
}
