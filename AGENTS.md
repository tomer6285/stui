# AGENTS.md

## Developer Commands
- Build: `go build -o stui stui.go`
- Run: `go run stui.go`

## Architecture & Conventions
- TUI framework: Bubble Tea (`charm.land/bubbletea/v2`).
- Configuration: Games list is managed in `~/.config/stui/games.json`.
- Launches games using `steam` command with `steam://rungameid/<ID>`.

## Known Issues
- `ScanSteamGames()` is called in `rescanGames()` but is not defined in the codebase.
