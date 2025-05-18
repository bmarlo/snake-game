use crate::util::random_number;

pub const BOARD_SIZE: usize = 8;
pub const PLAYER_CHAR: char = '+';
pub const OPPONENT_CHAR: char = '-';
pub const TARGET_CHAR: char = 'o';
pub const CRASH_CHAR: char = 'x';

pub struct Board {
    pixels: Vec<Vec<char>>
}

impl Board {
    pub fn new() -> Self {
        let mut pixels = Vec::new();
        for _ in 0..BOARD_SIZE {
            let mut row = Vec::new();
            for _ in 0..BOARD_SIZE {
                row.push(' ');
            }
            pixels.push(row);
        }

        Board { pixels }
    }

    pub fn mark(&mut self, pos: (usize, usize), value: char) {
        self.pixels[pos.0][pos.1] = value;
    }

    pub fn unmark(&mut self, pos: (usize, usize)) {
        self.pixels[pos.0][pos.1] = ' ';
    }

    pub fn value(&self, pos: (usize, usize)) -> char {
        self.pixels[pos.0][pos.1]
    }

    pub fn is_full(&self) -> bool {
        for row in &self.pixels {
            for pixel in row {
                if *pixel == ' ' || *pixel == TARGET_CHAR {
                    return false;
                }
            }
        }

        true
    }

    pub fn random_position(&self) -> Option<(usize, usize)> {
        let mut available = Vec::new();
        for i in 0..BOARD_SIZE {
            for j in 0..BOARD_SIZE {
                if self.pixels[i][j] == ' ' {
                    available.push((i, j));
                }
            }
        }

        match available.is_empty() {
            false => Some(available[random_number() as usize % available.len()]),
            true => None
        }
    }

    pub fn draw(&self) -> String {
        let mut s = String::new();

        s.push('+');
        for _ in 0..BOARD_SIZE {
            s.push(' ');
            s.push('+');
            s.push(' ');
        }

        s.push('+');
        s.push('\n');
        for row in &self.pixels {
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
        s
    }
}
