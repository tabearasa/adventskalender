# Snowman Catch

A small terminal game for the Advent season: catch falling snowflakes with your
basket to build a snowman row by row, and dodge the red embers that melt it.

```
      ___
     |   |
    _|___|_
     (o o)
     ( > )
  --(  :  )--
   (   :   )
  (    :    )
  (         )
   '-------'
```

## Requirements

- [Rust](https://www.rust-lang.org/tools/install) 1.85 or newer (the crate uses
  the 2024 edition). Check with `cargo --version`.
- A terminal of at least **66 × 24** characters, with Unicode and color support.
  Most modern terminals on Linux, macOS and Windows work.

## Build and run

Run the game straight from the project folder:

```sh
cargo run --release
```

Or build it once and start the binary directly:

```sh
cargo build --release
./target/release/snowman-catch
```

To install it into `~/.cargo/bin` so you can start it from anywhere:

```sh
cargo install --path .
snowman-catch
```

## How to play

Catch snowflakes (`*`, `+`, `.`) in your basket. Every 4 flakes finish one row
of the snowman; 10 rows (40 flakes) complete it and win the game.

Red embers (`o`) melt 2 flakes and cost one of your 3 lives. Lose all lives and
your snowman melts. The higher your snowman grows, the faster things fall and
the more embers appear.

| Key                  | Action              |
| -------------------- | ------------------- |
| `←` / `→`, `a` / `d`, `h` / `l` | Move the basket |
| `Enter` / `Space`    | Start / play again  |
| `p`                  | Pause / continue    |
| `r`                  | Restart             |
| `q` / `Esc` / `Ctrl+C` | Quit              |

## Tests

The game rules in `src/game.rs` are independent of the terminal and have unit
tests:

```sh
cargo test
```

## Release

Pushing a version tag builds the Linux `.tar.gz`, the Windows `.exe` and the
macOS `.pkg`/`.tar.gz` and attaches them to a GitHub release
(see `.github/workflows/release.yml`):

```sh
git tag v0.1.0
git push origin v0.1.0
```

## Project layout

- `src/main.rs` – terminal setup, input handling and rendering (via
  [crossterm](https://crates.io/crates/crossterm))
- `src/game.rs` – game state and rules
