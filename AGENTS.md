# AGENTS.md

## Developer Commands
- Build: `cargo build` (Release: `cargo build --release`)
- Run: `cargo run` (e.g. `cargo run -- -q`)
- Test: `cargo test`

## Architecture & Conventions
- Language: Rust (2021 edition)
- TUI framework: Ratatui (`ratatui`) + Crossterm (`crossterm`)
- CLI Parser: Clap (`clap`)
- Serialization: Serde (`serde`, `serde_json`)
- Configuration: Managed in `~/.config/stui/games.json` and `~/.config/stui/state.json`.
- Game scanning: Scans `~/.local/share/Steam/steamapps/*.acf` and reads playtimes / persona state from `~/.local/share/Steam/userdata/<user>/config/localconfig.vdf`.
- Launches games using `steam` command with `steam://rungameid/<ID>`.

