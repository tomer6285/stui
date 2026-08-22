# Rust Rewrite Plan: `stui`

This document details the migration path and design plan to rewrite the `stui` Steam TUI launcher from Go to Rust.

---

## 1. Crates & Dependencies Mapping

The Go project relies on a few core mechanisms: Bubble Tea for the TUI, built-in regex/JSON for data parsing, and OS-specific path parsing. The table below maps these to Rust crates:

| Go Package / Feature | Rust Equivalent | Rationale |
| :--- | :--- | :--- |
| `charm.land/bubbletea/v2` | `ratatui` + `crossterm` | `ratatui` is the industry standard for modern Rust TUIs. We will use `crossterm` as the backend for terminal control and input event listening. |
| `encoding/json` | `serde` + `serde_json` | The standard way to deserialize and serialize structs in Rust. |
| `regexp` | `regex` | Fast, standard regular expressions for parsing `.acf` files and localconfig files. |
| `os.UserHomeDir()` & `filepath` | `dirs` | The `dirs` crate provides reliable multi-platform paths for home and configuration directories. |
| `os/exec` | `std::process::Command` | Standard library process spawning to launch `steam` commands. |
| `flag` | `clap` (or `argh`) | Robust command line argument parsing. |

---

## 2. Architecture & State Management

### Go Architecture (Bubble Tea)
Go uses the Elm architecture pattern:
- **Model**: `model` struct containing the list of games, search state, cursor index, steam online status.
- **Update**: `Update(msg)` matches on keyboard events, toggles favorite/hidden status, launches game process, and updates cursor state.
- **View**: `View()` constructs the terminal string rendering (favorites first, alphabetically, highlighting cursor).

### Rust Architecture (Ratatui + Crossterm)
We will implement an event-driven loop in Rust:
- **`App` State Struct**: Contains `games: Vec<Game>`, `cursor: usize`, `search_query: String`, `searching: bool`, `show_hidden: bool`, `quit_after_launch: bool`, and terminal dimensions.
- **Event Loop**: Listens to crossterm events (keyboard inputs, window resizing) and executes state updates accordingly.
- **UI Rendering Function**: Uses Ratatui's `Frame` to draw components:
  - A header block with Steam status.
  - A list block containing games (formatted with custom styles for favorites and cursor location).
  - A footer block showing help options.

---

## 3. Data Structures & Serializations

```rust
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Game {
    pub name: String,
    pub id: String,
    pub hidden: bool,
    pub playtime: u32, // in minutes
    pub favorite: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct State {
    pub last_selected_id: String,
}
```

---

## 4. Implementation Steps

### Step 1: Project Initialization & Cargo Setup
1. Run `cargo init` to scaffold the project.
2. Update `Cargo.toml` with the required dependencies:
   ```toml
   [dependencies]
   ratatui = "0.26"
   crossterm = "0.27"
   serde = { version = "1.0", features = ["derive"] }
   serde_json = "1.0"
   regex = "1.10"
   dirs = "5.0"
   clap = { version = "4.4", features = ["derive"] }
   ```

### Step 2: Storage & File Parsing Utilities
1. **Config Dir Setup**: Implement directory creation in `~/.config/stui/`.
2. **ACF Parsing (`ScanSteamGames`)**:
   - Read files from `~/.local/share/Steam/steamapps` matching `*.acf`.
   - Parse `"appid"` and `"name"` using regex.
3. **Localconfig Parsing (`getPlaytimesMap` & `getSteamOnlineStatus`)**:
   - Extract playtimes and persona status using regex matching on the `.vdf` structure.

### Step 3: Application Event Loop & State
1. Define the `App` struct representing the terminal application state.
2. Implement methods:
   - `App::new(games: Vec<Game>, quit_after_launch: bool, last_selected_id: Option<String>) -> Self`
   - `App::next_item()`, `App::prev_item()` (respecting favorites and hidden/search filtering).
   - `App::toggle_favorite()`, `App::toggle_hidden()`.

### Step 4: TUI & Rendering (Ratatui)
1. Initialize the terminal using `crossterm` raw mode and terminal buffering.
2. Build the main layout (header, game list, search input, footer).
3. Style favorites with bold text and/or colors. Use a highlight style (like `>` indicator or reverse colors) for the cursor.

### Step 5: Game Launching
1. Spawning Steam process using `std::process::Command`:
   ```rust
   std::process::Command::new("steam")
       .arg(format!("steam://rungameid/{}", game.id))
       .spawn()?;
   ```

---

## 5. Verification & Testing

1. **Dry-Run Validation**: Test parsing capabilities by logging list items to console.
2. **Interactive TUI testing**: Run under different terminal sizes and verify the viewport list constraint logic works seamlessly.
