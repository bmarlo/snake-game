use crate::{board::BOARD_SIZE, direction::Direction};

pub struct Snake {
    body: Vec<(usize, usize)>,
    direction: Direction
}

impl Snake {
    pub fn new(head: (usize, usize), direction: Direction) -> Self {
        Snake { body: vec![head], direction }
    }

    pub fn head(&self) -> (usize, usize) {
        self.body[0]
    }

    pub fn tail(&self) -> (usize, usize) {
        self.body[self.body.len() - 1]
    }

    pub fn grow(&mut self, tail: (usize, usize)) {
        self.body.push(tail);
    }

    pub fn size(&self) -> usize {
        self.body.len()
    }

    pub fn control(&mut self, direction: Direction) {
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

    pub fn update(&mut self) {
        for i in 0..self.body.len() {
            let i = self.body.len() - i - 1;
            if i > 0 {
                self.body[i] = self.body[i - 1];
            } else {
                let head = self.body[i];
                self.body[i] = match self.direction {
                    Direction::Right => (head.0, (head.1 + 1) % BOARD_SIZE),
                    Direction::Down => ((head.0 + 1) % BOARD_SIZE, head.1),
                    Direction::Left => (head.0, if head.1 > 0 { head.1 - 1 } else { BOARD_SIZE - 1 }),
                    Direction::Up => (if head.0 > 0 { head.0 - 1 } else { BOARD_SIZE - 1 }, head.1)
                };
            }
        }
    }
}
