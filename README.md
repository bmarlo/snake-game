## Snake Game

Multiplayer, terminal-based snake game in std Rust 🦀

Player is controlled by `WASD` + `Enter` (no raw mode).

## Singleplayer

`cargo run --release`

## Multiplayer

Server instance: `cargo run --release -- --accept <ip-addr>:<port>`

Client instance: `cargo run --release -- --connect <ip-addr>:<port>`
