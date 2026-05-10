use crate::config::Config;
use crate::jj::Revision;
use crate::theme::{Theme, ThemeKind};
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
    PushContextMenu(usize, Vec<String>),
    PushBookmarkList {
        global: bool,
        state: ListState,
        bookmarks: Vec<String>,
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

#[derive(PartialEq, Clone, Debug)]
pub enum PushTarget {
    Bookmark(String),
    Change(String),
}

#[derive(PartialEq, Clone, Debug)]
pub enum GitSync {
    Fetch,
    Push(PushTarget),
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
    pub pending_git_sync: Option<GitSync>,
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

    pub fn current_rev(&self) -> Option<&Revision> {
        self.list_state
            .selected()
            .and_then(|i| self.revisions.get(i))
    }
}
