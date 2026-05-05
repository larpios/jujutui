use crate::jj::Revision;
use crate::theme::Theme;
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::widgets::ListState;
use std::collections::HashSet;

#[derive(PartialEq)]
pub enum Mode {
    Normal,
    Describe,
    RebaseTarget,
    CommandPalette,
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
    pub status_message: String,
    pub status_is_error: bool,
    pub current_diff: String,
    pub diff_scroll: u16,
    pub mode: Mode,
    pub focus: Focus,
    pub command_input: String,
    pub describe_input: String,
    pub rebase_source: Option<String>,
    pub theme: Theme,
    pub show_help: bool,
}

impl App {
    pub fn new(theme: Theme) -> Self {
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
            status_message: "Ready  —  [?] help  [q] quit".to_string(),
            status_is_error: false,
            current_diff,
            diff_scroll: 0,
            mode: Mode::Normal,
            focus: Focus::Log,
            command_input: String::new(),
            describe_input: String::new(),
            rebase_source: None,
            theme,
            show_help: false,
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
        match self.mode {
            Mode::Normal => self.handle_normal(key, mods),
            Mode::CommandPalette => self.handle_command_palette(key),
            Mode::Describe => self.handle_describe(key),
            Mode::RebaseTarget => self.handle_rebase_target(key),
        }
    }

    fn handle_normal(&mut self, key: KeyCode, mods: KeyModifiers) {
        // Global keys (always active)
        match key {
            KeyCode::Char('q') => { self.should_quit = true; return; }
            KeyCode::Char('c') if mods.contains(KeyModifiers::CONTROL) => {
                self.should_quit = true;
                return;
            }
            KeyCode::Char('?') => { self.show_help = true; return; }
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
                KeyCode::Char('j') | KeyCode::Down => self.diff_scroll = self.diff_scroll.saturating_add(3),
                KeyCode::Char('k') | KeyCode::Up   => self.diff_scroll = self.diff_scroll.saturating_sub(3),
                _ => {}
            }
            return;
        }

        // Log pane
        match key {
            KeyCode::Char('j') | KeyCode::Down => self.next(),
            KeyCode::Char('k') | KeyCode::Up   => self.previous(),
            KeyCode::Char('l') => self.focus = Focus::Diff,
            KeyCode::Char('g') => self.go_to(0),
            KeyCode::Char('G') => { let last = self.revisions.len().saturating_sub(1); self.go_to(last); }
            KeyCode::Char(' ') | KeyCode::Char('v') => self.toggle_selection(),
            KeyCode::Char('a') => self.abandon_selected(),
            KeyCode::Char('s') => self.squash_selected(),
            KeyCode::Char('n') => self.new_revision(),
            KeyCode::Char('e') => self.edit_revision(),
            KeyCode::Char('d') => self.start_describe(),
            KeyCode::Char('D') => self.duplicate_revision(),
            KeyCode::Char('r') => self.start_rebase(),
            KeyCode::Char('u') => self.undo(),
            KeyCode::Char('R') => { self.refresh_log(); self.ok("Log refreshed"); }
            _ => {}
        }
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
            KeyCode::Backspace => { self.command_input.pop(); }
            KeyCode::Char(c) => { self.command_input.push(c); }
            _ => {}
        }
    }

    fn handle_describe(&mut self, key: KeyCode) {
        match key {
            KeyCode::Enter => {
                if let Some(rev) = self.current_rev().cloned() {
                    let msg = self.describe_input.clone();
                    match crate::jj::describe(&rev.change_id, &msg) {
                        Ok(_) => self.ok("Description updated"),
                        Err(e) => self.err(e.to_string()),
                    }
                    self.refresh_log();
                }
                self.mode = Mode::Normal;
                self.describe_input.clear();
            }
            KeyCode::Esc => {
                self.mode = Mode::Normal;
                self.describe_input.clear();
                self.ok("Describe cancelled");
            }
            KeyCode::Backspace => { self.describe_input.pop(); }
            KeyCode::Char(c) => { self.describe_input.push(c); }
            _ => {}
        }
    }

    fn handle_rebase_target(&mut self, key: KeyCode) {
        match key {
            KeyCode::Enter => {
                if let Some(source) = self.rebase_source.take() {
                    if let Some(dest) = self.current_rev().map(|r| r.change_id.clone()) {
                        match crate::jj::rebase(&source, &dest) {
                            Ok(_) => self.ok(format!(
                                "Rebased {} → {}",
                                &source[..8.min(source.len())],
                                &dest[..8.min(dest.len())]
                            )),
                            Err(e) => self.err(e.to_string()),
                        }
                        self.refresh_log();
                    }
                }
                self.mode = Mode::Normal;
            }
            KeyCode::Esc => {
                self.rebase_source = None;
                self.mode = Mode::Normal;
                self.ok("Rebase cancelled");
            }
            KeyCode::Char('j') | KeyCode::Down => self.next(),
            KeyCode::Char('k') | KeyCode::Up   => self.previous(),
            _ => {}
        }
    }

    fn execute_command(&mut self, cmd: &str) {
        match cmd {
            "abandon"   | "a" => self.abandon_selected(),
            "squash"    | "s" => self.squash_selected(),
            "new"       | "n" => self.new_revision(),
            "edit"      | "e" => self.edit_revision(),
            "describe"  | "d" => self.start_describe(),
            "duplicate" | "D" => self.duplicate_revision(),
            "undo"      | "u" => self.undo(),
            "refresh"   | "r" => { self.refresh_log(); self.ok("Log refreshed"); }
            "q" | "quit"      => self.should_quit = true,
            _                 => self.err(format!("Unknown command: {cmd}")),
        }
    }

    // ── Navigation ────────────────────────────────────────────────────────────

    fn next(&mut self) {
        let len = self.revisions.len();
        if len == 0 { return; }
        let i = self.list_state.selected()
            .map_or(0, |i| if i + 1 >= len { 0 } else { i + 1 });
        self.list_state.select(Some(i));
        self.diff_scroll = 0;
        self.update_diff();
    }

    fn previous(&mut self) {
        let len = self.revisions.len();
        if len == 0 { return; }
        let i = self.list_state.selected()
            .map_or(0, |i| if i == 0 { len - 1 } else { i - 1 });
        self.list_state.select(Some(i));
        self.diff_scroll = 0;
        self.update_diff();
    }

    fn go_to(&mut self, idx: usize) {
        if !self.revisions.is_empty() {
            self.list_state.select(Some(idx.min(self.revisions.len() - 1)));
            self.diff_scroll = 0;
            self.update_diff();
        }
    }

    fn update_diff(&mut self) {
        if let Some(rev) = self.list_state.selected().and_then(|i| self.revisions.get(i)) {
            self.current_diff = crate::jj::get_diff(&rev.change_id).unwrap_or_default();
        }
    }

    pub fn current_rev(&self) -> Option<&Revision> {
        self.list_state.selected().and_then(|i| self.revisions.get(i))
    }

    fn refresh_log(&mut self) {
        self.revisions = crate::jj::get_log().unwrap_or_default();
        let len = self.revisions.len();
        if let Some(i) = self.list_state.selected() {
            if i >= len {
                self.list_state.select(if len == 0 { None } else { Some(len - 1) });
            }
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

    fn abandon_selected(&mut self) {
        let ids: Vec<String> = if self.selected_revisions.is_empty() {
            match self.current_rev() {
                Some(r) => vec![r.change_id.clone()],
                None => return,
            }
        } else {
            self.selected_revisions.iter().cloned().collect()
        };
        let n = ids.len();
        let mut errs: Vec<String> = Vec::new();
        for id in &ids {
            if let Err(e) = crate::jj::abandon(id) { errs.push(e.to_string()); }
        }
        self.selected_revisions.clear();
        self.refresh_log();
        if errs.is_empty() { self.ok(format!("Abandoned {n} revision(s)")); }
        else { self.err(errs.join("; ")); }
    }

    fn squash_selected(&mut self) {
        let ids: Vec<String> = self.selected_revisions.iter().cloned().collect();
        if ids.is_empty() { self.err("Select revisions with Space first"); return; }
        let n = ids.len();
        match crate::jj::squash(&ids) {
            Ok(_) => self.ok(format!("Squashed {n} revisions")),
            Err(e) => self.err(e.to_string()),
        }
        self.selected_revisions.clear();
        self.refresh_log();
    }

    fn new_revision(&mut self) {
        let Some(id) = self.current_rev().map(|r| r.change_id.clone()) else { return };
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
        let Some(id) = self.current_rev().map(|r| r.change_id.clone()) else { return };
        match crate::jj::edit_revision(&id) {
            Ok(_) => { self.refresh_log(); self.ok(format!("Editing {}", &id[..8.min(id.len())])); }
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
        let Some(id) = self.current_rev().map(|r| r.change_id.clone()) else { return };
        match crate::jj::duplicate(&id) {
            Ok(_) => { self.refresh_log(); self.ok("Revision duplicated"); }
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
            Ok(_) => { self.refresh_log(); self.ok("Undid last operation"); }
            Err(e) => self.err(e.to_string()),
        }
    }
}
