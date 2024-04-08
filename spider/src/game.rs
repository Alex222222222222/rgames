use std::io::{self, Stdout};

use crossterm::event::{self, MouseEventKind};
use rand::Rng;
use tui::{
    backend::CrosstermBackend,
    layout::{Constraint, Layout, Rect},
    style::Style,
    widgets::{Block, Borders},
    Frame,
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
    /// used to decided whether the stock has been clicked
    pub stock_ui_pos: Option<Rect>,
    stock_chunks: Vec<Rect>,
    tableau_chunks: Vec<Rect>,
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
        /// If is move card from tableau to tableau,
        /// if the card before the src card is not turn up,
        /// then after the move, this card need to be turn up.
        /// and before_visible is set to be true,
        ///
        /// If the card before the src card is turn up,
        /// then after the move, before_visible is set to false.
        ///
        /// Otherwise None.
        before_visible: Option<bool>,
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

/// test if a point is in the Rect
fn test_point_in_rect(x: u16, y: u16, rect: Rect) -> bool {
    x >= rect.x && y >= rect.y && x < rect.x + rect.width && y < rect.y + rect.height
}

/// verity a card could go under another card
fn verify_under(game_suit: GameSuitNumber, up: Card, down: Card) -> bool {
    let up_rank: u8 = up.rank.into();
    let down_rank: u8 = down.rank.into();
    if up_rank < down_rank {
        return false;
    }
    if up_rank - down_rank != 1 {
        return false;
    }

    match game_suit {
        GameSuitNumber::One => true,
        GameSuitNumber::Two => up.suit.color() == down.suit.color(),
        GameSuitNumber::Four => up.suit == down.suit,
    }
}

impl Game {
    /// test if a game is win
    pub fn test_win(&self) -> bool {
        // if stock not empty,
        // return false
        if !self.stock.is_empty() {
            return false;
        }

        for i in 0..10 {
            let pile = self.tableau.get(i);
            if pile.is_none() {
                continue;
            }
            let pile = pile.unwrap();

            if pile.is_empty() {
                continue;
            }

            if pile.len() != 13 {
                return false;
            }

            #[allow(clippy::needless_range_loop)]
            for j in 0..13 {
                let card = pile[j];
                if !card.is_up {
                    return false;
                }
            }
        }

        true
    }

    /// undo once
    pub fn undo_once(&mut self) {
        let game_move = self.history_moves.last();
        if game_move.is_none() {
            return;
        }
        let game_move = *game_move.unwrap();

        let res = self.undo_move(game_move);
        if res.is_ok() {
            self.history_moves.pop();
        }
    }

    /// undo a move
    fn undo_move(&mut self, game_move: GameMove) -> Result<(), MoveError> {
        match game_move {
            GameMove::DrawStock => self.undo_move_draw_stock(),
            GameMove::RecycleStock => self.undo_recycle_stock(),
            GameMove::MoveCard {
                src,
                dst,
                before_visible,
            } => {
                if src.pile == 0 {
                    self.undo_move_stock_to_tableau(dst)
                } else {
                    self.undo_move_tableau_to_tableau(src, dst, before_visible)
                }
            }
        }
    }

    /// undo the draw stock move
    fn undo_move_draw_stock(&mut self) -> Result<(), MoveError> {
        if self.current_stock_pos == 0 {
            return Err(MoveError::DrawEmptyStock);
        }

        self.current_stock_pos -= 1;

        Ok(())
    }

    /// undo the recycle stock move
    fn undo_recycle_stock(&mut self) -> Result<(), MoveError> {
        if self.current_stock_pos != 0 {
            return Err(MoveError::RecycleNoneEmptyStock);
        }

        self.current_stock_pos = self.stock.len();

        Ok(())
    }

    /// undo the move card from stock to tableau
    fn undo_move_stock_to_tableau(&mut self, dst: CardPosition) -> Result<(), MoveError> {
        // get dst card
        if dst.pile == 0 {
            return Err(MoveError::MoveDstNotValid);
        }
        let dst_pile = self.tableau.get_mut(dst.pile - 1);
        if dst_pile.is_none() {
            return Err(MoveError::MoveDstNotValid);
        }
        let dst_pile = dst_pile.unwrap();
        let dst_card = dst_pile.get(dst.card);
        if dst_card.is_none() {
            return Err(MoveError::MoveDstNotValid);
        }
        let dst_card = *dst_card.unwrap();

        self.stock.insert(self.current_stock_pos, dst_card);
        self.current_stock_pos += 1;

        if dst.card < dst_pile.len() {
            let n = dst_pile.len();
            for _ in dst.card..n {
                dst_pile.pop();
            }
        }

        Ok(())
    }

    /// undo move card from tableau to tableau
    fn undo_move_tableau_to_tableau(
        &mut self,
        src: CardPosition,
        dst: CardPosition,
        before_visible: Option<bool>,
    ) -> Result<(), MoveError> {
        let src_pile = self.tableau.get_mut(src.pile - 1);
        if src_pile.is_none() {
            return Err(MoveError::MoveSrcNotExist);
        }

        if let Some(before_visible) = before_visible {
            if src.card > 0 {
                let card = self.tableau[src.pile - 1].get_mut(src.card - 1);
                if let Some(card) = card {
                    card.is_up = !before_visible;
                }
            }
        }

        let dst_pile = self.tableau.get_mut(dst.pile - 1);
        if dst_pile.is_none() {
            return Err(MoveError::MoveDstNotValid);
        }
        let dst_pile = dst_pile.unwrap();

        if dst_pile.len() <= dst.card {
            return Err(MoveError::MoveDstNotValid);
        }

        let n = dst_pile.len() - dst.card;
        for _ in 0..n {
            let card = self.tableau[dst.pile - 1].remove(dst.card);
            self.tableau[src.pile - 1].push(card);
        }

        Ok(())
    }

    /// do a move
    pub fn do_move(&mut self, game_move: GameMove) -> Result<(), MoveError> {
        let res = match game_move {
            GameMove::DrawStock => self.do_move_draw_stock(),
            GameMove::RecycleStock => self.do_move_recycle_stock(),
            GameMove::MoveCard {
                src,
                dst,
                before_visible: _,
            } => self.do_move_card(src, dst),
        };

        if res.is_ok() {
            self.history_moves.push(game_move);
        }

        res
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
        if !src_card.is_up {
            return Err(MoveError::MoveSrcNotExist);
        }

        let dst_pile = self.tableau.get_mut(dst.pile - 1);
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

        if !verify_under(self.game_suit, dst_before.card, src_card.card) {
            return Err(MoveError::MoveDstNotValid);
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
        let src_pile = self.tableau.get(src.pile - 1);
        if src_pile.is_none() {
            return Err(MoveError::MoveSrcNotExist);
        }
        let src_pile = src_pile.unwrap().clone();

        let src_card = src_pile.get(src.card);
        if src_card.is_none() {
            return Err(MoveError::MoveSrcNotExist);
        }
        let src_card = *src_card.unwrap();
        if !src_card.is_up {
            return Err(MoveError::MoveSrcNotExist);
        }

        let dst_pile = self.tableau.get_mut(dst.pile - 1);
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
            let src_pile = self.tableau.get_mut(src.pile - 1).unwrap();
            for _ in 0..n {
                src_pile.pop();
            }

            // auto turn the last card to up
            let last = src_pile.last_mut();
            if let Some(last) = last {
                last.is_up = true;
            }

            return Ok(());
        }

        let dst_before = dst_pile.last();
        if dst_before.is_none() {
            return Err(MoveError::MoveDstNotValid);
        }
        let dst_before = *dst_before.unwrap();

        verify_under(self.game_suit, dst_before.card, src_card.card);

        let n = src_pile.len() - src.card;
        src_pile
            .into_iter()
            .skip(src.card)
            .for_each(|v| dst_pile.push(v));
        let src_pile = self.tableau.get_mut(src.pile - 1).unwrap();
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

        if self.current_stock_pos == 0 {
            self.current_stock_pos = 1;
            return Ok(());
        }

        self.current_stock_pos += 1;

        Ok(())
    }

    /// recycle the stock
    fn do_move_recycle_stock(&mut self) -> Result<(), MoveError> {
        if self.current_stock_pos < self.stock.len() {
            return Err(MoveError::RecycleNoneEmptyStock);
        }

        self.current_stock_pos = 0;

        Ok(())
    }

    /// find possible move for a given card
    ///
    /// if no, return none
    fn find_possible_move(&self, src: CardPosition) -> Option<GameMove> {
        let card = if src.pile == 0 {
            self.stock.get(src.card)
        } else {
            let pile = self.tableau.get(src.pile - 1);
            if let Some(pile) = pile {
                pile.get(src.card)
            } else {
                return None;
            }
        }?;

        let before_visible = if src.pile == 0 {
            None
        } else if src.card < 1 {
            Some(false)
        } else {
            let pile = self.tableau.get(src.pile - 1);

            if let Some(pile) = pile {
                let card = pile.get(src.card - 1);
                if let Some(card) = card {
                    Some(!card.is_up)
                } else {
                    Some(false)
                }
            } else {
                Some(false)
            }
        };

        for i in 0..10 {
            if i + 1 == src.pile {
                continue;
            }

            let pile = self.tableau.get(i);
            if pile.is_none() {
                continue;
            }
            let pile = pile.unwrap();

            if pile.is_empty() && card.card.rank == Rank::King {
                return Some(GameMove::MoveCard {
                    src,
                    dst: CardPosition {
                        pile: i + 1,
                        card: 0,
                    },
                    before_visible,
                });
            }

            let last_card = pile.last();
            if last_card.is_none() {
                continue;
            }
            let last_card = last_card.unwrap();

            if !verify_under(self.game_suit, last_card.card, card.card) {
                continue;
            }

            return Some(GameMove::MoveCard {
                src,
                dst: CardPosition {
                    pile: i + 1,
                    card: pile.len(),
                },
                before_visible,
            });
        }

        None
    }

    /// the function to handle crossterm click event
    fn handle_click(&mut self, event: crossterm::event::MouseEvent) -> crossterm::Result<()> {
        let button = match event.kind {
            MouseEventKind::Down(button) => button,
            _ => return Ok(()),
        };

        match button {
            crossterm::event::MouseButton::Left => {}
            _ => return Ok(()),
        }

        let x = event.column;
        let y = event.row;

        if test_point_in_rect(x, y, self.stock_chunks[0]) {
            let card = self.stock.get(self.current_stock_pos - 1);
            if card.is_none() {
                return Ok(());
            }
            let card = card.unwrap();

            if card.pos.is_none() {
                return Ok(());
            }

            if !test_point_in_rect(x, y, card.pos.unwrap()) {
                return Ok(());
            }

            let game_move = self.find_possible_move(CardPosition {
                pile: 0,
                card: self.current_stock_pos - 1,
            });
            if let Some(game_move) = game_move {
                let _ = self.do_move(game_move);
            }

            return Ok(());
        }

        if test_point_in_rect(x, y, self.stock_ui_pos.unwrap()) {
            if self.current_stock_pos >= self.stock.len() {
                let _ = self.do_move(GameMove::RecycleStock);
            } else {
                let _ = self.do_move(GameMove::DrawStock);
            }

            return Ok(());
        }

        for i in 0..10 {
            if test_point_in_rect(x, y, self.tableau_chunks[i]) {
                for j in 0..self.tableau[i].len() {
                    let c = self.tableau[i][j];
                    if let Some(pos) = c.pos {
                        if c.is_up && test_point_in_rect(x, y, pos) {
                            let game_move = self.find_possible_move(CardPosition {
                                pile: i + 1,
                                card: j,
                            });
                            if let Some(game_move) = game_move {
                                let _ = self.do_move(game_move);
                            }
                            return Ok(());
                        }
                    }
                }

                return Ok(());
            }
        }

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
                    suit: Suit::Diamonds,
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
                    suit: Suit::Hearts,
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
                    suit: Suit::Spades,
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

        let mut stock = Vec::with_capacity(50);
        for _ in 0..50 {
            let num = rng.gen_range(0..all_cards.len());
            let mut card = all_cards.swap_remove(num);
            card.is_up = true;
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
            stock_chunks: Vec::new(),
            tableau_chunks: Vec::new(),
        }
    }

    /// reset all the ui pos of card before render
    fn render_reset_ui_pos(&mut self) {
        self.stock_ui_pos = None;

        self.stock.iter_mut().for_each(|c| {
            c.pos = None;
        });

        self.tableau.iter_mut().for_each(|pile| {
            pile.iter_mut().for_each(|c| {
                c.pos = None;
            })
        });
    }

    /// Render the game ui
    fn render_all(&mut self) -> io::Result<()> {
        self.render_reset_ui_pos();

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
                .constraints([Constraint::Length(50), Constraint::Length(10)].as_ref())
                .split(stock_tableau_chunks[0]);

            let mut tableau_constraint = Vec::new();
            for _ in 0..10 {
                tableau_constraint.push(Constraint::Length(10));
            }
            tableau_chunks = Layout::default()
                .direction(tui::layout::Direction::Horizontal)
                .margin(1)
                .constraints(tableau_constraint.clone())
                .split(stock_tableau_chunks[1]);

            self.render_left_stock(stock_chunks[1], f);
            self.render_visible_stock(stock_chunks[0], f);
            #[allow(clippy::needless_range_loop)]
            for i in 0..10 {
                self.render_pile(i, tableau_chunks[i], f);
            }
        })?;
        drop(terminal);

        self.stock_chunks = stock_chunks;
        self.tableau_chunks = tableau_chunks;

        Ok(())
    }

    /// render the left stock ui
    fn render_left_stock(&mut self, area: Rect, f: &mut Frame<CrosstermBackend<Stdout>>) {
        let mut area = area;
        if area.height > 8 {
            area.height = 8;
        }
        if area.width > 8 {
            area.width = 8;
        }

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

        self.stock_ui_pos = Some(area);
    }

    /// render the tableau
    fn render_pile(&mut self, pile: usize, area: Rect, f: &mut Frame<CrosstermBackend<Stdout>>) {
        let pile = self.tableau.get_mut(pile).unwrap();

        let n = pile.len();

        if n == 0 {
            let area = Rect::new(area.x, area.y, 8, 8);
            let card_block = Block::default().title("Empty").borders(Borders::ALL);

            f.render_widget(card_block, area);

            return;
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
                card.pos = Some(area);
                card.card.to_string()
            } else {
                String::from("")
            };

            let mut card_block = if i == 0 {
                Block::default().title(title).borders(Borders::ALL)
            } else {
                Block::default()
                    .title(title)
                    .borders(Borders::LEFT)
                    .borders(Borders::RIGHT)
                    .borders(Borders::TOP)
            };

            if card.is_up {
                card_block = card_block.style(Style::default().fg(card.card.suit.color()));
            }

            f.render_widget(card_block, area);

            if i == 0 {
                area.height = 2;
            }
            area.y -= 2;
        }
    }

    /// render the stock
    fn render_visible_stock(&mut self, area: Rect, f: &mut Frame<CrosstermBackend<Stdout>>) {
        let mut n = self.current_stock_pos;
        if n > 4 {
            n = 4;
        }

        let mut area = Rect::new(area.x + area.width - 10, area.y, 8, 8);

        for i in 0..n {
            let card = self.current_stock_pos - i - 1;
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
            let card_block = card_block.style(Style::default().fg(card.card.suit.color()));

            f.render_widget(card_block, area);

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
        f.render_widget(card_block, area);
    }

    /// run the game
    pub fn run_game(&mut self) -> crossterm::Result<()> {
        loop {
            self.render_all()?;

            let event = crossterm::event::read()?;

            let key = match event {
                crossterm::event::Event::Key(c) => c,
                crossterm::event::Event::Mouse(event) => {
                    self.handle_click(event)?;
                    continue;
                }
                _ => continue,
            };

            let c = match key.code {
                event::KeyCode::Esc => return Ok(()),
                event::KeyCode::Char(c) => c,
                _ => continue,
            };

            match c {
                'q' => return Ok(()),
                'u' => self.undo_once(),
                's' => {
                    let _ = self.do_move(GameMove::DrawStock);
                }
                'w' => {
                    if self.test_win() {
                        return Ok(());
                    }
                }
                _ => continue,
            }
        }
    }
}
