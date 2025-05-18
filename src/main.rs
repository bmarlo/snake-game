use std::{
    collections::VecDeque,
    env::args,
    io::{
        stdin, ErrorKind, Read, Write
    },
    net::{
        SocketAddr, SocketAddrV4, TcpListener, TcpStream
    },
    sync::{
        mpsc::channel, Mutex
    },
    thread::{
        sleep, spawn
    },
    time::{
        Duration, SystemTime, UNIX_EPOCH
    },
    vec
};

const BOARD_SIZE: usize = 16;
const PLAYER_CHAR: char = '+';
const OPPONENT_CHAR: char = '-';
const TARGET_CHAR: char = 'o';
const CRASH_CHAR: char = 'x';
const GAME_PACE: Duration = Duration::from_millis(350);

#[derive(Clone, Debug, PartialEq)]
enum GameMode {
    Singleplayer,
    Multiplayer(SocketMode),
}

#[derive(Clone, Debug, PartialEq)]
enum SocketMode {
    Client(SocketAddrV4),
    Server(SocketAddrV4),
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Direction {
    Right,
    Down,
    Left,
    Up
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum GameResult {
    Win,
    Lose,
    Draw
}

const PROTOCOL_ID: u64 = 0xaefdb87fe753ba07;
const HEADER_SIZE: usize = 12;

#[derive(Clone, Copy, Debug, PartialEq)]
enum Opcode {
    Sync = 0x01,
    NewDirection,
    NewTarget
}

struct Board {
    pixels: Vec<Vec<char>>
}

struct Packet {
    opcode: Opcode,
    data: Vec<u8>
}

struct Snake {
    body: Vec<(usize, usize)>,
    direction: Direction
}

struct SnakeGame {
    board: Board,
    player: Snake,
    target: VecDeque<(usize, usize)>,
    socket: Option<TcpStream>,
    opponent: Option<Snake>,
    queue: VecDeque<Packet>,
    tick_id: u64
}

impl Board {
    fn new() -> Self {
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

    fn mark(&mut self, pos: (usize, usize), value: char) {
        self.pixels[pos.0][pos.1] = value;
    }

    fn unmark(&mut self, pos: (usize, usize)) {
        self.pixels[pos.0][pos.1] = ' ';
    }

    fn value(&self, pos: (usize, usize)) -> char {
        self.pixels[pos.0][pos.1]
    }

    fn is_full(&self) -> bool {
        for row in &self.pixels {
            for pixel in row {
                if *pixel == ' ' {
                    return false;
                }
            }
        }

        true
    }

    fn random_position(&self) -> Option<(usize, usize)> {
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

    fn draw(&self) -> String {
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

impl Direction {
    fn from(value: u8) -> Direction {
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
}

impl Snake {
    fn new(head: (usize, usize), direction: Direction) -> Self {
        Snake { body: vec![head], direction }
    }

    fn head(&self) -> (usize, usize) {
        self.body[0]
    }

    fn tail(&self) -> (usize, usize) {
        self.body[self.body.len() - 1]
    }

    fn grow(&mut self, tail: (usize, usize)) {
        self.body.push(tail);
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

    fn update(&mut self) {
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

impl Packet {
    fn new(opcode: Opcode, size: usize) -> Packet {
        Packet { opcode, data: Vec::with_capacity(size) }
    }

    fn push_data(&mut self, data: &[u8]) {
        if self.data.len() + data.len() > u16::MAX as usize {
            panic!("bad data size [Packet::push_data()]");
        }

        self.data.extend_from_slice(data);
    }

    fn opcode(&self) -> Opcode {
        self.opcode
    }

    fn data(&self) -> &Vec<u8> {
        &self.data
    }

    fn encode(&self) -> Vec<u8> {
        let size = HEADER_SIZE + self.data.len();
        let mut buffer = Vec::with_capacity(size);

        buffer.push((PROTOCOL_ID >> 56) as u8);
        buffer.push((PROTOCOL_ID >> 48) as u8);
        buffer.push((PROTOCOL_ID >> 40) as u8);
        buffer.push((PROTOCOL_ID >> 32) as u8);
        buffer.push((PROTOCOL_ID >> 24) as u8);
        buffer.push((PROTOCOL_ID >> 16) as u8);
        buffer.push((PROTOCOL_ID >> 8) as u8);
        buffer.push((PROTOCOL_ID >> 0) as u8);

        buffer.push((self.opcode as u16 >> 8) as u8);
        buffer.push((self.opcode as u16 >> 0) as u8);

        let size = self.data.len();
        buffer.push((size >> 8) as u8);
        buffer.push((size >> 0) as u8);

        buffer.extend_from_slice(&self.data);
        buffer
    }

    fn decode(buffer: &[u8]) -> Option<Packet> {
        if buffer.len() < HEADER_SIZE {
            return None;
        }

        let mut protocol_id: u64 = 0;
        protocol_id |= (buffer[0] as u64) << 56;
        protocol_id |= (buffer[1] as u64) << 48;
        protocol_id |= (buffer[2] as u64) << 40;
        protocol_id |= (buffer[3] as u64) << 32;
        protocol_id |= (buffer[4] as u64) << 24;
        protocol_id |= (buffer[5] as u64) << 16;
        protocol_id |= (buffer[6] as u64) << 8;
        protocol_id |= (buffer[7] as u64) << 0;

        if protocol_id != PROTOCOL_ID {
            return None;
        }

        let mut opcode: u16 = 0;
        opcode |= (buffer[8] as u16) << 8;
        opcode |= (buffer[9] as u16) << 0;

        let opcode = match opcode {
            0x01 => {
                Opcode::Sync
            },
            0x02 => {
                Opcode::NewDirection
            },
            0x03 => {
                Opcode::NewTarget
            },
            _ => {
                return None;
            }
        };

        let mut size: u16 = 0;
        size |= (buffer[10] as u16) << 8;
        size |= (buffer[11] as u16) << 0;

        if size as usize != buffer.len() - HEADER_SIZE {
            return None;
        }

        let mut packet = Packet::new(opcode, size as usize);
        packet.data.extend_from_slice(&buffer[HEADER_SIZE..]);
        Some(packet)
    }
}

static HASH: Mutex<u64> = Mutex::new(0xcbf29ce484222325);

fn random_number() -> u64 {
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

fn random_direction() -> Direction {
    match random_number() % 4 {
        0 => Direction::Right,
        1 => Direction::Down,
        2 => Direction::Left,
        _ => Direction::Up
    }
}

impl SnakeGame {
    fn new(mode: GameMode) -> Self {
        let mut board = Board::new();

        let player;
        let target;
        let socket;
        let opponent;

        match mode {
            GameMode::Singleplayer => {
                let head = board.random_position().unwrap();
                player = Snake::new(head, random_direction());
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

    fn play(&mut self) {
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
            GameResult::Win => {
                println!("You won :D");
            },
            GameResult::Lose => {
                println!("You lost :/");
            },
            GameResult::Draw => {
                println!("It's a draw ._.");
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

        let mut opponent_tail = None;
        match &mut self.opponent {
            Some(opponent) => {
                let tail = opponent.tail();
                opponent_tail = Some(tail);
                self.board.unmark(tail);
                opponent.update();

                if self.player.head() == opponent.head() {
                    self.board.mark(self.player.head(), CRASH_CHAR);
                    return Some(GameResult::Draw);
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
            return Some(GameResult::Lose);
        }

        let mut opponent_grow = false;
        self.board.mark(self.player.head(), PLAYER_CHAR);

        match &mut self.opponent {
            Some(opponent) => {
                let pixel = self.board.value(opponent.head());
                if pixel == OPPONENT_CHAR || pixel == PLAYER_CHAR {
                    self.board.mark(opponent.head(), CRASH_CHAR);
                    return Some(GameResult::Win);
                }

                self.board.mark(opponent.head(), OPPONENT_CHAR);
                if opponent.head() == *self.target.front().unwrap() {
                    let tail = opponent_tail.unwrap();
                    opponent.grow(tail);
                    self.board.mark(tail, OPPONENT_CHAR);
                    if self.board.is_full() {
                        return Some(GameResult::Lose);
                    }

                    self.target.pop_front();
                    opponent_grow = true;
                }
            },
            None => {}
        }

        if !opponent_grow && self.player.head() == *self.target.front().unwrap() {
            self.player.grow(tail);
            self.board.mark(tail, PLAYER_CHAR);

            let target = self.board.random_position();
            if target.is_none() {
                return Some(GameResult::Win);
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
                self.board.mark(target, TARGET_CHAR);
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

                        if n != buffer.len() {
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

fn main() {
    let usage = || {
        println!("Usage: {{--accept <interface>:<port> | --connect <host>:<port>}}");
    };

    let args: Vec<String> = args().collect();
    let mode = match args.len() {
        1 => GameMode::Singleplayer,
        3 => {
            match &args[1] as &str {
                "--connect" => {
                    let remote: SocketAddrV4 = match args[2].parse() {
                        Ok(addr) => addr,
                        Err(_) => {
                            usage();
                            return;
                        }
                    };
                    GameMode::Multiplayer(SocketMode::Client(remote))
                },
                "--accept" => {
                    let local: SocketAddrV4 = match args[2].parse() {
                        Ok(addr) => addr,
                        Err(_) => {
                            usage();
                            return;
                        }
                    };
                    GameMode::Multiplayer(SocketMode::Server(local))
                },
                _ => {
                    usage();
                    return;
                }
            }
        },
        _ => {
            usage();
            return;
        }
    };

    let mut game = SnakeGame::new(mode);
    game.play();
}
