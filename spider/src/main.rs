use std::{
    io::{self, Stdout},
    sync::Mutex,
};

use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use game::Game;
use game_suit_prompt::ask_for_game_suit_loop;
use tui::{backend::CrosstermBackend, Terminal};

mod card;
mod game;
mod game_suit_prompt;

static TERMINAL: once_cell::sync::Lazy<Mutex<Terminal<CrosstermBackend<Stdout>>>> =
    once_cell::sync::Lazy::new(|| {
        let stdout = io::stdout();
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend).unwrap();

        Mutex::new(terminal)
    });

fn main() -> Result<(), io::Error> {
    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    stdout.execute(EnterAlternateScreen)?;
    stdout.execute(EnableMouseCapture)?;

    let game_suit = ask_for_game_suit_loop()?;

    if let Some(game_suit) = game_suit {
        let mut game = Game::new(game_suit);
        let res = game.run_game();
        if let Err(err) = res {
            println!("{}", err)
        }
    }

    // restore terminal
    let mut terminal = TERMINAL.lock().unwrap();
    disable_raw_mode()?;
    terminal.backend_mut().execute(LeaveAlternateScreen)?;
    terminal.backend_mut().execute(DisableMouseCapture)?;
    terminal.show_cursor()?;

    Ok(())
}
