# jutui (Jujutsu TUI)

A Rust-based Terminal User Interface for the [Jujutsu (jj)](https://github.com/martinvonz/jj) version control system.

`jutui` is designed to provide a smooth, visual experience for managing your `jj` workflow, inspired by tools like `lazygit` and `jujutsu.nvim`.

## Features

- **Interactive Log View:** Navigate your commit graph with ease.
- **Non-Adjacent Multi-Selection:** Select multiple, non-adjacent revisions to perform batch operations.
- **Rich Diff View:** Integrated, syntax-highlighted diffs using `jj`'s own color output.
- **Vim-like Keybindings:** Fast navigation and operation using familiar keys.
- **Command Palette:** Quick discovery and execution of `jj` commands.
- **Visual Feedback:** Distinct styling for working copy, immutable, empty, and conflicted revisions.

## Installation

### Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (latest stable)
- [Jujutsu (jj)](https://github.com/martinvonz/jj#installation) installed and in your PATH.

### Building from source

```bash
cargo install --git https://github.com/larpios/jutui
```

## Usage

Run `jutui` within any directory managed by Jujutsu:

```bash
jutui
```

### Keybindings

| Key | Action |
|-----|--------|
| `j` / `Down` | Navigate down the log |
| `k` / `Up` | Navigate up the log |
| `Space` / `v` | Toggle selection of the current revision |
| `a` | Abandon highlighted or selected revisions |
| `s` | Squash selected revisions |
| `:` | Open Command Palette |
| `q` | Quit |

### Command Palette

Press `:` to enter the command palette. Supported commands:
- `abandon`: Abandon selected revisions.
- `squash`: Squash selected revisions.
- `quit` or `q`: Exit the application.

## Project Structure

- `src/main.rs`: Core TUI application loop, state, and rendering logic.
- `src/jj.rs`: Wrapper for interacting with the `jj` CLI.

## License

MIT
