mod jj;

use anyhow::Result;
use ansi_to_tui::IntoText;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use jj::Revision;
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
    Frame, Terminal,
};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::io;
use std::path::PathBuf;

// ── Theme ──────────────────────────────────────────────────────────────────────

#[derive(Clone)]
struct Theme {
    name: &'static str,
    working_copy: Color,
    immutable: Color,
    conflict: Color,
    empty: Color,
    selected: Color,
    rebase_src: Color,
    change_id: Color,
    author: Color,
    desc: Color,
    border_focused: Color,
    border_unfocused: Color,
    status_ok: Color,
    status_err: Color,
    header_bg: Color,
    header_fg: Color,
    help_key: Color,
    help_section: Color,
}

#[derive(Clone, Copy, PartialEq)]
enum ThemeKind {
    CatppuccinMocha,
    CatppuccinLatte,
    TokyoNight,
    Dracula,
    GruvboxDark,
    Nord,
    OneDark,
    SolarizedDark,
}

impl ThemeKind {
    const ALL: &'static [ThemeKind] = &[
        ThemeKind::CatppuccinMocha,
        ThemeKind::CatppuccinLatte,
        ThemeKind::TokyoNight,
        ThemeKind::Dracula,
        ThemeKind::GruvboxDark,
        ThemeKind::Nord,
        ThemeKind::OneDark,
        ThemeKind::SolarizedDark,
    ];

    fn next(self) -> ThemeKind {
        let pos = Self::ALL.iter().position(|t| *t == self).unwrap_or(0);
        Self::ALL[(pos + 1) % Self::ALL.len()]
    }

    fn slug(self) -> &'static str {
        match self {
            ThemeKind::CatppuccinMocha => "catppuccin-mocha",
            ThemeKind::CatppuccinLatte => "catppuccin-latte",
            ThemeKind::TokyoNight => "tokyo-night",
            ThemeKind::Dracula => "dracula",
            ThemeKind::GruvboxDark => "gruvbox-dark",
            ThemeKind::Nord => "nord",
            ThemeKind::OneDark => "one-dark",
            ThemeKind::SolarizedDark => "solarized-dark",
        }
    }

    fn from_slug(s: &str) -> ThemeKind {
        Self::ALL
            .iter()
            .find(|t| t.slug() == s)
            .copied()
            .unwrap_or(ThemeKind::CatppuccinMocha)
    }

    fn theme(self) -> Theme {
        match self {
            ThemeKind::CatppuccinMocha => catppuccin_mocha(),
            ThemeKind::CatppuccinLatte => catppuccin_latte(),
            ThemeKind::TokyoNight => tokyo_night(),
            ThemeKind::Dracula => dracula(),
            ThemeKind::GruvboxDark => gruvbox_dark(),
            ThemeKind::Nord => nord(),
            ThemeKind::OneDark => one_dark(),
            ThemeKind::SolarizedDark => solarized_dark(),
        }
    }
}

fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color::Rgb(r, g, b)
}

fn catppuccin_mocha() -> Theme {
    Theme {
        name: "Catppuccin Mocha",
        working_copy: rgb(148, 226, 213),  // Teal
        immutable: rgb(127, 132, 156),     // Overlay1
        conflict: rgb(243, 139, 168),      // Red
        empty: rgb(249, 226, 175),         // Yellow
        selected: rgb(249, 226, 175),      // Yellow
        rebase_src: rgb(250, 179, 135),    // Peach
        change_id: rgb(116, 199, 236),     // Sapphire
        author: rgb(203, 166, 247),        // Mauve
        desc: rgb(205, 214, 244),          // Text
        border_focused: rgb(148, 226, 213),
        border_unfocused: rgb(88, 91, 112),  // Surface2
        status_ok: rgb(166, 227, 161),     // Green
        status_err: rgb(243, 139, 168),    // Red
        header_bg: rgb(17, 17, 27),        // Crust
        header_fg: rgb(205, 214, 244),     // Text
        help_key: rgb(250, 179, 135),      // Peach
        help_section: rgb(203, 166, 247),  // Mauve
    }
}

fn catppuccin_latte() -> Theme {
    Theme {
        name: "Catppuccin Latte",
        working_copy: rgb(23, 146, 153),   // Teal
        immutable: rgb(156, 160, 176),     // Overlay1
        conflict: rgb(210, 15, 57),        // Red
        empty: rgb(223, 142, 29),          // Yellow
        selected: rgb(64, 160, 43),        // Green
        rebase_src: rgb(254, 100, 11),     // Peach
        change_id: rgb(32, 159, 181),      // Sapphire
        author: rgb(136, 57, 239),         // Mauve
        desc: rgb(76, 79, 105),            // Text
        border_focused: rgb(23, 146, 153),
        border_unfocused: rgb(188, 192, 204), // Surface2
        status_ok: rgb(64, 160, 43),       // Green
        status_err: rgb(210, 15, 57),      // Red
        header_bg: rgb(220, 224, 232),     // Crust
        header_fg: rgb(76, 79, 105),       // Text
        help_key: rgb(254, 100, 11),       // Peach
        help_section: rgb(136, 57, 239),   // Mauve
    }
}

fn tokyo_night() -> Theme {
    Theme {
        name: "Tokyo Night",
        working_copy: rgb(115, 218, 202),  // teal
        immutable: rgb(86, 95, 137),       // comment
        conflict: rgb(247, 118, 142),      // red
        empty: rgb(224, 175, 104),         // yellow
        selected: rgb(158, 206, 106),      // green
        rebase_src: rgb(255, 158, 100),    // orange
        change_id: rgb(122, 162, 247),     // blue
        author: rgb(187, 154, 247),        // purple
        desc: rgb(192, 202, 245),          // fg
        border_focused: rgb(115, 218, 202),
        border_unfocused: rgb(56, 62, 90),
        status_ok: rgb(158, 206, 106),
        status_err: rgb(247, 118, 142),
        header_bg: rgb(13, 17, 23),
        header_fg: rgb(192, 202, 245),
        help_key: rgb(255, 158, 100),
        help_section: rgb(187, 154, 247),
    }
}

fn dracula() -> Theme {
    Theme {
        name: "Dracula",
        working_copy: rgb(80, 250, 123),   // green
        immutable: rgb(98, 114, 164),      // comment
        conflict: rgb(255, 85, 85),        // red
        empty: rgb(241, 250, 140),         // yellow
        selected: rgb(255, 184, 108),      // orange
        rebase_src: rgb(255, 121, 198),    // pink
        change_id: rgb(139, 233, 253),     // cyan
        author: rgb(189, 147, 249),        // purple
        desc: rgb(248, 248, 242),          // fg
        border_focused: rgb(80, 250, 123),
        border_unfocused: rgb(68, 71, 90), // current line
        status_ok: rgb(80, 250, 123),
        status_err: rgb(255, 85, 85),
        header_bg: rgb(21, 22, 30),
        header_fg: rgb(248, 248, 242),
        help_key: rgb(255, 184, 108),
        help_section: rgb(189, 147, 249),
    }
}

fn gruvbox_dark() -> Theme {
    Theme {
        name: "Gruvbox Dark",
        working_copy: rgb(142, 192, 124),  // green
        immutable: rgb(146, 131, 116),     // gray
        conflict: rgb(251, 73, 52),        // red
        empty: rgb(250, 189, 47),          // yellow
        selected: rgb(254, 128, 25),       // orange
        rebase_src: rgb(211, 134, 155),    // pink
        change_id: rgb(131, 165, 152),     // aqua
        author: rgb(211, 134, 155),        // purple
        desc: rgb(235, 219, 178),          // fg1
        border_focused: rgb(142, 192, 124),
        border_unfocused: rgb(80, 73, 69), // bg3
        status_ok: rgb(142, 192, 124),
        status_err: rgb(251, 73, 52),
        header_bg: rgb(29, 32, 33),        // bg0_hard
        header_fg: rgb(235, 219, 178),
        help_key: rgb(254, 128, 25),
        help_section: rgb(211, 134, 155),
    }
}

fn nord() -> Theme {
    Theme {
        name: "Nord",
        working_copy: rgb(143, 188, 187),  // nord7 (teal)
        immutable: rgb(76, 86, 106),       // nord3
        conflict: rgb(191, 97, 106),       // nord11 (red)
        empty: rgb(235, 203, 139),         // nord13 (yellow)
        selected: rgb(163, 190, 140),      // nord14 (green)
        rebase_src: rgb(208, 135, 112),    // nord12 (orange)
        change_id: rgb(129, 161, 193),     // nord9 (blue)
        author: rgb(180, 142, 173),        // nord15 (purple)
        desc: rgb(236, 239, 244),          // nord6 (snow)
        border_focused: rgb(143, 188, 187),
        border_unfocused: rgb(59, 66, 82), // nord1
        status_ok: rgb(163, 190, 140),
        status_err: rgb(191, 97, 106),
        header_bg: rgb(36, 41, 51),        // nord0
        header_fg: rgb(236, 239, 244),
        help_key: rgb(208, 135, 112),
        help_section: rgb(180, 142, 173),
    }
}

fn one_dark() -> Theme {
    Theme {
        name: "One Dark",
        working_copy: rgb(86, 182, 194),   // cyan
        immutable: rgb(92, 99, 112),       // comment
        conflict: rgb(224, 108, 117),      // red
        empty: rgb(229, 192, 123),         // yellow
        selected: rgb(152, 195, 121),      // green
        rebase_src: rgb(209, 154, 102),    // orange
        change_id: rgb(97, 175, 239),      // blue
        author: rgb(198, 120, 221),        // purple
        desc: rgb(171, 178, 191),          // fg
        border_focused: rgb(86, 182, 194),
        border_unfocused: rgb(55, 59, 69), // bg2
        status_ok: rgb(152, 195, 121),
        status_err: rgb(224, 108, 117),
        header_bg: rgb(24, 26, 31),
        header_fg: rgb(171, 178, 191),
        help_key: rgb(209, 154, 102),
        help_section: rgb(198, 120, 221),
    }
}

fn solarized_dark() -> Theme {
    Theme {
        name: "Solarized Dark",
        working_copy: rgb(42, 161, 152),   // cyan
        immutable: rgb(88, 110, 117),      // base01
        conflict: rgb(220, 50, 47),        // red
        empty: rgb(181, 137, 0),           // yellow
        selected: rgb(133, 153, 0),        // green
        rebase_src: rgb(203, 75, 22),      // orange
        change_id: rgb(38, 139, 210),      // blue
        author: rgb(108, 113, 196),        // violet
        desc: rgb(131, 148, 150),          // base0
        border_focused: rgb(42, 161, 152),
        border_unfocused: rgb(7, 54, 66),  // base02
        status_ok: rgb(133, 153, 0),
        status_err: rgb(220, 50, 47),
        header_bg: rgb(0, 43, 54),         // base03
        header_fg: rgb(131, 148, 150),
        help_key: rgb(203, 75, 22),
        help_section: rgb(108, 113, 196),
    }
}

// ── Config ─────────────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Default)]
struct Config {
    #[serde(default = "default_theme_slug")]
    theme: String,
}

fn default_theme_slug() -> String {
    "catppuccin-mocha".to_string()
}

fn config_path() -> PathBuf {
    // $XDG_CONFIG_HOME/jutui/config.toml or ~/.config/jutui/config.toml
    let base = std::env::var("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
            PathBuf::from(home).join(".config")
        });
    base.join("jutui").join("config.toml")
}

fn load_config() -> Config {
    let path = config_path();
    let Ok(text) = std::fs::read_to_string(&path) else {
        return Config::default();
    };
    toml::from_str(&text).unwrap_or_default()
}

fn save_config(config: &Config) {
    let path = config_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(text) = toml::to_string(config) {
        let _ = std::fs::write(&path, text);
    }
}

// ── App state ──────────────────────────────────────────────────────────────────

#[derive(PartialEq)]
enum Mode {
    Normal,
    Describe,
    RebaseTarget,
    CommandPalette,
}

#[derive(PartialEq)]
enum Focus {
    Log,
    Diff,
}

struct App {
    should_quit: bool,
    revisions: Vec<Revision>,
    list_state: ListState,
    selected_revisions: HashSet<String>,
    status_message: String,
    status_is_error: bool,
    current_diff: String,
    diff_scroll: u16,
    mode: Mode,
    focus: Focus,
    command_input: String,
    describe_input: String,
    rebase_source: Option<String>,
    theme: Theme,
    theme_kind: ThemeKind,
    show_help: bool,
}

impl App {
    fn new(theme_kind: ThemeKind) -> Self {
        let revisions = jj::get_log().unwrap_or_default();
        let mut list_state = ListState::default();
        let mut current_diff = String::new();
        if !revisions.is_empty() {
            list_state.select(Some(0));
            current_diff = jj::get_diff(&revisions[0].change_id).unwrap_or_default();
        }
        Self {
            should_quit: false,
            revisions,
            list_state,
            selected_revisions: HashSet::new(),
            status_message: "Ready  —  [?] help  [t] cycle theme  [q] quit".to_string(),
            status_is_error: false,
            current_diff,
            diff_scroll: 0,
            mode: Mode::Normal,
            focus: Focus::Log,
            command_input: String::new(),
            describe_input: String::new(),
            rebase_source: None,
            theme: theme_kind.theme(),
            theme_kind,
            show_help: false,
        }
    }

    fn ok(&mut self, msg: impl Into<String>) {
        self.status_message = msg.into();
        self.status_is_error = false;
    }

    fn err(&mut self, msg: impl Into<String>) {
        self.status_message = msg.into();
        self.status_is_error = true;
    }

    fn cycle_theme(&mut self) {
        self.theme_kind = self.theme_kind.next();
        self.theme = self.theme_kind.theme();
        self.ok(format!("Theme: {}", self.theme.name));
        save_config(&Config { theme: self.theme_kind.slug().to_string() });
    }

    fn on_key(&mut self, key: KeyCode, mods: KeyModifiers) {
        // Help overlay intercepts all keys — any key closes it
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
        // Global keys
        match key {
            KeyCode::Tab => {
                self.focus = if self.focus == Focus::Log { Focus::Diff } else { Focus::Log };
                return;
            }
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
            KeyCode::Char('t') => { self.cycle_theme(); return; }
            // Ctrl+D / Ctrl+U always scroll diff
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

        // Diff pane focused
        if self.focus == Focus::Diff {
            match key {
                KeyCode::Char('j') | KeyCode::Down => self.diff_scroll = self.diff_scroll.saturating_add(3),
                KeyCode::Char('k') | KeyCode::Up => self.diff_scroll = self.diff_scroll.saturating_sub(3),
                _ => {}
            }
            return;
        }

        // Log pane focused
        match key {
            KeyCode::Char('j') | KeyCode::Down => self.next(),
            KeyCode::Char('k') | KeyCode::Up => self.previous(),
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
                    match jj::describe(&rev.change_id, &msg) {
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
                        match jj::rebase(&source, &dest) {
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
            KeyCode::Char('k') | KeyCode::Up => self.previous(),
            _ => {}
        }
    }

    fn execute_command(&mut self, cmd: &str) {
        match cmd {
            "abandon" | "a" => self.abandon_selected(),
            "squash" | "s" => self.squash_selected(),
            "new" | "n" => self.new_revision(),
            "edit" | "e" => self.edit_revision(),
            "describe" | "d" => self.start_describe(),
            "duplicate" | "D" => self.duplicate_revision(),
            "undo" | "u" => self.undo(),
            "refresh" | "r" => { self.refresh_log(); self.ok("Log refreshed"); }
            "q" | "quit" => self.should_quit = true,
            _ => self.err(format!("Unknown command: {cmd}")),
        }
    }

    fn next(&mut self) {
        let len = self.revisions.len();
        if len == 0 { return; }
        let i = self.list_state.selected().map_or(0, |i| if i + 1 >= len { 0 } else { i + 1 });
        self.list_state.select(Some(i));
        self.diff_scroll = 0;
        self.update_diff();
    }

    fn previous(&mut self) {
        let len = self.revisions.len();
        if len == 0 { return; }
        let i = self.list_state.selected().map_or(0, |i| if i == 0 { len - 1 } else { i - 1 });
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
        if let Some(i) = self.list_state.selected() {
            if let Some(rev) = self.revisions.get(i) {
                self.current_diff = jj::get_diff(&rev.change_id).unwrap_or_default();
            }
        }
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

    fn refresh_log(&mut self) {
        self.revisions = jj::get_log().unwrap_or_default();
        let len = self.revisions.len();
        if let Some(i) = self.list_state.selected() {
            if i >= len {
                self.list_state.select(if len == 0 { None } else { Some(len - 1) });
            }
        }
        self.update_diff();
    }

    fn current_rev(&self) -> Option<&Revision> {
        self.list_state.selected().and_then(|i| self.revisions.get(i))
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
        let n = ids.len();
        let mut errs = Vec::new();
        for id in &ids {
            if let Err(e) = jj::abandon(id) { errs.push(e.to_string()); }
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
        match jj::squash(&ids) {
            Ok(_) => self.ok(format!("Squashed {n} revisions")),
            Err(e) => self.err(e.to_string()),
        }
        self.selected_revisions.clear();
        self.refresh_log();
    }

    fn new_revision(&mut self) {
        let id = match self.current_rev().map(|r| r.change_id.clone()) { Some(id) => id, None => return };
        match jj::new_revision(&id) {
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
        let id = match self.current_rev().map(|r| r.change_id.clone()) { Some(id) => id, None => return };
        match jj::edit_revision(&id) {
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
        let id = match self.current_rev().map(|r| r.change_id.clone()) { Some(id) => id, None => return };
        match jj::duplicate(&id) {
            Ok(_) => { self.refresh_log(); self.ok("Revision duplicated"); }
            Err(e) => self.err(e.to_string()),
        }
    }

    fn start_rebase(&mut self) {
        if let Some(id) = self.current_rev().map(|r| r.change_id.clone()) {
            self.rebase_source = Some(id.clone());
            self.mode = Mode::RebaseTarget;
            self.ok(format!(
                "Rebase {}  — navigate to destination, Enter to confirm, Esc to cancel",
                &id[..8.min(id.len())]
            ));
        }
    }

    fn undo(&mut self) {
        match jj::undo() {
            Ok(_) => { self.refresh_log(); self.ok("Undid last operation"); }
            Err(e) => self.err(e.to_string()),
        }
    }
}

// ── Entry point ────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> Result<()> {
    let config = load_config();
    let theme_kind = ThemeKind::from_slug(&config.theme);

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(theme_kind);
    let res = run_app(&mut terminal, &mut app);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    if let Err(e) = res { eprintln!("{e:?}"); }
    Ok(())
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> Result<()>
where
    <B as Backend>::Error: Send + Sync + 'static,
{
    loop {
        terminal.draw(|f| ui(f, app))?;
        if app.should_quit { break; }
        if event::poll(std::time::Duration::from_millis(16))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    app.on_key(key.code, key.modifiers);
                }
            }
        }
    }
    Ok(())
}

// ── Rendering ──────────────────────────────────────────────────────────────────

fn ui(f: &mut Frame, app: &mut App) {
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
            Constraint::Length(1),
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

    // Overlays (drawn last, on top)
    match app.mode {
        Mode::CommandPalette => render_input_overlay(
            f, &app.theme, " Command Palette ",
            &app.command_input, app.theme.help_key,
            "commands: abandon · squash · new · edit · describe · duplicate · undo · refresh · quit",
        ),
        Mode::Describe => render_input_overlay(
            f, &app.theme, " Describe Revision ",
            &app.describe_input, app.theme.border_focused,
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
            " jutui ",
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
    let border_color = if in_rebase { t.rebase_src } else if focused { t.border_focused } else { t.border_unfocused };

    let items: Vec<ListItem> = app.revisions.iter().map(|rev| {
        let is_sel = app.selected_revisions.contains(&rev.change_id);
        let is_src = app.rebase_source.as_deref() == Some(&rev.change_id);
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
    let focused = app.focus == Focus::Diff;
    let border_color = if focused { t.border_focused } else { t.border_unfocused };

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
    let style = if app.status_is_error { Style::default().fg(t.status_err) } else { Style::default().fg(t.status_ok) };
    f.render_widget(
        Paragraph::new(format!(" {}", app.status_message)).style(style),
        area,
    );
}

fn render_help_bar(f: &mut Frame, app: &App, area: Rect) {
    let t = &app.theme;
    let text = match app.mode {
        Mode::Normal if app.focus == Focus::Diff => {
            " [?] help  [j/k] scroll  [Ctrl+D/U] page  [Tab] switch to log"
        }
        Mode::Normal => " [?] help  [:] commands  [t] theme  [Tab] diff  [q] quit",
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

    let key = |k: &'static str| Span::styled(format!("{k:<12}"), Style::default().fg(t.help_key).add_modifier(Modifier::BOLD));
    let desc = |d: &'static str| Span::styled(format!("{d:<28}"), Style::default().fg(t.desc));
    let blank = Line::raw("");
    let row = |k1: &'static str, d1: &'static str, k2: &'static str, d2: &'static str| {
        Line::from(vec![
            Span::raw("  "),
            key(k1), desc(d1),
            Span::raw("  "),
            key(k2), desc(d2),
        ])
    };

    let lines: Vec<Line> = vec![
        blank.clone(),
        Line::from(vec![
            Span::raw("  "),
            Span::styled("Navigation              ", Style::default().fg(t.help_section).add_modifier(Modifier::BOLD)),
            Span::raw("  "),
            Span::styled("Log Operations", Style::default().fg(t.help_section).add_modifier(Modifier::BOLD)),
        ]),
        row("j / k",      "move up / down",      "n",  "new revision"),
        row("g / G",      "top / bottom",         "e",  "edit (set working copy)"),
        row("Tab",        "switch pane",           "d",  "describe commit"),
        row("Space / v",  "toggle select",         "a",  "abandon"),
        row("Esc",        "clear selection",       "s",  "squash selected"),
        row("",           "",                      "D",  "duplicate"),
        row("",           "",                      "r",  "rebase (pick target)"),
        blank.clone(),
        Line::from(vec![
            Span::raw("  "),
            Span::styled("Diff Pane (when focused)  ", Style::default().fg(t.help_section).add_modifier(Modifier::BOLD)),
            Span::raw("  "),
            Span::styled("General", Style::default().fg(t.help_section).add_modifier(Modifier::BOLD)),
        ]),
        row("j / k",      "scroll 3 lines",        "u",  "undo last operation"),
        row("Ctrl+D",     "page down",             "R",  "refresh log"),
        row("Ctrl+U",     "page up",               ":",  "command palette"),
        row("",           "",                      "t",  "cycle theme"),
        row("",           "",                      "?",  "toggle this help"),
        row("",           "",                      "q",  "quit"),
        blank.clone(),
        Line::from(vec![
            Span::raw("  "),
            Span::styled("Theme: ", Style::default().fg(t.immutable)),
            Span::styled(app.theme.name, Style::default().fg(t.help_key).add_modifier(Modifier::BOLD)),
            Span::styled("  (press t to cycle through 8 themes)", Style::default().fg(t.immutable)),
        ]),
        blank.clone(),
        Line::from(vec![
            Span::raw("  "),
            Span::styled("Press any key to close", Style::default().fg(t.immutable).add_modifier(Modifier::ITALIC)),
        ]),
    ];

    f.render_widget(Paragraph::new(lines), inner);
}

// ── List item builder ──────────────────────────────────────────────────────────

fn revision_item<'a>(rev: &Revision, is_selected: bool, is_rebase_src: bool, t: &Theme) -> ListItem<'a> {
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

    let id_fg = if is_rebase_src { t.rebase_src } else if is_selected { t.selected } else if rev.is_immutable { t.immutable } else { t.change_id };
    let author_fg = if rev.is_immutable { t.immutable } else { t.author };
    let desc_fg = if rev.is_immutable { t.immutable } else if is_selected || is_rebase_src { t.selected } else if rev.is_empty { t.empty } else { t.desc };

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

// ── Helpers ────────────────────────────────────────────────────────────────────

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
