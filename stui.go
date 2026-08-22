package main

import (
	"encoding/json"
	"flag"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"regexp"
	"sort"
	"strings"

	tea "charm.land/bubbletea/v2"
)

type Game struct {
	Name     string
	ID       string
	Hidden   bool
	Playtime int
	Favorite bool
}

type State struct {
	LastSelectedID string `json:"last_selected_id"`
}

func loadState() State {
	home, _ := os.UserHomeDir()
	file := filepath.Join(home, ".config/stui", "state.json")
	read, err := os.ReadFile(file)
	if err != nil {
		return State{}
	}
	var state State
	_ = json.Unmarshal(read, &state)
	return state
}

func saveState(lastSelectedID string) {
	home, _ := os.UserHomeDir()
	file := filepath.Join(home, ".config/stui", "state.json")
	state := State{LastSelectedID: lastSelectedID}
	data, err := json.Marshal(state)
	if err == nil {
		_ = os.WriteFile(file, data, 0644)
	}
}

type model struct {
	games           []Game // items on the to-do list
	cursor          int    // which to-do list item our cursor is pointing at
	searching       bool   // whether we are currently searching
	searchQuery     string // the current search query
	showHidden      bool   // whether to show hidden games instead of visible ones
	quitAfterLaunch bool   // whether to quit after launching a game
	windowHeight    int    // height of the terminal window
	steamOnline     bool   // whether steam status is online or offline
}

func getPlaytimesMap() (map[string]int, error) {
	home, _ := os.UserHomeDir()
	userdataPath := filepath.Join(home, ".local/share/Steam/userdata")
	entries, err := os.ReadDir(userdataPath)
	if err != nil || len(entries) == 0 {
		return nil, fmt.Errorf("could not read userdata directory")
	}

	// Use the first user directory found
	userDir := entries[0].Name()
	configPath := filepath.Join(userdataPath, userDir, "config", "localconfig.vdf")
	content, err := os.ReadFile(configPath)
	if err != nil {
		return nil, err
	}

	playtimes := make(map[string]int)
	// This regex looks for appids and their associated Playtime in localconfig.vdf
	// It expects "appid" { ... "Playtime" "value" ... }
	re := regexp.MustCompile(`"(\d+)"\s+\{\s+[^}]*"Playtime"\s+"(\d+)"`)
	matches := re.FindAllStringSubmatch(string(content), -1)

	for _, match := range matches {
		if len(match) == 3 {
			var pt int
			fmt.Sscanf(match[2], "%d", &pt)
			playtimes[match[1]] = pt
		}
	}

	return playtimes, nil
}

func ScanSteamGames() ([]Game, error) {
	home, _ := os.UserHomeDir()
	steamapps := filepath.Join(home, ".local/share/Steam/steamapps")
	files, err := os.ReadDir(steamapps)
	if err != nil {
		return nil, err
	}

	playtimes, _ := getPlaytimesMap()

	var games []Game
	reID := regexp.MustCompile(`"appid"\s+"(\d+)"`)
	reName := regexp.MustCompile(`"name"\s+"([^"]+)"`)

	for _, file := range files {
		if !file.IsDir() && filepath.Ext(file.Name()) == ".acf" {
			content, err := os.ReadFile(filepath.Join(steamapps, file.Name()))
			if err != nil {
				continue
			}

			idMatch := reID.FindStringSubmatch(string(content))
			nameMatch := reName.FindStringSubmatch(string(content))

			if len(idMatch) > 1 && len(nameMatch) > 1 {
				games = append(games, Game{
					ID:       idMatch[1],
					Name:     nameMatch[1],
					Playtime: playtimes[idMatch[1]],
				})
			}
		}
	}
	return games, nil
}

func refreshPlaytimes(games []Game) []Game {
	ptMap, err := getPlaytimesMap()
	if err != nil {
		return games
	}
	for i := range games {
		if pt, ok := ptMap[games[i].ID]; ok {
			games[i].Playtime = pt
		}
	}
	return games
}

func saveGames(games []Game) error {
	home, _ := os.UserHomeDir()
	path := filepath.Join(home, ".config/stui")
	file := filepath.Join(path, "games.json")

	data, err := json.MarshalIndent(games, "", "  ")
	if err != nil {
		return err
	}

	return os.WriteFile(file, data, 0644)
}

func snapCursorToFirstMatch(m *model) {
	// Build visual order (favorites first, then others, alphabetically)
	var favs, others []int
	for i, game := range m.games {
		if game.Hidden == m.showHidden && (m.searchQuery == "" || strings.Contains(strings.ToLower(game.Name), strings.ToLower(m.searchQuery))) {
			if game.Favorite {
				favs = append(favs, i)
			} else {
				others = append(others, i)
			}
		}
	}
	sort.Slice(favs, func(i, j int) bool { return m.games[favs[i]].Name < m.games[favs[j]].Name })
	sort.Slice(others, func(i, j int) bool { return m.games[others[i]].Name < m.games[others[j]].Name })

	visual := append(favs, others...)
	if len(visual) > 0 {
		m.cursor = visual[0]
	} else {
		m.cursor = 0
	}
	if len(m.games) > 0 && m.cursor >= 0 && m.cursor < len(m.games) {
		saveState(m.games[m.cursor].ID)
	}
}

func getSteamOnlineStatus() bool {
	home, _ := os.UserHomeDir()
	userdataPath := filepath.Join(home, ".local/share/Steam/userdata")
	entries, err := os.ReadDir(userdataPath)
	if err != nil || len(entries) == 0 {
		return true
	}

	userDir := entries[0].Name()
	configPath := filepath.Join(userdataPath, userDir, "config", "localconfig.vdf")
	content, err := os.ReadFile(configPath)
	if err != nil {
		return true
	}

	re := regexp.MustCompile(`ePersonaState\\":(\d+)`)
	match := re.FindStringSubmatch(string(content))
	if len(match) > 1 {
		if match[1] == "7" || match[1] == "0" {
			return false
		}
	}
	return true
}

func initialModel(games []Game, quitAfterLaunch bool, lastSelectedID string) model {
	cursor := 0
	if lastSelectedID != "" {
		found := false
		for i, g := range games {
			if g.ID == lastSelectedID && !g.Hidden {
				cursor = i
				found = true
				break
			}
		}
		if !found {
			for i, g := range games {
				if !g.Hidden {
					cursor = i
					break
				}
			}
		}
	} else {
		for i, g := range games {
			if !g.Hidden {
				cursor = i
				break
			}
		}
	}
	return model{
		games:           games,
		cursor:          cursor,
		quitAfterLaunch: quitAfterLaunch,
		steamOnline:     getSteamOnlineStatus(),
	}
}

func (m model) Init() tea.Cmd {
	// Just return `nil`, which means "no I/O right now, please."
	return nil
}

func (m model) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
	switch msg := msg.(type) {

	case tea.WindowSizeMsg:
		m.windowHeight = msg.Height

	case tea.KeyPressMsg:
		if m.searching {
			switch msg.String() {
			case "enter":
				m.searching = false
				snapCursorToFirstMatch(&m)
				return m, nil
			case "esc":
				m.searching = false
				m.searchQuery = ""
				snapCursorToFirstMatch(&m)
				return m, nil
			case "backspace", "backspace2":
				if len(m.searchQuery) > 0 {
					m.searchQuery = m.searchQuery[:len(m.searchQuery)-1]
				}
				snapCursorToFirstMatch(&m)
				return m, nil
			case "space":
				m.searchQuery += " "
				snapCursorToFirstMatch(&m)
				return m, nil
			default:
				if len(msg.String()) == 1 {
					m.searchQuery += msg.String()
					snapCursorToFirstMatch(&m)
				}
				return m, nil
			}
		}

		// If there's a search filter active, clear it with escape
		if !m.searching && m.searchQuery != "" {
			switch msg.String() {
			case "esc":
				m.searchQuery = ""
				snapCursorToFirstMatch(&m)
				return m, nil
			}
		}

		// Cool, what was the actual key pressed?
		switch msg.String() {

		case "o", "O":
			m.steamOnline = !m.steamOnline
			var status string
			if m.steamOnline {
				status = "online"
			} else {
				status = "offline"
			}
			cmd := exec.Command("steam", "steam://friends/status/"+status)
			_ = cmd.Start()
			return m, nil

		case "/":
			m.searching = true
			m.searchQuery = ""
			return m, nil

		// These keys should exit the program.
		case "ctrl+c", "q":
			if len(m.games) > 0 && m.cursor >= 0 && m.cursor < len(m.games) {
				saveState(m.games[m.cursor].ID)
			}
			return m, tea.Quit

		// The "up" and "k" keys move the cursor up
		case "up", "k":
			// Calculate visual order to move cursor correctly
			var visualIndices []int
			var favs, others []int
			for i, game := range m.games {
				if game.Hidden == m.showHidden && (m.searchQuery == "" || strings.Contains(strings.ToLower(game.Name), strings.ToLower(m.searchQuery))) {
					if game.Favorite {
						favs = append(favs, i)
					} else {
						others = append(others, i)
					}
				}
			}
			sort.Slice(favs, func(i, j int) bool { return m.games[favs[i]].Name < m.games[favs[j]].Name })
			sort.Slice(others, func(i, j int) bool { return m.games[others[i]].Name < m.games[others[j]].Name })
			visualIndices = append(favs, others...)

			cursorIdx := -1
			for i, idx := range visualIndices {
				if idx == m.cursor {
					cursorIdx = i
					break
				}
			}

			if cursorIdx > 0 {
				m.cursor = visualIndices[cursorIdx-1]
			} else if len(visualIndices) > 0 {
				m.cursor = visualIndices[len(visualIndices)-1]
			}

		// The "down" and "j" keys move the cursor down
		case "down", "j":
			// Calculate visual order to move cursor correctly
			var visualIndices []int
			var favs, others []int
			for i, game := range m.games {
				if game.Hidden == m.showHidden && (m.searchQuery == "" || strings.Contains(strings.ToLower(game.Name), strings.ToLower(m.searchQuery))) {
					if game.Favorite {
						favs = append(favs, i)
					} else {
						others = append(others, i)
					}
				}
			}
			sort.Slice(favs, func(i, j int) bool { return m.games[favs[i]].Name < m.games[favs[j]].Name })
			sort.Slice(others, func(i, j int) bool { return m.games[others[i]].Name < m.games[others[j]].Name })
			visualIndices = append(favs, others...)

			cursorIdx := -1
			for i, idx := range visualIndices {
				if idx == m.cursor {
					cursorIdx = i
					break
				}
			}

			if cursorIdx != -1 && cursorIdx < len(visualIndices)-1 {
				m.cursor = visualIndices[cursorIdx+1]
			} else if len(visualIndices) > 0 {
				m.cursor = visualIndices[0]
			}

		// Toggle hidden status
		case "h":
			if len(m.games) > 0 {
				m.games[m.cursor].Hidden = !m.games[m.cursor].Hidden
				saveGames(m.games)

				// Move cursor to next visible game (matching current filter)
				found := false
				for i := m.cursor + 1; i < len(m.games); i++ {
					if m.games[i].Hidden == m.showHidden {
						m.cursor = i
						found = true
						break
					}
				}
				if !found {
					for i := m.cursor - 1; i >= 0; i-- {
						if m.games[i].Hidden == m.showHidden {
							m.cursor = i
							found = true
							break
						}
					}
				}
				if !found {
					m.cursor = 0
				}
			}

		case "f":
			if len(m.games) > 0 {
				m.games[m.cursor].Favorite = !m.games[m.cursor].Favorite
				saveGames(m.games)
			}

		// Toggle between showing shown and hidden games
		case "H":
			m.showHidden = !m.showHidden
			// Ensure cursor is on a game matching the new filter
			found := false
			for i := 0; i < len(m.games); i++ {
				if m.games[i].Hidden == m.showHidden {
					m.cursor = i
					found = true
					break
				}
			}
			if !found {
				m.cursor = 0
			}

		// The "enter" key toggles the selected state
		// for the item that the cursor is pointing at.
		case "enter":
			if len(m.games) == 0 {
				return m, nil
			}
			game := m.games[m.cursor]
			saveState(game.ID)
			cmd := exec.Command(
				"steam",
				"steam://rungameid/"+game.ID,
			)

			if err := cmd.Start(); err != nil {
				fmt.Println("Failed to launch:", err)
				return m, nil
			}

			if m.quitAfterLaunch {
				return m, tea.Quit
			}

		}
	}

	// Return the updated model to the Bubble Tea runtime for processing.
	// Note that we're not returning a command.
	if len(m.games) > 0 && m.cursor >= 0 && m.cursor < len(m.games) {
		saveState(m.games[m.cursor].ID)
	}
	return m, nil
}

func (m model) View() tea.View {
	// The header
	var statusStr string
	if m.steamOnline {
		statusStr = "\033[32mOnline\033[0m"
	} else {
		statusStr = "\033[31mOffline\033[0m"
	}
	s := fmt.Sprintf("Stui: A TUI Steam Game Launcher | Steam Status: %s\n\n", statusStr)

	if m.searching {
		s += fmt.Sprintf("Search: %s\n\n", m.searchQuery)
	} else if m.searchQuery != "" {
		s += fmt.Sprintf("Filtered: %s\n\n", m.searchQuery)
	} else if m.showHidden {
		s += "Showing Hidden Games\n\n"
	}

	// Filter and sort games: Favorites first, then alphabetically
	var favIndices []int
	var otherIndices []int
	for i, game := range m.games {
		if game.Hidden != m.showHidden {
			continue
		}
		if m.searchQuery != "" && !strings.Contains(strings.ToLower(game.Name), strings.ToLower(m.searchQuery)) {
			continue
		}
		if game.Favorite {
			favIndices = append(favIndices, i)
		} else {
			otherIndices = append(otherIndices, i)
		}
	}

	// Sort indices by game name to maintain consistency
	sort.Slice(favIndices, func(i, j int) bool {
		return m.games[favIndices[i]].Name < m.games[favIndices[j]].Name
	})
	sort.Slice(otherIndices, func(i, j int) bool {
		return m.games[otherIndices[i]].Name < m.games[otherIndices[j]].Name
	})

	// Build a visual list of elements (indices or gaps)
	type element struct {
		index int
		isGap bool
	}
	var visualList []element
	for _, idx := range favIndices {
		visualList = append(visualList, element{index: idx, isGap: false})
	}
	for _, idx := range otherIndices {
		visualList = append(visualList, element{index: idx, isGap: false})
	}

	// Find the visual position of the cursor
	cursorVisualIdx := -1
	for i, el := range visualList {
		if !el.isGap && el.index == m.cursor {
			cursorVisualIdx = i
			break
		}
	}

	// Determine the window of games to show
	availableHeight := m.windowHeight - 6
	if availableHeight < 1 {
		availableHeight = 1
	}

	start := 0
	if cursorVisualIdx != -1 {
		start = cursorVisualIdx - availableHeight/2
		if start < 0 {
			start = 0
		}
		if start+availableHeight > len(visualList) {
			start = len(visualList) - availableHeight
			if start < 0 {
				start = 0
			}
		}
	}

	end := start + availableHeight
	if end > len(visualList) {
		end = len(visualList)
	}

	// Iterate over the visual list in the window
	for i := start; i < end; i++ {
		el := visualList[i]
		if el.isGap {
			s += "\n"
			continue
		}

		game := m.games[el.index]
		cursor := " "
		if m.cursor == el.index {
			cursor = ">"
		}

		favMarker := " "
		if game.Favorite {
			favMarker = "*"
		}

		line := fmt.Sprintf("%s %s %s [%d hours]\n", cursor, favMarker, game.Name, game.Playtime/60)
		if game.Favorite {
			line = "\033[1m" + line + "\033[0m"
		}
		s += line
	}

	// The footer
	s += "\n"
	s += "q:quit  /:search  h:hide  H:toggle view  f:fav  O:toggle status\n"

	v := tea.NewView(s)
	v.AltScreen = true
	if m.searching {
		v.Cursor = tea.NewCursor(len("Search: ")+len(m.searchQuery), 2)
	}
	return v
}

func main() {
	quitAfterLaunch := flag.Bool("q", false, "quit after launching a game")
	flag.Parse()

	f, _ := os.Create("/tmp/stui-debug")
	fmt.Fprintf(f, "args: %#v\n", os.Args)
	fmt.Fprintf(f, "quitAfterLaunch: %v\n", *quitAfterLaunch)
	f.Close()

	//Check if files exist and creates them if needed
	home, _ := os.UserHomeDir()
	path := (home + "/.config/stui")
	file := (path + "/games.json")
	_, err := os.Stat(path)
	if os.IsNotExist(err) {
		os.Mkdir(path, os.ModePerm)
	}
	_, err = os.Stat(file)
	if os.IsNotExist(err) {
		f, _ := os.Create(file)
		f.WriteString("[]")
		f.Close()
	}

	//Reads files into memory
	read, _ := os.ReadFile(file)
	var games []Game
	_ = json.Unmarshal(read, &games)

	// Scan games on boot to ensure the list is up to date
	scannedGames, err := ScanSteamGames()
	if err == nil {
		// Merge scanned games with existing games (preserving hidden/favorite status and removing uninstalled games)
		existingGames := make(map[string]Game)
		for _, g := range games {
			existingGames[g.ID] = g
		}
		var updatedGames []Game
		for _, g := range scannedGames {
			if existing, ok := existingGames[g.ID]; ok {
				g.Hidden = existing.Hidden
				g.Favorite = existing.Favorite
			}
			updatedGames = append(updatedGames, g)
		}
		games = updatedGames
		saveGames(games)
	}

	games = refreshPlaytimes(games)

	if len(games) == 0 {
		games = []Game{} // Ensure it's not nil
	} else {
		sort.Slice(games, func(i, j int) bool {
			return games[i].Name < games[j].Name
		})
	}

	state := loadState()
	p := tea.NewProgram(initialModel(games, *quitAfterLaunch, state.LastSelectedID))
	if _, err := p.Run(); err != nil {
		fmt.Printf("Alas, there's been an error: %v", err)
		os.Exit(1)
	}
}
