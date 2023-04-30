use std::fmt::Display;

use crossterm::style::Color;
use tui::layout::Rect;

#[derive(Debug, Clone, Copy)]
pub struct Card {
    pub suit: Suit,
    pub rank: Rank,
}

#[derive(Debug, Clone, Copy)]
pub struct GameCard {
    pub card: Card,
    pub is_up: bool,
    /// should initialised at first render
    ///
    /// used to decide whether the card has been clicked
    pub pos: Option<Rect>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Suit {
    Clubs,
    Diamonds,
    Hearts,
    Spades,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rank {
    Ace,
    Two,
    Three,
    Four,
    Five,
    Six,
    Seven,
    Eight,
    Nine,
    Ten,
    Jack,
    Queen,
    King,
}

impl From<u8> for Rank {
    fn from(value: u8) -> Self {
        match value {
            1 => Rank::Ace,
            2 => Rank::Two,
            3 => Rank::Three,
            4 => Rank::Four,
            5 => Rank::Five,
            6 => Rank::Six,
            7 => Rank::Seven,
            8 => Rank::Eight,
            9 => Rank::Nine,
            10 => Rank::Ten,
            11 => Rank::Jack,
            12 => Rank::Queen,
            13 => Rank::King,
            _ => panic!("Invalid rank"),
        }
    }
}

impl From<Rank> for u8 {
    fn from(value: Rank) -> Self {
        match value {
            Rank::Ace => 1,
            Rank::Two => 2,
            Rank::Three => 3,
            Rank::Four => 4,
            Rank::Five => 5,
            Rank::Six => 6,
            Rank::Seven => 7,
            Rank::Eight => 8,
            Rank::Nine => 9,
            Rank::Ten => 10,
            Rank::Jack => 11,
            Rank::Queen => 12,
            Rank::King => 13,
        }
    }
}

#[derive(Debug, Default)]
pub enum GameSuitNumber {
    One,
    #[default]
    Two,
    Four,
}

impl Display for GameSuitNumber {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GameSuitNumber::One => write!(f, "One"),
            GameSuitNumber::Two => write!(f, "Two"),
            GameSuitNumber::Four => write!(f, "Four"),
        }
    }
}

impl Display for Rank {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Rank::Ace => write!(f, "A "),
            Rank::Two => write!(f, "2 "),
            Rank::Three => write!(f, "3 "),
            Rank::Four => write!(f, "4 "),
            Rank::Five => write!(f, "5 "),
            Rank::Six => write!(f, "6 "),
            Rank::Seven => write!(f, "7 "),
            Rank::Eight => write!(f, "8 "),
            Rank::Nine => write!(f, "9 "),
            Rank::Ten => write!(f, "10"),
            Rank::Jack => write!(f, "J "),
            Rank::Queen => write!(f, "Q "),
            Rank::King => write!(f, "K "),
        }
    }
}

impl Display for Suit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Suit::Clubs => write!(f, "♣"),
            Suit::Diamonds => write!(f, "♦"),
            Suit::Hearts => write!(f, "♥"),
            Suit::Spades => write!(f, "♠"),
        }
    }
}

impl Display for Card {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.rank, self.suit)
    }
}

impl Suit {
    pub fn color(&self) -> Color {
        match self {
            Suit::Clubs => Color::Black,
            Suit::Diamonds => Color::Red,
            Suit::Hearts => Color::Red,
            Suit::Spades => Color::Black,
        }
    }
}

#[derive(Default)]
pub struct GameSuitNumberPrompt {
    pub current_select: GameSuitNumber,
}
