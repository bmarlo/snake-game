use std::{io::stdin, sync::{mpsc::channel, Mutex}, thread::{sleep, spawn}, time::{Duration, SystemTime, UNIX_EPOCH}};

const BOARD_SIZE: usize = 16;
const PLAYER_CHAR: char = '+';
const TARGET_CHAR: char = 'o';
const GAME_PACE: Duration = Duration::from_millis(350);

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Direction {
    Right,
    Down,
    Left,
    Up
}

pub struct SnakeGame {
    board: Vec<Vec<char>>,
    player: Vec<(usize, usize)>,
    target: (usize, usize),
    direction: Direction
}

static HASH: Mutex<u64> = Mutex::new(0xcbf29ce484222325);

pub fn random_number() -> u64 {
    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::new(0, 0))
        .subsec_nanos();

    let mut value = HASH.lock().unwrap();
    for i in 0..4 {
        *value *= 0x100000001b3;
        *value ^= ((seed >> (3 - i) * 8) as u8) as u64;
    }

    *value
}

fn random_position() -> (usize, usize) {
    let value = random_number() as usize;
    (((value >> 8) & 0xff) % BOARD_SIZE, (value & 0xff) % BOARD_SIZE)
}

fn random_direction() -> Direction {
    match random_number() % 4 {
        0 => Direction::Right,
        1 => Direction::Down,
        2 => Direction::Left,
        _ => Direction::Up
    }
}

impl SnakeGame {
    pub fn new() -> Self {
        let mut board = Vec::new();
        for _ in 0..BOARD_SIZE {
            let mut row = Vec::new();
            for _ in 0..BOARD_SIZE {
                row.push(' ');
            }
            board.push(row);
        }

        let head = random_position();
        board[head.0][head.1] = PLAYER_CHAR;

        let mut target;
        loop {
            target = random_position();
            if board[target.0][target.1] == ' ' {
                board[target.0][target.1] = TARGET_CHAR;
                break;
            }
        }

        SnakeGame { board, player: vec![head], target, direction: random_direction() }
    }

    pub fn play(&mut self) {
        println!("\x1b[?25l");
        let (ctrl_tx, ctrl_rx) = channel::<Direction>();

        spawn(move || {
            loop {
                let mut line = String::new();
                stdin().read_line(&mut line).unwrap();
                match line.trim() {
                    "d" => {
                        ctrl_tx.send(Direction::Right).unwrap();
                    },
                    "s" => {
                        ctrl_tx.send(Direction::Down).unwrap();
                    },
                    "a" => {
                        ctrl_tx.send(Direction::Left).unwrap();
                    },
                    "w" => {
                        ctrl_tx.send(Direction::Up).unwrap();
                    }
                    _ => {}
                }
            }
        });

        let mut lose = false;
        while !self.is_win() && !lose {
            match ctrl_rx.try_recv() {
                Ok(direction) => {
                    self.control(direction);
                },
                Err(_) => {}
            }

            lose = self.update();
            self.draw();
            sleep(GAME_PACE);
        }

        if lose {
            println!("You lost :/");
        } else {
            println!("You won :D");
        }

        println!("\x1b[?25h");
    }

    fn is_win(&self) -> bool {
        for row in &self.board {
            for pixel in row {
                if *pixel == ' ' {
                    return false;
                }
            }
        }

        true
    }

    fn control(&mut self, direction: Direction) {
        match self.direction {
            Direction::Right => {
                if direction != Direction::Left {
                    self.direction = direction;
                }
            },
            Direction::Down => {
                if direction != Direction::Up {
                    self.direction = direction;
                }
            },
            Direction::Left => {
                if direction != Direction::Right {
                    self.direction = direction;
                }
            },
            Direction::Up => {
                if direction != Direction::Down {
                    self.direction = direction;
                }
            }
        }
    }

    fn update(&mut self) -> bool {
        let tail = self.player[self.player.len() - 1];
        self.board[tail.0][tail.1] = ' ';

        for i in 0..self.player.len() {
            let i = self.player.len() - i - 1;
            if i > 0 {
                self.player[i] = self.player[i - 1];
            } else {
                let pixel = self.player[i];
                self.player[i] = match self.direction {
                    Direction::Right => (pixel.0, (pixel.1 + 1) % BOARD_SIZE),
                    Direction::Down => ((pixel.0 + 1) % BOARD_SIZE, pixel.1),
                    Direction::Left => (pixel.0, (pixel.1 - 1) % BOARD_SIZE),
                    Direction::Up => ((pixel.0 - 1) % BOARD_SIZE, pixel.1)
                };

                let head = self.player[i];
                if self.board[head.0][head.1] == PLAYER_CHAR {
                    return true
                }

                self.board[head.0][head.1] = PLAYER_CHAR;
            }
        }

        let head = self.player[0];
        if head == self.target {
            self.player.push(tail);
            self.board[tail.0][tail.1] = PLAYER_CHAR;

            loop {
                let target = random_position();
                if self.board[target.0][target.1] == ' ' {
                    self.board[target.0][target.1] = TARGET_CHAR;
                    self.target = target;
                    break;
                }
            }
        }

        false
    }

    fn draw(&self) {
        let mut s = String::new();
        s.push_str("\x1b[2J");
        s.push_str("\x1b[1;1H");

        s.push('+');
        for _ in 0..BOARD_SIZE {
            s.push(' ');
            s.push('+');
            s.push(' ');
        }

        s.push('+');
        s.push('\n');
        for row in &self.board {
            s.push('+');
            for pixel in row {
                s.push(' ');
                s.push(*pixel);
                s.push(' ');
            }
            s.push('+');
            s.push('\n');
        }

        s.push('+');
        for _ in 0..BOARD_SIZE {
            s.push(' ');
            s.push('+');
            s.push(' ');
        }

        s.push('+');
        s.push('\n');
        println!("{}", s);
    }
}

fn main() {
    let mut game = SnakeGame::new();
    game.play();
}
