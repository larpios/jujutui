mod app;
mod config;
mod jj;
mod theme;
mod ui;

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::{Backend, CrosstermBackend}, Terminal};
use std::io;

fn main() -> Result<()> {
    let cfg = config::load_config();
    let theme = theme::ThemeKind::from_slug(&cfg.theme).theme();

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = app::App::new(theme);
    let res = run_app(&mut terminal, &mut app);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    if let Err(e) = res { eprintln!("{e:?}"); }
    Ok(())
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, app: &mut app::App) -> Result<()>
where
    <B as Backend>::Error: Send + Sync + 'static,
{
    loop {
        terminal.draw(|f| ui::ui(f, app))?;
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
