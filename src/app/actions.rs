use super::App;
use crate::app::{GitSync, Mode, PendingAction, PushTarget};

impl App {
    pub(super) fn execute_command(&mut self, cmd: &str) {
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
            "redo" | "U" => self.redo(),
            "refresh" | "r" => {
                self.refresh_log();
                self.ok("Log refreshed");
            }
            "fetch" => {
                self.ok("Fetching from remote...");
                self.pending_git_sync = Some(GitSync::Fetch);
            }
            "push" => self.start_push_cursor(),
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

    pub(super) fn next(&mut self) {
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

    pub(super) fn previous(&mut self) {
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

    pub(super) fn go_to(&mut self, idx: usize) {
        if !self.revisions.is_empty() {
            self.list_state
                .select(Some(idx.min(self.revisions.len() - 1)));
            self.diff_scroll = 0;
            self.update_diff();
        }
    }

    pub(super) fn update_diff(&mut self) {
        if let Some(rev) = self
            .list_state
            .selected()
            .and_then(|i| self.revisions.get(i))
        {
            self.current_diff = crate::jj::get_diff(&rev.change_id).unwrap_or_default();
        }
    }

    pub(super) fn refresh_log(&mut self) {
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

    pub(super) fn toggle_selection(&mut self) {
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

    pub(super) fn perform_or_confirm(&mut self, action: PendingAction) {
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

    pub(super) fn is_rev_immutable(&self, id: &str) -> bool {
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
                    if self.active_tab == crate::app::ActiveTab::Status {
                        self.refresh_status();
                    }
                    self.selected_revisions.clear();
                    self.selected_files.clear();
                }
                Err(e) => self.err(e.to_string()),
            }
        }
    }

    pub(super) fn abandon_selected(&mut self) {
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

    pub(super) fn squash_selected(&mut self) {
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

    pub(super) fn squash_cursor(&mut self) {
        let Some(id) = self.current_rev().map(|r| r.change_id.clone()) else {
            return;
        };
        self.perform_or_confirm(PendingAction::Squash {
            revisions: vec![id],
            target: None,
            files: Vec::new(),
        });
    }

    pub(super) fn new_revision(&mut self) {
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

    pub(super) fn edit_revision(&mut self) {
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

    pub(super) fn start_describe(&mut self) {
        if let Some(rev) = self.current_rev() {
            self.describe_input = rev.description.clone();
            self.mode = Mode::Describe;
        }
    }

    pub(super) fn duplicate_revision(&mut self) {
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

    pub(super) fn start_rebase(&mut self) {
        if let Some(id) = self.current_rev().map(|r| r.change_id.clone()) {
            self.rebase_source = Some(id.clone());
            self.mode = Mode::RebaseTarget;
            self.ok(format!(
                "Rebase {} — navigate to destination, Enter to confirm, Esc to cancel",
                &id[..8.min(id.len())]
            ));
        }
    }

    pub(super) fn undo(&mut self) {
        match crate::jj::undo() {
            Ok(_) => {
                self.refresh_log();
                self.ok("Undid last operation");
            }
            Err(e) => self.err(e.to_string()),
        }
    }

    pub(super) fn redo(&mut self) {
        match crate::jj::redo() {
            Ok(_) => {
                self.refresh_log();
                self.ok("Redid last operation");
            }
            Err(e) => self.err(e.to_string()),
        }
    }

    pub fn git_sync(&mut self, sync: GitSync) {
        let (res, success_msg): (anyhow::Result<String>, &str) = match sync {
            GitSync::Fetch => (crate::jj::git_fetch(), "Git fetch completed"),
            GitSync::Push(PushTarget::Bookmark(b)) => (
                crate::jj::git_push_bookmark(&b),
                "Git push bookmark completed",
            ),
            GitSync::Push(PushTarget::Change(c)) => {
                (crate::jj::git_push_change(&c), "Git push change completed")
            }
        };
        match res {
            Ok(out) => {
                self.command_output = Some(out);
                self.refresh_log();
                self.ok(success_msg);
            }
            Err(e) => self.err(e.to_string()),
        }
    }

    pub(super) fn run_interactive(&mut self, args: &[&str]) {
        self.pending_interactive_command = Some(args.iter().map(|s| s.to_string()).collect());
    }

    pub(super) fn cycle_theme(&mut self) {
        let all = crate::theme::ThemeKind::ALL;
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
    use crate::app::{App, Mode, PendingAction};
    use crate::config::Config;
    use crate::jj::Revision;
    use crate::theme::ThemeKind;
    use crossterm::event::{KeyCode, KeyModifiers};

    #[test]
    pub(super) fn test_squash_cursor_generates_correct_action() {
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
    pub(super) fn test_s_enters_squash_target_mode() {
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
