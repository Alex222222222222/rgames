use rand::Rng;

use crate::card::{self, Card, GameCard, GameSuitNumber, Rank, Suit};

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
    /// do a move
    pub fn do_move(&mut self, game_move: GameMove) -> Result<(), MoveError> {
        match game_move {
            GameMove::DrawStock => self.do_move_draw_stock(),
            GameMove::RecycleStock => self.do_move_recycle_stock(),
            GameMove::MoveCard { src, dst } => self.do_move_card(src, dst),
        }
    }

    /// draw one card from stock
    fn do_move_draw_stock(&mut self) -> Result<(), MoveError> {
        if self.current_stock_pos >= self.stock.len() {
            return Err(MoveError::DrawEmptyStock);
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
            });
            all_cards.push(GameCard {
                card: Card {
                    suit: Suit::Clubs,
                    rank: Rank::from(i),
                },
                is_up: false,
            });
        }
        for i in 1..14 {
            all_cards.push(GameCard {
                card: Card {
                    suit: Suit::Clubs,
                    rank: Rank::from(i),
                },
                is_up: false,
            });
            all_cards.push(GameCard {
                card: Card {
                    suit: Suit::Diamonds,
                    rank: Rank::from(i),
                },
                is_up: false,
            });
        }
        for i in 1..14 {
            all_cards.push(GameCard {
                card: Card {
                    suit: Suit::Clubs,
                    rank: Rank::from(i),
                },
                is_up: false,
            });
            all_cards.push(GameCard {
                card: Card {
                    suit: Suit::Hearts,
                    rank: Rank::from(i),
                },
                is_up: false,
            });
        }
        for i in 1..14 {
            all_cards.push(GameCard {
                card: Card {
                    suit: Suit::Clubs,
                    rank: Rank::from(i),
                },
                is_up: false,
            });
            all_cards.push(GameCard {
                card: Card {
                    suit: Suit::Spades,
                    rank: Rank::from(i),
                },
                is_up: false,
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
        }
    }
}
