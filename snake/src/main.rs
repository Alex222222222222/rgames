use std::{
    collections::{HashMap, HashSet},
    io::{stdout, Write},
};

use crossterm::{
    cursor::MoveTo,
    event,
    style::{Color, Print, SetBackgroundColor},
    ExecutableCommand, QueueableCommand, Result,
};
use rand::Rng;

const INIT_SPEED: f32 = 0.000000002;
const INIT_LENGTH: u16 = 3;
const FOOD_NUM: usize = 5;
const FOOD_MAX_SCORE: u16 = 5;
const UPDATES_INTERVAL: std::time::Duration = std::time::Duration::from_millis(20);

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
pub struct Position {
    pub x: u16,
    pub y: u16,
}

impl From<Position> for u32 {
    fn from(pos: Position) -> Self {
        let mut res = pos.x as u32;
        res <<= 16;
        res += pos.y as u32;
        res
    }
}

impl From<u32> for Position {
    fn from(pos: u32) -> Self {
        let mut res = pos;
        let y = res & 0xffff;
        res >>= 16;
        let x = res & 0xffff;
        Position {
            x: x as u16,
            y: y as u16,
        }
    }
}

pub struct Snake {
    pub body: Vec<Position>,
    pub direction: Direction,
}

pub struct Game {
    pub snake: Snake,
    pub food: HashMap<Position, u16>,
    pub width: u16,
    pub height: u16,
    pub score: u16,
    // per block per nanoseconds
    //
    // increase in ln(score)
    pub speed: f32,
    pub clear: Vec<Position>,
    // unix timestamp in nanoseconds
    pub last_move: u128,
}

/// Loop with interval.
///
/// Each iteration of the loop will be executed with a given interval.
/// If the execution of the loop body takes longer than the interval,
/// the next iteration will be executed immediately.
/// This function will block the current thread.
fn loop_with_interval<F>(interval: std::time::Duration, mut f: F)
where
    F: FnMut(),
{
    loop {
        let start = std::time::Instant::now();
        f();
        let elapsed = start.elapsed();
        if elapsed < interval {
            std::thread::sleep(interval - elapsed);
        }
    }
}

fn main() -> std::io::Result<()> {
    // execute!(
    // stdout(),
    // SetForegroundColor(Color::Blue),
    // SetBackgroundColor(Color::Red),
    // Print("Styled text here."),
    // ResetColor
    // )?;
    //
    // stdout()
    // .execute(SetForegroundColor(Color::Blue))?
    // .execute(SetBackgroundColor(Color::Red))?
    // .execute(Print("Styled text here."))?
    // .execute(ResetColor)?;

    // Get size of terminal
    let (width, height) = crossterm::terminal::size()?;
    let mut width = width;
    if width % 2 != 0 {
        width -= 1;
    }
    width -= 2;
    width /= 2;
    let height = height - 4;

    let mut game = Game::new(width, height);
    game.run().unwrap();

    Ok(())
}

impl Game {
    /// check if snake eat food
    fn check_eat_food(&mut self) -> Result<()> {
        // get head position
        let head = self.snake.body[0];

        // check if snake eat food
        let score = self.food.get(&head);
        if let Some(&score) = score {
            // remove food
            self.food.remove(&head);

            // increase score
            self.score += score;

            // increase speed
            self.speed = ((self.score as f32).ln() + 1.0) * INIT_SPEED;

            // generate new food
            self.generate_food();

            // grow snake
            let tail = *self.snake.body.last().unwrap();
            for _ in 0..score {
                self.snake.body.push(tail);
            }
        }

        Ok(())
    }

    /// check if hit wall
    ///
    /// if hit wall, then move snake to other side
    fn check_hit_wall(&mut self) -> Result<()> {
        // get head position
        let head = self.snake.body[0];

        if head.x == 0 {
            self.snake.body[0].x = self.width;
        } else if head.x == self.width + 1 {
            self.snake.body[0].x = 1;
        } else if head.y == 0 {
            self.snake.body[0].y = self.height;
        } else if head.y == self.height + 1 {
            self.snake.body[0].y = 1;
        }

        Ok(())
    }

    /// check if hit itself
    fn check_hit_itself(&mut self) -> Result<()> {
        // get head position
        let head = self.snake.body[0];

        // check if hit itself
        for pos in self.snake.body.iter().skip(1) {
            if head == *pos {
                self.game_over();
            }
        }

        Ok(())
    }

    /// game over
    fn game_over(&self) {
        let mut stdout = stdout();
        stdout.execute(crossterm::cursor::Show).unwrap();
        stdout
            .execute(crossterm::terminal::LeaveAlternateScreen)
            .unwrap();
        stdout.flush().unwrap();
        crossterm::terminal::disable_raw_mode().unwrap();

        let height = self.height + 4;

        stdout.queue(MoveTo(0, height - 1)).unwrap();

        // print game over
        stdout.queue(Print("\nGame Over\n")).unwrap();

        // print score
        stdout
            .queue(Print(format!("Score: {}\n", self.score)))
            .unwrap();

        // flush
        stdout.flush().unwrap();

        // exit
        std::process::exit(0);
    }

    fn clear_screen(&mut self) -> Result<()> {
        let mut stdout = stdout();

        stdout.queue(SetBackgroundColor(Color::Reset))?;

        for pos in &self.clear {
            stdout.queue(crossterm::cursor::MoveTo(pos.x, pos.y))?;
            stdout.queue(Print(" "))?;
        }

        self.clear.clear();

        Ok(())
    }

    /// Draw the game
    fn draw(&mut self) -> Result<()> {
        self.clear_screen()?;
        self.draw_frame()?;
        self.draw_score()?;
        self.draw_help()?;
        self.draw_snake()?;
        self.draw_food()?;

        let mut stdout = stdout();
        stdout.flush()?;

        Ok(())
    }

    fn draw_food(&self) -> Result<()> {
        let mut stdout = stdout();

        // Draw the food
        for pos in self.food.keys() {
            // TODO change color based on score
            stdout.queue(SetBackgroundColor(Color::Red))?;

            let pos: Position = *pos;
            stdout.queue(crossterm::cursor::MoveTo(pos.x * 2, pos.y))?;
            stdout.queue(Print(" "))?;
            stdout.queue(crossterm::cursor::MoveTo(pos.x * 2 - 1, pos.y))?;
            stdout.queue(Print(" "))?;
        }

        Ok(())
    }

    fn draw_frame(&self) -> Result<()> {
        let mut stdout = stdout();

        // Draw the frame of the game
        // Top line
        stdout.queue(SetBackgroundColor(Color::Reset))?;
        stdout.queue(crossterm::cursor::MoveTo(0, 0))?;
        stdout.queue(Print("╔"))?;
        for i in 1..self.width * 2 + 1 {
            stdout.queue(MoveTo(i, 0))?;
            stdout.queue(Print("═"))?;
        }
        stdout.queue(MoveTo(self.width * 2 + 1, 0))?;
        stdout.queue(Print("╗"))?;
        // line break
        // Middle lines
        for i in 1..self.height + 1 {
            stdout.queue(crossterm::cursor::MoveTo(0, i))?;
            stdout.queue(Print("║"))?;
            stdout.queue(crossterm::cursor::MoveTo(self.width * 2 + 1, i))?;
            stdout.queue(Print("║"))?;
        }
        // Bottom line
        stdout.queue(crossterm::cursor::MoveTo(0, self.height + 1))?;
        stdout.queue(Print("╚"))?;
        for i in 1..self.width * 2 + 1 {
            stdout.queue(MoveTo(i, self.height + 1))?;
            stdout.queue(Print("═"))?;
        }
        stdout.queue(MoveTo(self.width * 2 + 1, self.height + 1))?;
        stdout.queue(Print("╝"))?;

        Ok(())
    }

    fn draw_help(&self) -> Result<()> {
        let mut stdout = stdout();

        let help = "Move: ←↑→↓ Quit: q, Esc";

        for (i, c) in help.chars().enumerate() {
            stdout.queue(crossterm::cursor::MoveTo(i as u16, self.height + 3))?;
            stdout.queue(Print(c))?;
        }

        Ok(())
    }

    fn draw_score(&self) -> Result<()> {
        let mut stdout = stdout();

        let score = format!("Score: {}", self.score);
        for (i, c) in score.chars().enumerate() {
            stdout.queue(crossterm::cursor::MoveTo(i as u16, self.height + 2))?;
            stdout.queue(Print(c))?;
        }

        Ok(())
    }

    fn draw_snake(&self) -> Result<()> {
        let mut stdout = stdout();

        // Draw the snake
        stdout.queue(SetBackgroundColor(Color::Green))?;
        for pos in &self.snake.body {
            stdout.queue(crossterm::cursor::MoveTo(pos.x * 2 - 1, pos.y))?;
            stdout.queue(Print(" "))?;
            stdout.queue(crossterm::cursor::MoveTo(pos.x * 2, pos.y))?;
            stdout.queue(Print(" "))?;
        }
        stdout.queue(SetBackgroundColor(Color::Reset))?;

        Ok(())
    }

    /// generate food in random position that not in snake body
    fn generate_food(&mut self) {
        let max = if self.snake.body.len() > (self.width * self.height) as usize {
            0
        } else if FOOD_NUM + self.snake.body.len() > (self.width * self.height) as usize {
            (self.width * self.height) as usize - self.snake.body.len()
        } else {
            FOOD_NUM
        };
        for _ in self.food.len()..max {
            let p = self.food.len() + self.snake.body.len();
            let p = p as f32 / (self.width * self.height) as f32;
            let mut rng = rand::thread_rng();

            if p < 0.7 {
                let x = rng.gen_range(1..=self.width);
                let y = rng.gen_range(1..=self.height);
                let mut pos = Position { x, y };

                loop {
                    // if pos in snake body, generate new pos
                    if self.snake.body.contains(&pos) {
                        pos.x = rng.gen_range(1..=self.width);
                        pos.y = rng.gen_range(1..=self.height);
                        continue;
                    }

                    // if pos in foods, generate new pos
                    if self.food.contains_key(&pos) {
                        pos.x = rng.gen_range(1..=self.width);
                        pos.y = rng.gen_range(1..=self.height);
                        continue;
                    }

                    break;
                }

                let score = rng.gen_range(1..=FOOD_MAX_SCORE);
                self.food.insert(pos, score);
            } else {
                let mut all: HashSet<(u16, u16)> =
                    HashSet::from_iter((1..=self.width).zip(1..=self.height));
                for pos in &self.snake.body {
                    all.remove(&(pos.x, pos.y));
                }

                for pos in &self.food {
                    let pos: Position = *pos.0;
                    all.remove(&(pos.x, pos.y));
                }

                if all.is_empty() {
                    break;
                }

                let pos = all.iter().nth(rng.gen_range(0..all.len())).unwrap();
                let score = rng.gen_range(1..=FOOD_MAX_SCORE);

                self.food.insert(Position { x: pos.0, y: pos.1 }, score);
            }
        }
    }

    /// handle event
    fn handle_event(&mut self) -> Result<()> {
        let event = event::poll(std::time::Duration::from_millis(0))?;
        if event {
            if let event::Event::Key(e) = event::read()? {
                match e.code {
                    event::KeyCode::Char('q') => quit(),
                    event::KeyCode::Esc => quit(),
                    event::KeyCode::Up => {
                        if self.snake.direction == Direction::Up {
                            self.move_forward_once()?;
                        } else if self.snake.direction != Direction::Down {
                            self.snake.direction = Direction::Up
                        }
                    }
                    event::KeyCode::Down => {
                        if self.snake.direction == Direction::Down {
                            self.move_forward_once()?;
                        } else if self.snake.direction != Direction::Up {
                            self.snake.direction = Direction::Down
                        }
                    }
                    event::KeyCode::Left => {
                        if self.snake.direction == Direction::Left {
                            self.move_forward_once()?;
                        } else if self.snake.direction != Direction::Right {
                            self.snake.direction = Direction::Left
                        }
                    }
                    event::KeyCode::Right => {
                        if self.snake.direction == Direction::Right {
                            self.move_forward_once()?;
                        } else if self.snake.direction != Direction::Left {
                            self.snake.direction = Direction::Right
                        }
                    }
                    _ => {}
                }
            }
        }

        Ok(())
    }

    /// move snake
    fn move_snake(&mut self) -> Result<()> {
        // get timestamp in milliseconds
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();

        let interval = now - self.last_move;
        let pass = 1.0 / self.speed;
        let pass = pass.floor() as u128;

        if pass > interval {
            return Ok(());
        }

        let jump = (interval / pass) as u16;

        self.last_move += pass * jump as u128;

        for _ in 0..jump {
            self.move_forward_once()?;
        }

        Ok(())
    }

    /// move forward
    fn move_forward_once(&mut self) -> Result<()> {
        // get head position
        let head = self.snake.body[0];

        // get next position
        let next = match self.snake.direction {
            Direction::Up => Position {
                x: head.x,
                y: head.y - 1,
            },
            Direction::Down => Position {
                x: head.x,
                y: head.y + 1,
            },
            Direction::Left => Position {
                x: head.x - 1,
                y: head.y,
            },
            Direction::Right => Position {
                x: head.x + 1,
                y: head.y,
            },
        };

        // move snake
        self.snake.body.insert(0, next);

        // clear tail
        let tail = self.snake.body.pop().unwrap();
        self.clear.push(Position {
            x: tail.x * 2 - 1,
            y: tail.y,
        });
        self.clear.push(Position {
            x: tail.x * 2,
            y: tail.y,
        });

        self.check_hit_wall()?;
        self.check_eat_food()?;
        self.check_hit_itself()?;

        Ok(())
    }

    pub fn new(width: u16, height: u16) -> Self {
        let mut snake = Snake {
            body: vec![],
            direction: Direction::Right,
        };
        for i in (1..INIT_LENGTH + 1).rev() {
            snake.body.push(Position {
                x: i,
                y: height / 2,
            });
        }

        let mut game = Game {
            snake,
            food: HashMap::new(),
            width,
            height,
            score: 0,
            speed: INIT_SPEED,
            clear: vec![],
            last_move: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
        };

        game.generate_food();

        game
    }

    /// Run the game
    pub fn run(&mut self) -> Result<()> {
        let mut stdout = stdout();
        stdout.execute(crossterm::cursor::Hide)?;
        stdout.execute(crossterm::terminal::EnterAlternateScreen)?;
        crossterm::terminal::enable_raw_mode()?;

        // Draw the game
        self.draw()?;

        // Loop with interval
        loop_with_interval(UPDATES_INTERVAL, || {
            // Update game state
            self.update().unwrap();

            // Draw the game
            self.draw().unwrap();
        });

        Ok(())
    }

    /// update game state
    fn update(&mut self) -> Result<()> {
        // handle event
        self.handle_event()?;

        // update snake
        self.update_snake()?;

        Ok(())
    }

    /// update snake
    fn update_snake(&mut self) -> Result<()> {
        // move snake
        self.move_snake()?;

        Ok(())
    }
}

fn quit() {
    let mut stdout = stdout();
    stdout.execute(crossterm::cursor::Show).unwrap();
    stdout
        .execute(crossterm::terminal::LeaveAlternateScreen)
        .unwrap();
    stdout.flush().unwrap();
    crossterm::terminal::disable_raw_mode().unwrap();

    std::process::exit(0);
}
