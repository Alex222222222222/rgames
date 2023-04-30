use std::io;

use rand::Rng;
use tui::{
    layout::{Constraint, Layout, Rect},
    widgets::{Block, Borders},
};

use crate::{
    card::{Card, GameCard, GameSuitNumber, Rank, Suit},
    TERMINAL,
};

#[derive(Debug)]
pub struct Game {
    /// in unix milliseconds
    ///
    /// start from the first move
    pub start_time: Option<u128>,
    /// the tableau
    pub tableau: Vec<Vec<GameCard>>,
    /// the stock
    pub stock: Vec<GameCard>,
    /// current stock position
    ///
    /// indicate how many card already been draw
    pub current_stock_pos: usize,
    /// the score
    pub score: u32,
    /// the game suit
    pub game_suit: GameSuitNumber,
    /// history moves
    pub history_moves: Vec<GameMove>,
    /// the ui pos of the stock,
    /// should be initialised after first render
    ///
    /// used to decided wether the stock has been clicked
    pub stock_ui_pos: Option<Rect>,
}

/// The position of a card in the game
#[derive(Debug, Clone, Copy)]
pub struct CardPosition {
    /// The pile position.
    ///
    /// 0 is the stock
    ///
    /// 1-10 are the tableau
    pub pile: usize,
    /// The card position in the pile.
    ///
    /// 0 is the bottom card.
    pub card: usize,
}

/// The move the player wants to make.
#[derive(Debug, Clone, Copy)]
pub enum GameMove {
    /// Draw a card from the stock.
    DrawStock,
    /// Recycle the stock.
    RecycleStock,
    /// Move a card from the tableau to the tableau.
    ///
    /// Or a list of cards from the tableau to the tableau.
    MoveCard {
        src: CardPosition,
        dst: CardPosition,
    },
}

/// the error might occurred in a move
pub enum MoveError {
    /// try to draw a empty stock
    DrawEmptyStock,
    /// try to recycle a none empty stock
    RecycleNoneEmptyStock,
    /// move card src not exist
    MoveSrcNotExist,
    /// move invalid card in stock
    MoveInvalidStockCard,
    /// move dst not exist or occupied,
    /// or not valid regarding the game suit
    MoveDstNotValid,
}

impl Game {
    /// Render the game ui
    fn render_all(&mut self) -> io::Result<()> {
        let mut terminal = TERMINAL.lock().unwrap();

        let mut stock_chunks = Vec::new();
        let mut tableau_chunks = Vec::new();

        terminal.draw(|f| {
            let size = f.size();

            let outer_block = Block::default().title("Spider").borders(Borders::ALL);
            let new_size = outer_block.inner(size);
            f.render_widget(outer_block, size);
            let size = new_size;

            let stock_tableau_chunks = Layout::default()
                .direction(tui::layout::Direction::Vertical)
                .margin(1)
                .constraints([Constraint::Length(10), Constraint::Length(50)].as_ref())
                .split(size);

            stock_chunks = Layout::default()
                .direction(tui::layout::Direction::Horizontal)
                .margin(1)
                .constraints([Constraint::Length(24), Constraint::Length(10)].as_ref())
                .split(stock_tableau_chunks[0]);

            let mut tableau_constraint = Vec::new();
            for _ in 0..10 {
                tableau_constraint.push(Constraint::Length(10));
            }
            tableau_chunks = Layout::default()
                .direction(tui::layout::Direction::Horizontal)
                .margin(1)
                .constraints(tableau_constraint.as_ref())
                .split(stock_tableau_chunks[1]);
        })?;
        drop(terminal);

        self.render_left_stock(stock_chunks[1])?;
        self.render_visible_stock(stock_chunks[0])?;
        #[allow(clippy::needless_range_loop)]
        for i in 0..10 {
            self.render_pile(i, tableau_chunks[i])?;
        }

        Ok(())
    }

    fn render_pile(&mut self, pile: usize, area: Rect) -> io::Result<()> {
        let mut terminal = TERMINAL.lock().unwrap();

        let pile = self.tableau.get_mut(pile).unwrap();

        let n = pile.len();

        if n == 0 {
            let area = Rect::new(area.x, area.y, 8, 8);
            let card_block = Block::default().title("Empty").borders(Borders::ALL);

            terminal.draw(|f| {
                f.render_widget(card_block, area);
            })?;

            return Ok(());
        }

        let mut area = Rect::new(area.x, area.y + (2 * (n - 1)) as u16, 8, 8);
        for i in 0..n {
            let card = n - i - 1;
            let card = pile.get_mut(card);
            if card.is_none() {
                continue;
            }
            let card = card.unwrap();

            let title = if card.is_up {
                card.card.to_string()
            } else {
                String::from("")
            };

            let card_block = if i == 0 {
                Block::default().title(title).borders(Borders::ALL)
            } else {
                Block::default()
                    .title(title)
                    .borders(Borders::LEFT)
                    .borders(Borders::RIGHT)
                    .borders(Borders::TOP)
            };

            terminal.draw(|f| {
                f.render_widget(card_block, area);
            })?;

            if i == 0 {
                area.height = 2;
            }
            area.y -= 2;
        }

        Ok(())
    }

    fn render_visible_stock(&mut self, area: Rect) -> io::Result<()> {
        let mut terminal = TERMINAL.lock().unwrap();

        let mut n = self.current_stock_pos;
        if n > 4 {
            n = 4;
        }

        let mut area = Rect::new(area.x + area.width - 10, area.y, 8, 8);

        for i in 0..n {
            let card = n - i - 1;
            let card = self.stock.get_mut(card);
            if card.is_none() {
                continue;
            }
            let card = card.unwrap();

            if i == 0 {
                card.pos = Some(area);
            }

            let card_block = if i == 0 {
                Block::default()
                    .title(card.card.to_string())
                    .borders(Borders::all())
            } else {
                Block::default()
                    .title(card.card.to_string())
                    .borders(Borders::TOP)
                    .borders(Borders::BOTTOM)
                    .borders(Borders::LEFT)
            };

            terminal.draw(|f| f.render_widget(card_block, area))?;

            if i == 0 {
                area.width = 6;
            }
            area.x -= 6;
        }

        let card_block = Block::default()
            .title(format!("{}", self.current_stock_pos - n))
            .borders(Borders::TOP)
            .borders(Borders::BOTTOM)
            .borders(Borders::LEFT);
        terminal.draw(|f| f.render_widget(card_block, area))?;

        Ok(())
    }

    fn render_left_stock(&mut self, area: Rect) -> io::Result<()> {
        let mut terminal = TERMINAL.lock().unwrap();
        let mut area = area;
        if area.height > 8 {
            area.height = 8;
        }
        if area.width > 8 {
            area.width = 8;
        }

        terminal.draw(|f| {
            let stock_block = Block::default().title("Stock").borders(Borders::ALL);
            let inner = stock_block.inner(area);
            let chunks = Layout::default()
                .direction(tui::layout::Direction::Vertical)
                .constraints(
                    [
                        Constraint::Percentage(25),
                        Constraint::Percentage(50),
                        Constraint::Percentage(25),
                    ]
                    .as_ref(),
                )
                .margin(0)
                .split(inner);
            let left_block = Block::default()
                .title(format!("{}", self.stock.len() - self.current_stock_pos))
                .borders(Borders::empty());

            f.render_widget(stock_block, area);
            f.render_widget(left_block, chunks[1]);
        })?;

        self.stock_ui_pos = Some(area);

        Ok(())
    }

    /// do a move
    pub fn do_move(&mut self, game_move: GameMove) -> Result<(), MoveError> {
        match game_move {
            GameMove::DrawStock => self.do_move_draw_stock(),
            GameMove::RecycleStock => self.do_move_recycle_stock(),
            GameMove::MoveCard { src, dst } => self.do_move_card(src, dst),
        }
    }

    /// move a card
    fn do_move_card(&mut self, src: CardPosition, dst: CardPosition) -> Result<(), MoveError> {
        if src.pile == 0 {
            self.do_move_card_stock_to_tableau(src, dst)
        } else {
            self.do_move_card_tableau_to_tableau(src, dst)
        }
    }

    /// move card from stock to tableau
    fn do_move_card_stock_to_tableau(
        &mut self,
        src: CardPosition,
        dst: CardPosition,
    ) -> Result<(), MoveError> {
        if src.card != self.current_stock_pos - 1 {
            return Err(MoveError::MoveInvalidStockCard);
        }

        let src_card = self.stock.get(src.card);
        if src_card.is_none() {
            return Err(MoveError::MoveSrcNotExist);
        }
        let src_card = *src_card.unwrap();

        let dst_pile = self.tableau.get_mut(dst.pile);
        if dst_pile.is_none() {
            return Err(MoveError::MoveDstNotValid);
        }
        let dst_pile: &mut Vec<GameCard> = dst_pile.unwrap();

        if dst_pile.len() > dst.card {
            return Err(MoveError::MoveDstNotValid);
        }

        if src_card.card.rank == Rank::King && dst_pile.is_empty() {
            dst_pile.push(src_card);

            self.stock.remove(src.card);
            self.current_stock_pos -= 1;

            return Ok(());
        }

        let dst_before = dst_pile.last();
        if dst_before.is_none() {
            return Err(MoveError::MoveDstNotValid);
        }
        let dst_before = *dst_before.unwrap();

        let dst_before_rank: u8 = dst_before.card.rank.into();
        let src_card_rank: u8 = src_card.card.rank.into();
        if src_card_rank > dst_before_rank || dst_before_rank - src_card_rank != 1 {
            return Err(MoveError::MoveDstNotValid);
        }

        match self.game_suit {
            GameSuitNumber::One => {
                // always valid do nothing
            }
            GameSuitNumber::Two => {
                if src_card.card.suit.color() != dst_before.card.suit.color() {
                    return Err(MoveError::MoveDstNotValid);
                }
            }
            GameSuitNumber::Four => {
                if src_card.card.suit != dst_before.card.suit {
                    return Err(MoveError::MoveDstNotValid);
                }
            }
        }

        dst_pile.push(src_card);

        self.stock.remove(src.card);
        self.current_stock_pos -= 1;

        Ok(())
    }

    /// move card from tableau to tableau
    fn do_move_card_tableau_to_tableau(
        &mut self,
        src: CardPosition,
        dst: CardPosition,
    ) -> Result<(), MoveError> {
        let src_pile = self.tableau.get(src.pile);
        if src_pile.is_none() {
            return Err(MoveError::MoveSrcNotExist);
        }
        let src_pile = src_pile.unwrap().clone();

        let src_card = src_pile.get(src.card);
        if src_card.is_none() {
            return Err(MoveError::MoveSrcNotExist);
        }
        let src_card = *src_card.unwrap();

        let dst_pile = self.tableau.get_mut(dst.pile);
        if dst_pile.is_none() {
            return Err(MoveError::MoveDstNotValid);
        }
        let dst_pile = dst_pile.unwrap();

        if dst_pile.len() > dst.card {
            return Err(MoveError::MoveDstNotValid);
        }

        if src_card.card.rank == Rank::King && dst_pile.is_empty() {
            let n = src_pile.len() - src.card;
            src_pile
                .into_iter()
                .skip(src.card)
                .for_each(|v| dst_pile.push(v));
            let src_pile = self.tableau.get_mut(src.pile).unwrap();
            for _ in 0..n {
                src_pile.pop();
            }

            return Ok(());
        }

        let dst_before = dst_pile.last();
        if dst_before.is_none() {
            return Err(MoveError::MoveDstNotValid);
        }
        let dst_before = *dst_before.unwrap();

        let dst_before_rank: u8 = dst_before.card.rank.into();
        let src_card_rank: u8 = src_card.card.rank.into();
        if src_card_rank > dst_before_rank || dst_before_rank - src_card_rank != 1 {
            return Err(MoveError::MoveDstNotValid);
        }

        match self.game_suit {
            GameSuitNumber::One => {
                // always valid do nothing
            }
            GameSuitNumber::Two => {
                if src_card.card.suit.color() != dst_before.card.suit.color() {
                    return Err(MoveError::MoveDstNotValid);
                }
            }
            GameSuitNumber::Four => {
                if src_card.card.suit != dst_before.card.suit {
                    return Err(MoveError::MoveDstNotValid);
                }
            }
        }

        let n = src_pile.len() - src.card;
        src_pile
            .into_iter()
            .skip(src.card)
            .for_each(|v| dst_pile.push(v));
        let src_pile = self.tableau.get_mut(src.pile).unwrap();
        for _ in 0..n {
            src_pile.pop();
        }

        // auto turn the last card to up
        let last = src_pile.last_mut();
        if let Some(last) = last {
            last.is_up = true;
        }

        Ok(())
    }

    /// draw one card from stock
    fn do_move_draw_stock(&mut self) -> Result<(), MoveError> {
        if self.current_stock_pos >= self.stock.len() {
            return Err(MoveError::DrawEmptyStock);
        }

        let c = self.stock.get_mut(self.current_stock_pos - 1);
        if let Some(c) = c {
            c.pos = None;
        }

        self.current_stock_pos += 1;

        Ok(())
    }

    /// recycle the stock
    fn do_move_recycle_stock(&mut self) -> Result<(), MoveError> {
        if self.current_stock_pos < self.stock.len() {
            return Err(MoveError::RecycleNoneEmptyStock);
        }

        let c = self.stock.get_mut(self.current_stock_pos - 1);
        if let Some(c) = c {
            c.pos = None;
        }

        self.current_stock_pos = 0;

        Ok(())
    }

    /// create a new game, with a given game suit
    pub fn new(game_suit: GameSuitNumber) -> Self {
        let mut rng = rand::thread_rng();

        let mut all_cards = Vec::with_capacity(104);
        for i in 1..14 {
            all_cards.push(GameCard {
                card: Card {
                    suit: Suit::Clubs,
                    rank: Rank::from(i),
                },
                is_up: false,
                pos: None,
            });
            all_cards.push(GameCard {
                card: Card {
                    suit: Suit::Clubs,
                    rank: Rank::from(i),
                },
                is_up: false,
                pos: None,
            });
        }
        for i in 1..14 {
            all_cards.push(GameCard {
                card: Card {
                    suit: Suit::Clubs,
                    rank: Rank::from(i),
                },
                is_up: false,
                pos: None,
            });
            all_cards.push(GameCard {
                card: Card {
                    suit: Suit::Diamonds,
                    rank: Rank::from(i),
                },
                is_up: false,
                pos: None,
            });
        }
        for i in 1..14 {
            all_cards.push(GameCard {
                card: Card {
                    suit: Suit::Clubs,
                    rank: Rank::from(i),
                },
                is_up: false,
                pos: None,
            });
            all_cards.push(GameCard {
                card: Card {
                    suit: Suit::Hearts,
                    rank: Rank::from(i),
                },
                is_up: false,
                pos: None,
            });
        }
        for i in 1..14 {
            all_cards.push(GameCard {
                card: Card {
                    suit: Suit::Clubs,
                    rank: Rank::from(i),
                },
                is_up: false,
                pos: None,
            });
            all_cards.push(GameCard {
                card: Card {
                    suit: Suit::Spades,
                    rank: Rank::from(i),
                },
                is_up: false,
                pos: None,
            });
        }

        let mut tableau = Vec::with_capacity(10);
        for _ in 0..4 {
            let mut pile = Vec::with_capacity(6);

            for _ in 0..5 {
                let num = rng.gen_range(0..all_cards.len());
                let card = all_cards.swap_remove(num);
                pile.push(card);
            }
            let num = rng.gen_range(0..all_cards.len());
            let mut card = all_cards.swap_remove(num);
            card.is_up = true;
            pile.push(card);
            tableau.push(pile);
        }
        for _ in 0..6 {
            let mut pile = Vec::with_capacity(6);

            for _ in 0..4 {
                let num = rng.gen_range(0..all_cards.len());
                let card = all_cards.swap_remove(num);
                pile.push(card);
            }
            let num = rng.gen_range(0..all_cards.len());
            let mut card = all_cards.swap_remove(num);
            card.is_up = true;
            pile.push(card);
            tableau.push(pile);
        }

        let mut stock = Vec::with_capacity(54);
        for _ in 0..54 {
            let num = rng.gen_range(0..all_cards.len());
            let card = all_cards.swap_remove(num);
            stock.push(card);
        }

        Game {
            start_time: None,
            tableau,
            stock,
            current_stock_pos: 0,
            score: 0,
            game_suit,
            history_moves: Vec::new(),
            stock_ui_pos: None,
        }
    }
}
