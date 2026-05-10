mod app;
mod config;
mod jj;
mod theme;
mod ui;

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Terminal,
    backend::{Backend, CrosstermBackend},
};
use std::io;

fn main() -> Result<()> {
    let cfg = config::load_config();
    let theme_kind = theme::ThemeKind::from_slug(&cfg.theme);

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = app::App::new(theme_kind, cfg);
    let res = run_app(&mut terminal, &mut app);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    if let Err(e) = res {
        eprintln!("{e:?}");
    }
    Ok(())
}

fn run_app<B: Backend + std::io::Write>(
    terminal: &mut Terminal<B>,
    app: &mut app::App,
) -> Result<()>
where
    <B as Backend>::Error: Send + Sync + 'static,
{
    loop {
        terminal.draw(|f| ui::ui(f, app))?;
        if app.should_quit {
            break;
        }

        if let Some(sync) = app.pending_git_sync.take() {
            app.git_sync(sync);
            continue;
        }

        if app.pending_absorb {
            app.pending_absorb = false;
            app.execute_pending_action(crate::app::PendingAction::Absorb, false);
            continue;
        }

        if let Some(args) = app.pending_interactive_command.take() {
            let args_ref: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
            disable_raw_mode()?;
            execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
            let res = jj::run_interactive(&args_ref);
            enable_raw_mode()?;
            execute!(terminal.backend_mut(), EnterAlternateScreen)?;
            terminal.clear()?;
            if let Err(e) = res {
                app.err(format!("Interactive command failed: {e}"));
            } else {
                app.ok("Interactive command completed");
            }
        }

        if event::poll(std::time::Duration::from_millis(16))?
            && let Event::Key(key) = event::read()?
            && key.kind == KeyEventKind::Press
        {
            app.on_key(key.code, key.modifiers);
        }
    }
    Ok(())
}
