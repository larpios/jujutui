# jujutui (Jujutsu TUI)

A Rust-based Terminal User Interface for the [Jujutsu (jj)](https://github.com/martinvonz/jj) version control system.

`jujutui` is designed to provide a smooth, visual experience for managing your `jj` workflow, inspired by tools like `lazygit` and `jujutsu.nvim`.

## Features

- **Interactive Log View:** Navigate your commit graph with ease.
- **Non-Adjacent Multi-Selection:** Select multiple, non-adjacent revisions to perform batch operations.
- **Rich Diff View:** Integrated, syntax-highlighted diffs using `jj`'s own color output.
- **Vim-like Keybindings:** `h`/`l` to move between panes, `j`/`k` to navigate.
- **Command Palette:** Quick discovery and execution of `jj` commands via `:`.
- **Multiple Themes:** 8 built-in color themes, configured via `config.toml`.
- **Visual Feedback:** Distinct styling for working copy, immutable, empty, and conflicted revisions.

## Installation

### Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (latest stable)
- [Jujutsu (jj)](https://github.com/martinvonz/jj#installation) installed and in your PATH.

### Building from source

```bash
cargo install --git https://github.com/larpios/jujutui
```

## Usage

Run `jujutui` within any directory managed by Jujutsu:

```bash
jujutui
```

Press `?` at any time to open the keybinding reference.

### Keybindings

#### Navigation (Log pane)

| Key | Action |
|-----|--------|
| `j` / `↓` | Move down |
| `k` / `↑` | Move up |
| `g` | Jump to top |
| `G` | Jump to bottom |
| `l` | Focus diff pane |
| `Space` / `v` | Toggle selection |
| `Esc` | Clear selection |

#### Log Operations

| Key | Action |
|-----|--------|
| `n` | New revision after cursor |
| `e` | Edit (set working copy to cursor) |
| `d` | Describe — edit commit message |
| `a` | Abandon cursor or all selected |
| `s` | Squash selected into parent |
| `D` | Duplicate revision |
| `r` | Rebase — navigate to destination, Enter to confirm |
| `u` | Undo last operation |
| `R` | Refresh log |

#### Diff pane

| Key | Action |
|-----|--------|
| `j` / `↓` | Scroll down 3 lines |
| `k` / `↑` | Scroll up 3 lines |
| `Ctrl+D` | Page down (works from either pane) |
| `Ctrl+U` | Page up (works from either pane) |
| `h` | Return to log pane |

#### General

| Key | Action |
|-----|--------|
| `?` | Toggle keybinding help overlay |
| `:` | Open command palette |
| `q` | Quit |

### Command Palette

Press `:` to open the palette. Accepted commands:

| Command | Alias | Action |
|---------|-------|--------|
| `abandon` | `a` | Abandon selected |
| `squash` | `s` | Squash selected |
| `new` | `n` | New revision |
| `edit` | `e` | Edit revision |
| `describe` | `d` | Describe revision |
| `duplicate` | `D` | Duplicate revision |
| `undo` | `u` | Undo last operation |
| `refresh` | `r` | Refresh log |
| `quit` | `q` | Exit |

---

## Configuration

jujutui reads its configuration from:

```
~/.config/jujutui/config.toml
```

(`$XDG_CONFIG_HOME/jujutui/config.toml` if `XDG_CONFIG_HOME` is set.)

The file is created automatically on first run if it does not exist.

### Options

```toml
# Color theme.
# Available values:
#   catppuccin-mocha   (default) — dark, pastel, warm
#   catppuccin-latte              — light variant of Catppuccin
#   tokyo-night                   — deep blue/purple dark theme
#   dracula                       — classic purple and pink
#   gruvbox-dark                  — warm earthy tones
#   nord                          — cool arctic blues
#   one-dark                      — Atom/VS Code One Dark
#   solarized-dark                — Ethan Schoonover's classic
theme = "catppuccin-mocha"
```

Example — switching to Tokyo Night:

```toml
theme = "tokyo-night"
```

Restart `jujutui` after editing the config for changes to take effect.

---

## Project Structure

- `src/main.rs` — TUI application loop, state, rendering, theme system, config.
- `src/jj.rs` — Typed wrappers around the `jj` CLI.

## License

MIT
