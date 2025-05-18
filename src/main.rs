use std::{env::args, net::SocketAddrV4};

mod board;
mod direction;
mod game;
mod packet;
mod snake;
mod util;

use game::{GameMode, SnakeGame, SocketMode};

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
