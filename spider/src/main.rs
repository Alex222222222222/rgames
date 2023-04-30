use std::io::{stdout, Write};

use crossterm::ExecutableCommand;

mod card;
mod game;

fn main() {
    let res = enter_alternative_terminal();
    if let Err(e) = res {
        let res = quit_alternative_terminal();
        if let Err(e) = res {
            panic!("Error while quitting alternative terminal: {}", e);
        }
        panic!("Error while entering alternative terminal: {}", e);
    }

    let res = quit_alternative_terminal();
    if let Err(e) = res {
        panic!("Error while quitting alternative terminal: {}", e);
    }
}

/// Enter crossterm raw terminal mode
fn enter_alternative_terminal() -> crossterm::Result<()> {
    let mut stdout = stdout();
    stdout.execute(crossterm::cursor::Hide)?;
    stdout.execute(crossterm::terminal::EnterAlternateScreen)?;
    crossterm::terminal::enable_raw_mode()?;
    stdout.flush()?;

    Ok(())
}

/// Quit crossterm raw terminal mode
fn quit_alternative_terminal() -> crossterm::Result<()> {
    let mut stdout = stdout();
    stdout.execute(crossterm::cursor::Show)?;
    stdout.execute(crossterm::terminal::LeaveAlternateScreen)?;
    crossterm::terminal::disable_raw_mode()?;

    stdout.flush()?;

    Ok(())
}
