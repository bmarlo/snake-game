use crate::util::random_number;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Direction {
    Right,
    Down,
    Left,
    Up
}

impl Direction {
    pub fn from(value: u8) -> Direction {
        match value {
            0x00 => {
                Direction::Right
            },
            0x01 => {
                Direction::Down
            },
            0x02 => {
                Direction::Left
            },
            0x03 => {
                Direction::Up
            },
            _ => {
                panic!("bad direction [Direction::from()]");
            }
        }
    }

    pub fn random() -> Direction {
        match random_number() % 4 {
            0 => Direction::Right,
            1 => Direction::Down,
            2 => Direction::Left,
            _ => Direction::Up
        }
    }
}
