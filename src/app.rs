use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::models::Game;
use crate::steam;

#[derive(Debug, Clone)]
pub struct App {
    pub games: Vec<Game>,
    pub cursor: usize,
    pub searching: bool,
    pub search_query: String,
    pub show_hidden: bool,
    pub quit_after_launch: bool,
    pub steam_online: bool,
    pub should_quit: bool,
}

impl App {
    pub fn new(games: Vec<Game>, quit_after_launch: bool, last_selected_id: Option<&str>) -> Self {
        let steam_online = steam::get_steam_online_status();
        let mut cursor = 0;

        if let Some(id) = last_selected_id {
            if let Some(pos) = games.iter().position(|g| g.id == id && !g.hidden) {
                cursor = pos;
            } else if let Some(pos) = games.iter().position(|g| !g.hidden) {
                cursor = pos;
            }
        } else if let Some(pos) = games.iter().position(|g| !g.hidden) {
            cursor = pos;
        }

        Self {
            games,
            cursor,
            searching: false,
            search_query: String::new(),
            show_hidden: false,
            quit_after_launch,
            steam_online,
            should_quit: false,
        }
    }

    pub fn filtered_indices(&self) -> Vec<usize> {
        let mut favs = Vec::new();
        let mut others = Vec::new();
        let query_lower = self.search_query.to_lowercase();

        for (i, game) in self.games.iter().enumerate() {
            if game.hidden != self.show_hidden {
                continue;
            }

            if !self.search_query.is_empty() && !game.name.to_lowercase().contains(&query_lower) {
                continue;
            }

            if game.favorite {
                favs.push(i);
            } else {
                others.push(i);
            }
        }

        favs.sort_by(|&a, &b| self.games[a].name.cmp(&self.games[b].name));
        others.sort_by(|&a, &b| self.games[a].name.cmp(&self.games[b].name));

        favs.extend(others);
        favs
    }

    pub fn snap_cursor_to_first_match(&mut self) {
        let visual = self.filtered_indices();
        if let Some(&first) = visual.first() {
            self.cursor = first;
        } else {
            self.cursor = 0;
        }
        self.save_current_state();
    }

    pub fn next_item(&mut self) {
        let visual = self.filtered_indices();
        if visual.is_empty() {
            return;
        }

        let cursor_idx = visual.iter().position(|&idx| idx == self.cursor);
        match cursor_idx {
            Some(idx) if idx + 1 < visual.len() => {
                self.cursor = visual[idx + 1];
            }
            _ => {
                self.cursor = visual[0];
            }
        }
        self.save_current_state();
    }

    pub fn prev_item(&mut self) {
        let visual = self.filtered_indices();
        if visual.is_empty() {
            return;
        }

        let cursor_idx = visual.iter().position(|&idx| idx == self.cursor);
        match cursor_idx {
            Some(idx) if idx > 0 => {
                self.cursor = visual[idx - 1];
            }
            _ => {
                self.cursor = visual[visual.len() - 1];
            }
        }
        self.save_current_state();
    }

    pub fn toggle_favorite(&mut self) {
        if self.cursor < self.games.len() {
            self.games[self.cursor].favorite = !self.games[self.cursor].favorite;
            let _ = steam::save_games(&self.games);
        }
    }

    pub fn toggle_hidden(&mut self) {
        if self.cursor < self.games.len() {
            self.games[self.cursor].hidden = !self.games[self.cursor].hidden;
            let _ = steam::save_games(&self.games);

            let mut found = false;
            for i in (self.cursor + 1)..self.games.len() {
                if self.games[i].hidden == self.show_hidden {
                    self.cursor = i;
                    found = true;
                    break;
                }
            }
            if !found {
                for i in (0..self.cursor).rev() {
                    if self.games[i].hidden == self.show_hidden {
                        self.cursor = i;
                        found = true;
                        break;
                    }
                }
            }
            if !found {
                self.cursor = 0;
            }
            self.save_current_state();
        }
    }

    pub fn toggle_view_mode(&mut self) {
        self.show_hidden = !self.show_hidden;
        let mut found = false;
        for i in 0..self.games.len() {
            if self.games[i].hidden == self.show_hidden {
                self.cursor = i;
                found = true;
                break;
            }
        }
        if !found {
            self.cursor = 0;
        }
        self.save_current_state();
    }

    pub fn toggle_steam_status(&mut self) {
        self.steam_online = !self.steam_online;
        let _ = steam::set_steam_status(self.steam_online);
    }

    pub fn launch_selected(&mut self) {
        if self.games.is_empty() || self.cursor >= self.games.len() {
            return;
        }

        let game_id = self.games[self.cursor].id.clone();
        self.save_current_state();
        let _ = steam::launch_game(&game_id);

        if self.quit_after_launch {
            self.should_quit = true;
        }
    }

    pub fn save_current_state(&self) {
        if !self.games.is_empty() && self.cursor < self.games.len() {
            steam::save_state(&self.games[self.cursor].id);
        }
    }

    pub fn handle_key_event(&mut self, key: KeyEvent) {
        if self.searching {
            match key.code {
                KeyCode::Enter => {
                    self.searching = false;
                    self.snap_cursor_to_first_match();
                }
                KeyCode::Esc => {
                    self.searching = false;
                    self.search_query.clear();
                    self.snap_cursor_to_first_match();
                }
                KeyCode::Backspace => {
                    self.search_query.pop();
                    self.snap_cursor_to_first_match();
                }
                KeyCode::Char(c) => {
                    self.search_query.push(c);
                    self.snap_cursor_to_first_match();
                }
                _ => {}
            }
            return;
        }

        if !self.search_query.is_empty() && key.code == KeyCode::Esc {
            self.search_query.clear();
            self.snap_cursor_to_first_match();
            return;
        }

        match key.code {
            KeyCode::Char('q') => {
                self.save_current_state();
                self.should_quit = true;
            }
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.save_current_state();
                self.should_quit = true;
            }
            KeyCode::Char('o') | KeyCode::Char('O') => {
                self.toggle_steam_status();
            }
            KeyCode::Char('/') => {
                self.searching = true;
                self.search_query.clear();
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.prev_item();
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.next_item();
            }
            KeyCode::Char('h') => {
                self.toggle_hidden();
            }
            KeyCode::Char('f') => {
                self.toggle_favorite();
            }
            KeyCode::Char('H') => {
                self.toggle_view_mode();
            }
            KeyCode::Enter => {
                self.launch_selected();
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_games() -> Vec<Game> {
        vec![
            Game {
                id: "1".into(),
                name: "Alpha Game".into(),
                hidden: false,
                playtime: 120,
                favorite: false,
            },
            Game {
                id: "2".into(),
                name: "Beta Game".into(),
                hidden: false,
                playtime: 60,
                favorite: true,
            },
            Game {
                id: "3".into(),
                name: "Gamma Game".into(),
                hidden: true,
                playtime: 0,
                favorite: false,
            },
            Game {
                id: "4".into(),
                name: "Delta Game".into(),
                hidden: false,
                playtime: 300,
                favorite: false,
            },
        ]
    }

    #[test]
    fn test_app_initialization_with_last_selected() {
        let games = sample_games();
        let app = App::new(games, false, Some("4"));
        assert_eq!(app.cursor, 3);
        assert_eq!(app.games[app.cursor].id, "4");
    }

    #[test]
    fn test_filtered_indices_ordering() {
        let games = sample_games();
        let app = App::new(games, false, None);
        let visual = app.filtered_indices();

        // Favorite ("Beta Game", index 1) should be first, followed by Alpha (0) and Delta (3)
        assert_eq!(visual, vec![1, 0, 3]);
    }

    #[test]
    fn test_navigation_wrap_around() {
        let games = sample_games();
        let mut app = App::new(games, false, None);
        // visual order: [1 (Beta), 0 (Alpha), 3 (Delta)]
        app.cursor = 1;

        app.next_item();
        assert_eq!(app.cursor, 0);

        app.next_item();
        assert_eq!(app.cursor, 3);

        app.next_item();
        assert_eq!(app.cursor, 1);

        app.prev_item();
        assert_eq!(app.cursor, 3);
    }

    #[test]
    fn test_search_filtering() {
        let games = sample_games();
        let mut app = App::new(games, false, None);

        app.handle_key_event(KeyEvent::new(KeyCode::Char('/'), KeyModifiers::empty()));
        assert!(app.searching);

        app.handle_key_event(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::empty()));
        app.handle_key_event(KeyEvent::new(KeyCode::Char('e'), KeyModifiers::empty()));
        assert_eq!(app.search_query, "de");

        let visual = app.filtered_indices();
        assert_eq!(visual, vec![3]); // "Delta Game"
        assert_eq!(app.cursor, 3);

        // Press Enter to confirm search filter
        app.handle_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()));
        assert!(!app.searching);
        assert_eq!(app.filtered_indices(), vec![3]);

        // Press Esc to clear filter
        app.handle_key_event(KeyEvent::new(KeyCode::Esc, KeyModifiers::empty()));
        assert_eq!(app.search_query, "");
        assert_eq!(app.filtered_indices(), vec![1, 0, 3]);
    }

    #[test]
    fn test_toggle_view_mode() {
        let games = sample_games();
        let mut app = App::new(games, false, None);
        assert!(!app.show_hidden);

        app.toggle_view_mode();
        assert!(app.show_hidden);
        let visual = app.filtered_indices();
        assert_eq!(visual, vec![2]); // Gamma Game (hidden)
        assert_eq!(app.cursor, 2);
    }
}
