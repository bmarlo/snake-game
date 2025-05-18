use std::{
    collections::VecDeque,
    io::{
        stdin, ErrorKind, Read, Write
    },
    net::{
        SocketAddr, SocketAddrV4, TcpListener, TcpStream
    },
    sync::mpsc::channel,
    thread::{
        sleep, spawn
    },
    time::Duration
};

use crate::{
    board::{
        Board, BOARD_SIZE, CRASH_CHAR, OPPONENT_CHAR, PLAYER_CHAR, TARGET_CHAR
    },
    direction::Direction,
    packet::{
        Opcode, Packet, HEADER_SIZE
    },
    snake::Snake
};

const GAME_PACE: Duration = Duration::from_millis(350);

#[derive(Clone, Debug, PartialEq)]
pub enum GameMode {
    Singleplayer,
    Multiplayer(SocketMode),
}

#[derive(Clone, Debug, PartialEq)]
pub enum SocketMode {
    Client(SocketAddrV4),
    Server(SocketAddrV4),
}

#[derive(Clone, Debug, PartialEq)]
enum GameResult {
    Win(String),
    Lose(String),
    Draw(String)
}

pub struct SnakeGame {
    board: Board,
    player: Snake,
    target: VecDeque<(usize, usize)>,
    socket: Option<TcpStream>,
    opponent: Option<Snake>,
    queue: VecDeque<Packet>,
    tick_id: u64
}

impl SnakeGame {
    pub fn new(mode: GameMode) -> Self {
        let mut board = Board::new();

        let player;
        let target;
        let socket;
        let opponent;

        match mode {
            GameMode::Singleplayer => {
                let head = board.random_position().unwrap();
                player = Snake::new(head, Direction::random());
                board.mark(head, PLAYER_CHAR);

                target = board.random_position().unwrap();
                board.mark(target, TARGET_CHAR);

                socket = None;
                opponent = None;
            },
            GameMode::Multiplayer(mode) => {
                match mode {
                    SocketMode::Client(remote) => {
                        if !remote.ip().is_loopback() && !remote.ip().is_private() {
                            panic!("not a local/private IP address [SnakeGame::new()]");
                        }

                        let head = (1, 1);
                        player = Snake::new(head, Direction::Right);
                        board.mark(head, PLAYER_CHAR);

                        let head = (BOARD_SIZE - 2, BOARD_SIZE - 2);
                        opponent = Some(Snake::new(head, Direction::Left));
                        board.mark(head, OPPONENT_CHAR);

                        target = (BOARD_SIZE / 2, BOARD_SIZE / 2);
                        board.mark(target, TARGET_CHAR);

                        println!("Connecting to {}", remote);
                        socket = match TcpStream::connect(&SocketAddr::V4(remote)) {
                            Ok(stream) => Some(stream),
                            Err(error) => {
                                panic!("{} [SnakeGame::new()]", error.kind());
                            }
                        }
                    },
                    SocketMode::Server(local) => {
                        if !local.ip().is_loopback() && !local.ip().is_private() {
                            panic!("not a local/private IP address [SnakeGame::new()]");
                        }

                        let head = (BOARD_SIZE - 2, BOARD_SIZE - 2);
                        player = Snake::new(head, Direction::Left);
                        board.mark(head, PLAYER_CHAR);

                        let head = (1, 1);
                        opponent = Some(Snake::new(head, Direction::Right));
                        board.mark(head, OPPONENT_CHAR);

                        target = (BOARD_SIZE / 2, BOARD_SIZE / 2);
                        board.mark(target, TARGET_CHAR);

                        let server = match TcpListener::bind(local) {
                            Ok(server) => server,
                            Err(error) => {
                                panic!("{} [SnakeGame::new()]", error.kind());
                            }
                        };

                        let local = server.local_addr().unwrap();
                        println!("Accepting connection at {}", local);
                        socket = match server.accept() {
                            Ok((stream, _)) => Some(stream),
                            Err(error) => {
                                panic!("{} [SnakeGame::new()]", error.kind());
                            }
                        }
                    }
                }
            }
        }

        let mut deque = VecDeque::new();
        deque.push_back(target);

        SnakeGame { board, player, target: deque, socket, opponent, queue: VecDeque::new(), tick_id: 0 }
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

        let mut result = None;
        while result == None {
            self.tick_id += 1;

            match ctrl_rx.try_recv() {
                Ok(direction) => {
                    self.control(true, direction);
                    if self.is_multiplayer() {
                        self.send_control(direction);
                    }
                },
                Err(_) => {}
            }

            if self.is_multiplayer() {
                self.synchronize();

                loop {
                    match self.queue.pop_front() {
                        Some(packet) => {
                            self.process(&packet);
                        },
                        None => {
                            match self.recv_packet() {
                                Some(packet) => {
                                    self.process(&packet);
                                },
                                None => {
                                    break;
                                }
                            }
                        }
                    }
                }
            }

            result = self.update();
            println!("\x1b[2J\x1b[1;1H{}", self.board.draw());
            sleep(GAME_PACE);
        }

        match result.unwrap() {
            GameResult::Win(msg) => {
                println!("You won :D ({})", msg);
            },
            GameResult::Lose(msg) => {
                println!("You lost :/ ({})", msg);
            },
            GameResult::Draw(msg) => {
                println!("It's a draw ._. ({})", msg);
            }
        }

        println!("\x1b[?25h");
    }

    fn is_multiplayer(&self) -> bool {
        self.socket.is_some()
    }

    fn control(&mut self, own: bool, direction: Direction) {
        if own {
            self.player.control(direction);
        } else {
            match &mut self.opponent {
                Some(opponent) => {
                    opponent.control(direction);
                },
                None => {
                    panic!("unreachable [SnakeGame::control()]");
                }
            }
        }
    }

    fn update(&mut self) -> Option<GameResult> {
        let tail = self.player.tail();
        self.board.unmark(tail);
        self.player.update();

        let target = *self.target.front().unwrap();
        self.board.mark(target, TARGET_CHAR);

        let mut opponent_tail = None;
        match &mut self.opponent {
            Some(opponent) => {
                let tail = opponent.tail();
                opponent_tail = Some(tail);
                self.board.unmark(tail);
                opponent.update();

                if self.player.head() == opponent.head() {
                    self.board.mark(self.player.head(), CRASH_CHAR);
                    return Some(GameResult::Draw("heads crash".into()));
                }
            },
            None => {}
        }

        let pixel = self.board.value(self.player.head());
        if pixel == PLAYER_CHAR || pixel == OPPONENT_CHAR {
            match &mut self.opponent {
                Some(opponent) => {
                    self.board.mark(opponent.head(), OPPONENT_CHAR);
                },
                None => {}
            }

            self.board.mark(self.player.head(), CRASH_CHAR);
            return Some(GameResult::Lose("player crash".into()));
        }

        let mut opponent_grow = false;
        self.board.mark(self.player.head(), PLAYER_CHAR);

        match &mut self.opponent {
            Some(opponent) => {
                let pixel = self.board.value(opponent.head());
                if pixel == OPPONENT_CHAR || pixel == PLAYER_CHAR {
                    self.board.mark(opponent.head(), CRASH_CHAR);
                    return Some(GameResult::Win("opponent crash".into()));
                }

                self.board.mark(opponent.head(), OPPONENT_CHAR);
                if opponent.head() == target {
                    let tail = opponent_tail.unwrap();
                    opponent.grow(tail);

                    self.board.mark(tail, OPPONENT_CHAR);
                    if self.board.is_full() {
                        if self.player.size() > opponent.size() {
                            return Some(GameResult::Win("board full, player size wins".into()));
                        } else if self.player.size() < opponent.size() {
                            return Some(GameResult::Lose("board full, opponent size wins".into()));
                        } else {
                            return Some(GameResult::Draw("board full, same size".into()));
                        }
                    }

                    self.target.pop_front();
                    opponent_grow = true;
                }
            },
            None => {}
        }

        if !opponent_grow && self.player.head() == target {
            self.player.grow(tail);
            self.board.mark(tail, PLAYER_CHAR);

            let target = self.board.random_position();
            if target.is_none() {
                match &mut self.opponent {
                    Some(opponent) => {
                        if self.player.size() > opponent.size() {
                            return Some(GameResult::Win("board full, player size wins".into()));
                        } else if self.player.size() < opponent.size() {
                            return Some(GameResult::Lose("board full, opponent size wins".into()));
                        } else {
                            return Some(GameResult::Draw("board full, same size".into()));
                        }
                    },
                    None => {
                        return Some(GameResult::Win("board full".into()));
                    }
                }
            }

            let target = target.unwrap();
            self.board.mark(target, TARGET_CHAR);
            if self.is_multiplayer() {
                self.send_target(target);
            }

            self.target.push_back(target);
            self.target.pop_front();
        }

        None
    }

    fn synchronize(&mut self) {
        let mut packet = Packet::new(Opcode::Sync, 8);

        let mut data = [0; 8];
        data[0] = (self.tick_id >> 56) as u8;
        data[1] = (self.tick_id >> 48) as u8;
        data[2] = (self.tick_id >> 40) as u8;
        data[3] = (self.tick_id >> 32) as u8;
        data[4] = (self.tick_id >> 24) as u8;
        data[5] = (self.tick_id >> 16) as u8;
        data[6] = (self.tick_id >> 8) as u8;
        data[7] = (self.tick_id >> 0) as u8;

        packet.push_data(&data);
        self.send_packet(&packet);

        match &mut self.socket {
            Some(socket) => {
                match socket.set_nonblocking(false) {
                    Ok(_) => {},
                    Err(error) => {
                        panic!("{} [SnakeGame::synchronize()]", error.kind());
                    }
                }
            },
            None => {}
        }

        loop {
            match self.recv_packet() {
                Some(packet) => {
                    match packet.opcode() {
                        Opcode::Sync => {
                            let data = packet.data();
                            let mut tick_id: u64 = 0;
                            tick_id |= (data[0] as u64) << 56;
                            tick_id |= (data[1] as u64) << 48;
                            tick_id |= (data[2] as u64) << 40;
                            tick_id |= (data[3] as u64) << 32;
                            tick_id |= (data[4] as u64) << 24;
                            tick_id |= (data[5] as u64) << 16;
                            tick_id |= (data[6] as u64) << 8;
                            tick_id |= (data[7] as u64) << 0;

                            if tick_id == self.tick_id {
                                break;
                            }
                        },
                        _ => {
                            self.queue.push_back(packet);
                        }
                    }
                },
                None => {
                    panic!("unreachable [SnakeGame::synchronize()]");
                }
            }
        }

        match &mut self.socket {
            Some(socket) => {
                match socket.set_nonblocking(true) {
                    Ok(_) => {},
                    Err(error) => {
                        panic!("{} [SnakeGame::synchronize()]", error.kind());
                    }
                }
            },
            None => {}
        }
    }

    fn process(&mut self, packet: &Packet) {
        match packet.opcode() {
            Opcode::Sync => {
                panic!("unreachable [SnakeGame::process()]");
            },
            Opcode::NewDirection => {
                let data = packet.data();
                let direction = Direction::from(data[0]);
                self.control(false, direction);
            },
            Opcode::NewTarget => {
                let data = packet.data();
                let target = (data[0] as usize, data[1] as usize);
                self.target.push_back(target);
            }
        }
    }

    fn send_control(&mut self, direction: Direction) {
        let mut packet = Packet::new(Opcode::NewDirection, 1);
        packet.push_data(&[direction as u8]);
        self.send_packet(&packet);
    }

    fn send_target(&mut self, target: (usize, usize)) {
        if !(target.0 < BOARD_SIZE) || !(target.1 < BOARD_SIZE) {
            panic!("bad position [SnakeGame::send_target()]");
        }

        let mut packet = Packet::new(Opcode::NewTarget, 2);
        packet.push_data(&[target.0 as u8, target.1 as u8]);
        self.send_packet(&packet);
    }

    fn send_packet(&mut self, packet: &Packet) {
        match &mut self.socket {
            Some(socket) => {
                let buffer = packet.encode();
                match socket.write(&buffer) {
                    Ok(n) => {
                        if n != buffer.len() {
                            panic!("write() error [SnakeGame::send_packet()]");
                        }
                    },
                    Err(error) => {
                        panic!("{} [SnakeGame::send_packet()]", error.kind());
                    }
                }
            },
            None => {
                panic!("unreachable [SnakeGame::send_packet()]");
            }
        }
    }

    fn recv_packet(&mut self) -> Option<Packet> {
        match &mut self.socket {
            Some(socket) => {
                let mut buffer = vec![0; HEADER_SIZE];
                match socket.read(&mut buffer) {
                    Ok(n) => {
                        if n == 0 {
                            panic!("disconnected [SnakeGame::recv_packet()]");
                        }

                        if n != HEADER_SIZE {
                            panic!("read() error [SnakeGame::recv_packet()]");
                        }

                        let mut size: u16 = 0;
                        size |= (buffer[10] as u16) << 8;
                        size |= (buffer[11] as u16) << 0;

                        if size > 0 {
                            buffer.resize(HEADER_SIZE + size as usize, 0);
                            match socket.read(&mut buffer[HEADER_SIZE..]) {
                                Ok(n) => {
                                    if n != size as usize {
                                        panic!("read() error [SnakeGame::recv_packet()]");
                                    }
                                },
                                Err(error) => {
                                    panic!("{} [SnakeGame::recv_packet()]", error.kind());
                                }
                            }
                        }

                        match Packet::decode(&buffer) {
                            Some(packet) => {
                                Some(packet)
                            },
                            None => {
                                panic!("bad packet [SnakeGame::recv_packet()]");
                            }
                        }
                    },
                    Err(error) => {
                        if error.kind() != ErrorKind::WouldBlock && error.kind() != ErrorKind::TimedOut {
                            panic!("{} [SnakeGame::recv_packet()]", error.kind());
                        }

                        None
                    }
                }
            },
            None => {
                panic!("unreachable [SnakeGame::recv_packet()]");
            }
        }
    }
}
