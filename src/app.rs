use crate::config::Config;
use crate::jj::Revision;
use crate::theme::{Theme, ThemeKind};
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::widgets::ListState;
use std::collections::HashSet;

#[derive(PartialEq, Clone)]
pub enum Mode {
    Normal,
    Describe,
    RebaseTarget,
    CommandPalette,
    SquashTarget,
    BookmarkMenu(usize),
    BookmarkList {
        action: BookmarkAction,
        state: ListState,
        bookmarks: Vec<String>,
    },
    BookmarkPrompt {
        action: BookmarkAction,
        input: String,
        target_bookmark: Option<String>,
    },
    Confirm(PendingAction),
}

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum BookmarkAction {
    Create,
    Delete,
    Rename,
    Move,
}

impl BookmarkAction {
    pub fn name(&self) -> &str {
        match self {
            Self::Create => "create",
            Self::Delete => "delete",
            Self::Rename => "rename",
            Self::Move => "move",
        }
    }
}

#[derive(PartialEq, Clone, Debug)]
pub enum PendingAction {
    Abandon(Vec<String>),
    Squash {
        revisions: Vec<String>,
        target: Option<String>,
        files: Vec<String>,
    },
    Describe {
        revision: String,
        message: String,
    },
    Rebase {
        source: String,
        destination: String,
    },
    Absorb,
    DiscardFiles(Vec<String>),
}

impl PendingAction {
    pub fn name(&self) -> &str {
        match self {
            Self::Abandon(_) => "abandon",
            Self::Squash { .. } => "squash",
            Self::Describe { .. } => "describe",
            Self::Rebase { .. } => "rebase",
            Self::Absorb => "absorb",
            Self::DiscardFiles(_) => "discard (restore)",
        }
    }
}

#[derive(PartialEq, Clone, Copy)]
pub enum ActiveTab {
    Log,
    Status,
}

#[derive(PartialEq)]
pub enum Focus {
    Log,
    Diff,
}

pub struct App {
    pub should_quit: bool,
    pub revisions: Vec<Revision>,
    pub list_state: ListState,
    pub selected_revisions: HashSet<String>,
    pub active_tab: ActiveTab,
    pub status_files: Vec<crate::jj::StatusFile>,
    pub status_list_state: ListState,
    pub selected_files: HashSet<String>,
    pub status_message: String,
    pub status_is_error: bool,
    pub current_diff: String,
    pub diff_scroll: u16,
    pub mode: Mode,
    pub focus: Focus,
    pub command_input: String,
    pub describe_input: String,
    pub rebase_source: Option<String>,
    pub command_output: Option<String>,
    pub theme: Theme,
    pub theme_kind: ThemeKind,
    pub config: Config,
    pub show_help: bool,
    pub pending_interactive_command: Option<Vec<String>>,
    pub pending_git_sync: Option<bool>,
    pub pending_absorb: bool,
    pub squash_source: Option<(Vec<String>, Vec<String>)>,
    #[cfg(test)]
    pub last_action: Option<PendingAction>,
}

impl App {
    pub fn new(theme_kind: ThemeKind, config: Config) -> Self {
        let mut theme = theme_kind.theme();
        theme.override_from_config(&config.colors);
        let revisions = crate::jj::get_log().unwrap_or_default();
        let mut list_state = ListState::default();
        let mut current_diff = String::new();
        if !revisions.is_empty() {
            list_state.select(Some(0));
            current_diff = crate::jj::get_diff(&revisions[0].change_id).unwrap_or_default();
        }
        Self {
            should_quit: false,
            revisions,
            list_state,
            selected_revisions: HashSet::new(),
            active_tab: ActiveTab::Log,
            status_files: Vec::new(),
            status_list_state: ListState::default(),
            selected_files: HashSet::new(),
            status_message: "Ready  —  [?] help  [q] quit".to_string(),
            status_is_error: false,
            current_diff,
            diff_scroll: 0,
            mode: Mode::Normal,
            focus: Focus::Log,
            command_input: String::new(),
            describe_input: String::new(),
            rebase_source: None,
            command_output: None,
            theme,
            theme_kind,
            config,
            show_help: false,
            pending_interactive_command: None,
            pending_git_sync: None,
            pending_absorb: false,
            squash_source: None,
            #[cfg(test)]
            last_action: None,
        }
    }

    pub fn ok(&mut self, msg: impl Into<String>) {
        self.status_message = msg.into();
        self.status_is_error = false;
    }

    pub fn err(&mut self, msg: impl Into<String>) {
        self.status_message = msg.into();
        self.status_is_error = true;
    }

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
            Mode::Confirm(_) => self.handle_confirm(key),
        }
    }

    fn handle_normal(&mut self, key: KeyCode, mods: KeyModifiers) {
        // Global keys (always active)
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
            // Ctrl+D / Ctrl+U scroll diff regardless of which pane is focused
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

        // Diff pane: j/k scroll, h returns to log
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

        // Log pane
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
                self.pending_git_sync = Some(true);
            }
            KeyCode::Char('p') => {
                self.ok("Pushing to remote...");
                self.pending_git_sync = Some(false);
            }
            _ => {}
        }
    }

    fn handle_status(&mut self, key: KeyCode, _mods: KeyModifiers) {
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

    fn refresh_status(&mut self) {
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

    fn discard_files(&mut self) {
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

    fn start_squash_selected_to_target(&mut self) {
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

    fn start_squash_to_target(&mut self) {
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

    fn squash_files(&mut self) {
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

    fn handle_command_palette(&mut self, key: KeyCode) {
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

    fn handle_describe(&mut self, key: KeyCode) {
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

    fn handle_rebase_target(&mut self, key: KeyCode) {
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

    fn handle_squash_target(&mut self, key: KeyCode) {
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

    fn handle_bookmark_menu(&mut self, key: KeyCode) {
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
                    0 => BookmarkAction::Create,
                    1 => BookmarkAction::Move,
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

    fn start_bookmark_action(&mut self, action: BookmarkAction) {
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

    fn handle_bookmark_list(&mut self, key: KeyCode) {
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

    fn handle_bookmark_prompt(&mut self, key: KeyCode) {
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

    fn handle_confirm(&mut self, key: KeyCode) {
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

    fn execute_command(&mut self, cmd: &str) {
        let parts: Vec<&str> = cmd.split_whitespace().collect();
        if parts.is_empty() {
            return;
        }

        match parts[0] {
            "abandon" | "a" => self.abandon_selected(),
            "squash" | "s" => self.squash_selected(),
            "new" | "n" => self.new_revision(),
            "edit" | "e" => self.edit_revision(),
            "describe" | "d" => self.start_describe(),
            "duplicate" | "D" => self.duplicate_revision(),
            "undo" | "u" => self.undo(),
            "refresh" | "r" => {
                self.refresh_log();
                self.ok("Log refreshed");
            }
            "fetch" => {
                self.ok("Fetching from remote...");
                self.pending_git_sync = Some(true);
            }
            "push" => {
                self.ok("Pushing to remote...");
                self.pending_git_sync = Some(false);
            }
            "absorb" => {
                self.ok("Absorbing changes...");
                self.pending_absorb = true;
            }
            "discard" => self.discard_files(),
            "split" => {
                let id = self
                    .current_rev()
                    .map(|r| r.change_id.clone())
                    .unwrap_or_else(|| "@".to_string());
                let args = crate::jj::split_args(&id);
                self.run_interactive(&args.iter().map(|s| s.as_str()).collect::<Vec<_>>());
            }
            "q" | "quit" => self.should_quit = true,
            "squash-to" if parts.len() > 1 => {
                let target = parts[1].to_string();
                let files: Vec<String> = self.selected_files.iter().cloned().collect();
                let src = "@".to_string();
                self.perform_or_confirm(PendingAction::Squash {
                    revisions: vec![src],
                    target: Some(target),
                    files,
                });
            }
            _ => match crate::jj::run_command(cmd) {
                Ok(out) => {
                    self.command_output = Some(out);
                    self.refresh_log();
                }
                Err(e) => self.err(e.to_string()),
            },
        }
    }

    // ── Navigation ────────────────────────────────────────────────────────────

    fn next(&mut self) {
        let len = self.revisions.len();
        if len == 0 {
            return;
        }
        let i = self
            .list_state
            .selected()
            .map_or(0, |i| if i + 1 >= len { 0 } else { i + 1 });
        self.list_state.select(Some(i));
        self.diff_scroll = 0;
        self.update_diff();
    }

    fn previous(&mut self) {
        let len = self.revisions.len();
        if len == 0 {
            return;
        }
        let i = self
            .list_state
            .selected()
            .map_or(0, |i| if i == 0 { len - 1 } else { i - 1 });
        self.list_state.select(Some(i));
        self.diff_scroll = 0;
        self.update_diff();
    }

    fn go_to(&mut self, idx: usize) {
        if !self.revisions.is_empty() {
            self.list_state
                .select(Some(idx.min(self.revisions.len() - 1)));
            self.diff_scroll = 0;
            self.update_diff();
        }
    }

    fn update_diff(&mut self) {
        if let Some(rev) = self
            .list_state
            .selected()
            .and_then(|i| self.revisions.get(i))
        {
            self.current_diff = crate::jj::get_diff(&rev.change_id).unwrap_or_default();
        }
    }

    pub fn current_rev(&self) -> Option<&Revision> {
        self.list_state
            .selected()
            .and_then(|i| self.revisions.get(i))
    }

    fn refresh_log(&mut self) {
        self.revisions = crate::jj::get_log().unwrap_or_default();
        let len = self.revisions.len();
        if let Some(i) = self.list_state.selected()
            && i >= len
        {
            self.list_state
                .select(if len == 0 { None } else { Some(len - 1) });
        }
        self.update_diff();
    }

    fn toggle_selection(&mut self) {
        if let Some(id) = self.current_rev().map(|r| r.change_id.clone()) {
            if self.selected_revisions.contains(&id) {
                self.selected_revisions.remove(&id);
                self.ok(format!("Deselected {}", &id[..8.min(id.len())]));
            } else {
                self.selected_revisions.insert(id.clone());
                self.ok(format!(
                    "Selected {} ({} total)",
                    &id[..8.min(id.len())],
                    self.selected_revisions.len()
                ));
            }
        }
    }

    // ── jj operations ─────────────────────────────────────────────────────────

    fn perform_or_confirm(&mut self, action: PendingAction) {
        let is_immutable = match &action {
            PendingAction::Abandon(ids) => ids.iter().any(|id| self.is_rev_immutable(id)),
            PendingAction::Squash { target, .. } => {
                target.as_ref().is_some_and(|t| self.is_rev_immutable(t))
            }
            PendingAction::Describe { revision, .. } => self.is_rev_immutable(revision),
            PendingAction::Rebase { destination, .. } => self.is_rev_immutable(destination),
            PendingAction::Absorb => false,
            PendingAction::DiscardFiles(_) => false,
        };

        if is_immutable && self.config.warn_on_immutable {
            self.mode = Mode::Confirm(action);
        } else {
            self.execute_pending_action(action, is_immutable);
        }
    }

    fn is_rev_immutable(&self, id: &str) -> bool {
        self.revisions
            .iter()
            .find(|r| r.change_id == id || r.commit_id == id || (id == "@" && r.is_working_copy))
            .map(|r| r.is_immutable)
            .unwrap_or(false)
    }

    pub fn execute_pending_action(&mut self, action: PendingAction, ignore_immutable: bool) {
        #[cfg(test)]
        {
            let _ = ignore_immutable;
            self.last_action = Some(action);
            return;
        }

        #[cfg(not(test))]
        {
            let res = match action {
                PendingAction::Abandon(ids) => {
                    let mut errs = Vec::new();
                    for id in &ids {
                        if let Err(e) = crate::jj::abandon(id, ignore_immutable) {
                            errs.push(e.to_string());
                        }
                    }
                    if errs.is_empty() {
                        Ok(format!("Abandoned {} revision(s)", ids.len()))
                    } else {
                        Err(anyhow::anyhow!(errs.join("; ")))
                    }
                }
                PendingAction::Squash {
                    revisions,
                    target,
                    files,
                } => crate::jj::squash(&revisions, target.as_deref(), &files, ignore_immutable)
                    .map(|_| match target {
                        Some(t) => format!("Squashed into {}", t),
                        None => "Squashed into parent".to_string(),
                    }),
                PendingAction::Describe { revision, message } => {
                    crate::jj::describe(&revision, &message, ignore_immutable)
                        .map(|_| "Description updated".to_string())
                }
                PendingAction::Rebase {
                    source,
                    destination,
                } => crate::jj::rebase(&source, &destination, ignore_immutable)
                    .map(|_| format!("Rebased {} -> {}", source, destination)),
                PendingAction::Absorb => crate::jj::absorb(ignore_immutable).map(|out| {
                    self.command_output = Some(out);
                    "Absorb completed".to_string()
                }),
                PendingAction::DiscardFiles(files) => crate::jj::restore(&files, None)
                    .map(|_| format!("Discarded changes in {} files", files.len())),
            };

            match res {
                Ok(msg) => {
                    self.ok(msg);
                    self.refresh_log();
                    if self.active_tab == ActiveTab::Status {
                        self.refresh_status();
                    }
                    self.selected_revisions.clear();
                    self.selected_files.clear();
                }
                Err(e) => self.err(e.to_string()),
            }
        }
    }

    fn abandon_selected(&mut self) {
        let ids: Vec<String> = if self.selected_revisions.is_empty() {
            match self.current_rev() {
                Some(r) => vec![r.change_id.clone()],
                None => return,
            }
        } else {
            self.selected_revisions.iter().cloned().collect()
        };
        self.perform_or_confirm(PendingAction::Abandon(ids));
    }

    fn squash_selected(&mut self) {
        let ids: Vec<String> = self.selected_revisions.iter().cloned().collect();
        if ids.is_empty() {
            self.err("Select revisions with Space first");
            return;
        }
        self.perform_or_confirm(PendingAction::Squash {
            revisions: ids,
            target: None,
            files: Vec::new(),
        });
    }

    fn squash_cursor(&mut self) {
        let Some(id) = self.current_rev().map(|r| r.change_id.clone()) else {
            return;
        };
        self.perform_or_confirm(PendingAction::Squash {
            revisions: vec![id],
            target: None,
            files: Vec::new(),
        });
    }

    fn new_revision(&mut self) {
        let Some(id) = self.current_rev().map(|r| r.change_id.clone()) else {
            return;
        };
        match crate::jj::new_revision(&id) {
            Ok(_) => {
                self.refresh_log();
                if let Some(wc) = self.revisions.iter().position(|r| r.is_working_copy) {
                    self.list_state.select(Some(wc));
                    self.update_diff();
                }
                self.ok("New revision created");
            }
            Err(e) => self.err(e.to_string()),
        }
    }

    fn edit_revision(&mut self) {
        let Some(id) = self.current_rev().map(|r| r.change_id.clone()) else {
            return;
        };
        match crate::jj::edit_revision(&id) {
            Ok(_) => {
                self.refresh_log();
                self.ok(format!("Editing {}", &id[..8.min(id.len())]));
            }
            Err(e) => self.err(e.to_string()),
        }
    }

    fn start_describe(&mut self) {
        if let Some(rev) = self.current_rev() {
            self.describe_input = rev.description.clone();
            self.mode = Mode::Describe;
        }
    }

    fn duplicate_revision(&mut self) {
        let Some(id) = self.current_rev().map(|r| r.change_id.clone()) else {
            return;
        };
        match crate::jj::duplicate(&id) {
            Ok(_) => {
                self.refresh_log();
                self.ok("Revision duplicated");
            }
            Err(e) => self.err(e.to_string()),
        }
    }

    fn start_rebase(&mut self) {
        if let Some(id) = self.current_rev().map(|r| r.change_id.clone()) {
            self.rebase_source = Some(id.clone());
            self.mode = Mode::RebaseTarget;
            self.ok(format!(
                "Rebase {} — navigate to destination, Enter to confirm, Esc to cancel",
                &id[..8.min(id.len())]
            ));
        }
    }

    fn undo(&mut self) {
        match crate::jj::undo() {
            Ok(_) => {
                self.refresh_log();
                self.ok("Undid last operation");
            }
            Err(e) => self.err(e.to_string()),
        }
    }

    pub fn git_sync(&mut self, fetch: bool) {
        let res = if fetch {
            crate::jj::git_fetch()
        } else {
            crate::jj::git_push()
        };
        match res {
            Ok(out) => {
                self.command_output = Some(out);
                self.refresh_log();
                self.ok(if fetch {
                    "Git fetch completed"
                } else {
                    "Git push completed"
                });
            }
            Err(e) => self.err(e.to_string()),
        }
    }

    fn run_interactive(&mut self, args: &[&str]) {
        self.pending_interactive_command = Some(args.iter().map(|s| s.to_string()).collect());
    }

    fn cycle_theme(&mut self) {
        let all = ThemeKind::ALL;
        let current = all.iter().position(|k| *k == self.theme_kind).unwrap_or(0);
        let next = all[(current + 1) % all.len()];
        self.theme_kind = next;
        self.theme = next.theme();
        self.theme.override_from_config(&self.config.colors);
        self.ok(format!("Theme: {}", self.theme.name));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::theme::ThemeKind;

    #[test]
    fn test_squash_cursor_generates_correct_action() {
        let mut app = App::new(ThemeKind::TokyoNight, Config::default());
        let rev = Revision {
            change_id: "test-id".to_string(),
            ..Revision::default()
        };
        app.revisions = vec![rev.clone()];
        app.list_state.select(Some(0));

        app.squash_cursor();

        if let Some(PendingAction::Squash {
            revisions,
            target,
            files,
        }) = app.last_action
        {
            assert_eq!(revisions, vec!["test-id".to_string()]);
            assert_eq!(target, None);
            assert!(files.is_empty());
        } else {
            panic!("Expected Squash action, got {:?}", app.last_action);
        }
    }

    #[test]
    fn test_s_enters_squash_target_mode() {
        let mut app = App::new(ThemeKind::TokyoNight, Config::default());
        let rev = Revision {
            change_id: "test-id".to_string(),
            ..Revision::default()
        };
        app.revisions = vec![rev.clone()];
        app.list_state.select(Some(0));

        app.on_key(KeyCode::Char('s'), KeyModifiers::empty());

        assert!(matches!(app.mode, Mode::SquashTarget));
        assert_eq!(
            app.squash_source,
            Some((vec!["test-id".to_string()], vec![]))
        );
    }
}
