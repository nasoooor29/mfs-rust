# Maze Runner

A small authoritative multiplayer maze game. The server generates and validates a reproducible maze; Bevy renders the client as a 3D first-person game.

## Run

```bash
cargo run -p server -- bind_addr=0.0.0.0:34254 difficulty=medium seed=12345 empty=1
cargo run -p client
```

The optional server arguments are `[bind-address] [easy|medium|hard] [seed]`. The server binds all interfaces in the example, so other machines can connect using the host's LAN address. Start additional clients to play locally.

Controls: `WASD` moves, the mouse looks around, the arrow keys provide keyboard look controls, and Space shoots. Press Escape to release the mouse and click the game window to capture it again. Close the client window to quit; use Ctrl-C to stop the server.

## Verify

```bash
cargo test
cargo clippy --all-targets -- -D warnings
```
