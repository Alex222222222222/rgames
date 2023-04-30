use tui::{
    layout::{Constraint, Direction, Layout},
    style::Style,
    widgets::{Block, Borders, List, ListItem, ListState},
};

use crate::{card::GameSuitNumber, TERMINAL};

static PROMPT_MESSAGE_BLOCK: once_cell::sync::Lazy<Block> = once_cell::sync::Lazy::new(|| {
    Block::default()
        .title("Please select a game suit number:")
        .borders(Borders::all())
});

static GAME_SUIT_STRING_LIST: once_cell::sync::Lazy<Vec<String>> =
    once_cell::sync::Lazy::new(|| {
        vec![
            String::from("1. One"),
            String::from("2. Two"),
            String::from("3. Four"),
        ]
    });
static GAME_SUIT_LIST: once_cell::sync::Lazy<List> = once_cell::sync::Lazy::new(|| {
    let list_items: Vec<ListItem> = GAME_SUIT_STRING_LIST
        .iter()
        .map(|i| ListItem::new(i.as_ref()))
        .collect();

    List::new(list_items).highlight_style(
        Style::default()
            .bg(tui::style::Color::Black)
            .fg(tui::style::Color::White),
    )
});

/// ask for a game suit
///
/// none means user press esc or q
/// otherwise return a valid game suit number
pub fn ask_for_game_suit_loop() -> crossterm::Result<Option<GameSuitNumber>> {
    let mut terminal = TERMINAL.lock().unwrap();

    let mut state = ListState::default();
    state.select(Some(2));

    loop {
        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints(
                    [
                        Constraint::Percentage(10),
                        Constraint::Percentage(80),
                        Constraint::Percentage(10),
                    ]
                    .as_ref(),
                )
                .split(f.size());
            let block = PROMPT_MESSAGE_BLOCK.clone();
            let inner = block.inner(chunks[1]);
            f.render_widget(block, chunks[1]);
            f.render_stateful_widget(GAME_SUIT_LIST.clone(), inner, &mut state);
        })?;

        let event = crossterm::event::read()?;
        let event = match event {
            crossterm::event::Event::Key(e) => e.code,
            _ => continue,
        };

        let c = match event {
            crossterm::event::KeyCode::Enter => {
                // return result
                let i = state.selected();
                match i {
                    Some(0) => return Ok(Some(GameSuitNumber::One)),
                    Some(1) => return Ok(Some(GameSuitNumber::Two)),
                    Some(2) => return Ok(Some(GameSuitNumber::Four)),
                    _ => return Ok(Some(GameSuitNumber::default())),
                }
            }
            crossterm::event::KeyCode::Char(c) => c,
            crossterm::event::KeyCode::Esc => {
                return Ok(None);
            }
            crossterm::event::KeyCode::Up => {
                // select previous item
                let i = state.selected();
                match i {
                    Some(0) => {
                        state.select(Some(2));
                        continue;
                    }
                    Some(1) => {
                        state.select(Some(0));
                        continue;
                    }
                    Some(2) => {
                        state.select(Some(1));
                        continue;
                    }
                    _ => {
                        state.select(Some(2));
                        continue;
                    }
                }
            }
            crossterm::event::KeyCode::Down => {
                // select next item
                let i = state.selected();
                match i {
                    Some(0) => {
                        state.select(Some(1));
                        continue;
                    }
                    Some(1) => {
                        state.select(Some(2));
                        continue;
                    }
                    Some(2) => {
                        state.select(Some(0));
                        continue;
                    }
                    _ => {
                        state.select(Some(2));
                        continue;
                    }
                }
            }
            _ => continue,
        };

        match c {
            'q' => return Ok(None),
            _ => continue,
        }
    }
}
